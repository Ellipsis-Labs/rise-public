import {
  decodeGlobalConfiguration,
  decodePerpAssetMap,
  fetchWithdrawQueueHeader,
  fetchOrderbookHeader,
  isExchangeEffectivelyActive,
} from "@/accounts";
import type {
  GlobalConfiguration,
  PerpAssetMetadata,
  WithdrawQueueHeader,
} from "@/accounts";
import type { AccountFetcherClient } from "@/accounts/fetcherFactory";
import type {
  ExchangeMarketSnapshot,
  ExchangeSnapshotView,
  ExchangeStateSnapshot,
  ExchangeWsCommodityMetadata,
  ExchangeWsMarkPriceParameters,
  ExchangeWsMarketPriceBand,
  ExchangeWsValidationRule,
} from "@/api/exchange/types";
import {
  getActiveTraderBufferAddresses,
  getGlobalTraderIndexAddresses,
} from "@/core/helpers";
import type { PhoenixPdaClient } from "@/pdaClient";
import type { PhoenixProgramAddress } from "@/primitives";
import { PhoenixRpcAccountFetcherClient } from "@/rpc";
import {
  getSysvarLastRestartSlotDecoder,
  SYSVAR_LAST_RESTART_SLOT_ADDRESS,
} from "@solana/sysvars";
import type { Address } from "@solana/kit";
import type { PhoenixExchangeCacheStore } from "./types";
import type {
  ExchangeCacheEvent,
  ExchangeCacheExchangeChangeKind,
  ExchangeCacheHealth,
  ExchangeCacheHealthReason,
  ExchangeCacheMarketChangeKind,
  ExchangeCacheSnapshotSource,
  ExchangeMetadataSource,
  PhoenixExchangeMetadataConfig,
} from "./types";

const lastRestartSlotDecoder = getSysvarLastRestartSlotDecoder();

export const DEFAULT_RPC_POLL_INTERVAL_MS = 5_000;
export const DEFAULT_RPC_TTL_MS = 30_000;
export const DEFAULT_API_RESYNC_BACKOFF_MS = 1_000;

export const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

const toValidationRule = (value: number): ExchangeWsValidationRule => {
  switch (value) {
    case 1:
      return "require";
    case 2:
      return "forbid";
    default:
      return "ignore";
  }
};

const decodeRiskActionPriceValidityRules = (
  values: number[]
): ExchangeWsMarkPriceParameters["riskActionPriceValidityRules"] => {
  const rules: ExchangeWsMarkPriceParameters["riskActionPriceValidityRules"] =
    [];

  for (let outer = 0; outer < 8; outer++) {
    const outerRules: ExchangeWsValidationRule[][] = [];
    for (let middle = 0; middle < 4; middle++) {
      const innerRules: ExchangeWsValidationRule[] = [];
      for (let inner = 0; inner < 8; inner++) {
        const index = outer * 32 + middle * 8 + inner;
        innerRules.push(toValidationRule(values[index] ?? 0));
      }
      outerRules.push(innerRules);
    }
    rules.push(outerRules);
  }

  return rules;
};

const toExchangeStatusFeatures = (bits: number): string[] => {
  const features: string[] = [];
  if ((bits & 0b1000_0000) !== 0) features.push("initialized");
  if ((bits & 0b0000_0001) !== 0) features.push("active");
  if ((bits & 0b0000_0010) !== 0) features.push("gated");
  return features;
};

const toMarketStatus = (status: number): string => {
  switch (status) {
    case 0:
      return "uninitialized";
    case 1:
      return "active";
    case 2:
      return "postOnly";
    case 3:
      return "paused";
    case 4:
      return "closed";
    case 5:
      return "tombstoned";
    default:
      return "unknown";
  }
};

const toExchangeStateSnapshot = (
  globalConfiguration: GlobalConfiguration,
  withdrawQueueHeader: WithdrawQueueHeader | null,
  globalTraderIndex: string[],
  activeTraderBuffer: string[],
  programId: PhoenixProgramAddress,
  lastRestartSlot: bigint | null
): ExchangeStateSnapshot => {
  const exchangeStatusBits = globalConfiguration.exchangeStatus;
  const active = isExchangeEffectivelyActive({
    globalConfiguration,
    lastRestartSlot,
  });
  return {
    programId,
    globalConfig: globalConfiguration.accountKey,
    currentAuthorities: {
      rootAuthority: globalConfiguration.currentAuthorities.rootAuthority,
      riskAuthority: globalConfiguration.currentAuthorities.riskAuthority,
      marketAuthority: globalConfiguration.currentAuthorities.marketAuthority,
      oracleAuthority: globalConfiguration.currentAuthorities.oracleAuthority,
      adlAuthority: globalConfiguration.currentAuthorities.adlAuthority,
      cancelAuthority: globalConfiguration.currentAuthorities.cancelAuthority,
      backstopAuthority:
        globalConfiguration.currentAuthorities.backstopAuthority,
    },
    canonicalMint: globalConfiguration.canonicalTokenMintKey,
    usdcMint: globalConfiguration.canonicalTokenMintKey,
    globalVault: globalConfiguration.globalVaultKey,
    perpAssetMap: globalConfiguration.perpAssetMapKey,
    globalTraderIndex,
    activeTraderBuffer,
    withdrawQueue: globalConfiguration.withdrawQueueKey,
    exchangeStatusBits,
    exchangeStatusFeatures: toExchangeStatusFeatures(exchangeStatusBits),
    active,
    gated:
      (exchangeStatusBits & 0b1000_0000) !== 0 &&
      (exchangeStatusBits & 0b0000_0010) !== 0,
    withdrawalsAvailable:
      active &&
      (withdrawQueueHeader === null ||
        withdrawQueueHeader.withdrawThrottle.maxBudget !== 0n),
  };
};

const DEFAULT_QUOTE_DECIMALS = 6;
const MAX_PRICE_DECIMALS = 18;

const pow10 = (exponent: number): bigint => 10n ** BigInt(exponent);

const divRoundNearest = (numerator: bigint, denominator: bigint): bigint =>
  (numerator + denominator / 2n) / denominator;

const formatScaledDecimal = (value: bigint, decimals: number): string => {
  const negative = value < 0n;
  const absolute = negative ? -value : value;
  if (decimals === 0) {
    return `${negative ? "-" : ""}${absolute.toString()}`;
  }

  const scale = pow10(decimals);
  const whole = absolute / scale;
  const fraction = absolute % scale;
  return `${negative ? "-" : ""}${whole.toString()}.${fraction
    .toString()
    .padStart(decimals, "0")}`;
};

const toPriceDecimal = (params: {
  ticks: bigint;
  tickSize: bigint;
  baseLotsDecimals: number;
  quoteDecimals: number;
}): {
  value: number;
  decimals: number;
  ui: string;
} => {
  const decimals = Math.max(
    0,
    Math.min(MAX_PRICE_DECIMALS, params.quoteDecimals - params.baseLotsDecimals)
  );
  const exponent = params.baseLotsDecimals + decimals - params.quoteDecimals;
  const tickValue = params.ticks * params.tickSize;
  const scaled =
    exponent >= 0
      ? tickValue * pow10(exponent)
      : divRoundNearest(tickValue, pow10(-exponent));

  return {
    value: Number(scaled),
    decimals,
    ui: formatScaledDecimal(scaled, decimals),
  };
};

const toCommodityStatus = (metadata: PerpAssetMetadata): string => {
  if (metadata.assetFlags.isCommoditiesAfterHours) {
    return "afterHours";
  }
  if (metadata.assetFlags.isCommoditiesReopen) {
    return "reopen";
  }
  return "active";
};

const toCommodityBand = (
  params:
    | {
        lower: bigint;
        upper: bigint;
        tickSize: bigint;
        baseLotsDecimals: number;
        quoteDecimals: number;
      }
    | undefined
): ExchangeWsMarketPriceBand | null => {
  if (!params) {
    return null;
  }

  return {
    lower: toPriceDecimal({
      ticks: params.lower,
      tickSize: params.tickSize,
      baseLotsDecimals: params.baseLotsDecimals,
      quoteDecimals: params.quoteDecimals,
    }),
    upper: toPriceDecimal({
      ticks: params.upper,
      tickSize: params.tickSize,
      baseLotsDecimals: params.baseLotsDecimals,
      quoteDecimals: params.quoteDecimals,
    }),
  };
};

export const toCommodityMetadata = (params: {
  metadata: PerpAssetMetadata;
  tickSize: bigint;
  baseLotsDecimals: number;
  quoteDecimals?: number;
}): ExchangeWsCommodityMetadata | null => {
  const { metadata, tickSize, baseLotsDecimals } = params;
  if (!metadata.assetFlags.isCommodity) {
    return null;
  }

  const quoteDecimals = params.quoteDecimals ?? DEFAULT_QUOTE_DECIMALS;
  const lastKnownIndexPrice = metadata.lastKnownIndexPrice;
  const afterHoursBand =
    metadata.assetFlags.isCommoditiesAfterHours && lastKnownIndexPrice !== null
      ? {
          lower:
            lastKnownIndexPrice > metadata.commoditiesAfterHoursRadius
              ? lastKnownIndexPrice - metadata.commoditiesAfterHoursRadius
              : 0n,
          upper: lastKnownIndexPrice + metadata.commoditiesAfterHoursRadius,
          tickSize,
          baseLotsDecimals,
          quoteDecimals,
        }
      : undefined;

  return {
    isCommodity: true,
    isReopen: metadata.assetFlags.isCommoditiesReopen,
    isAfterHours: metadata.assetFlags.isCommoditiesAfterHours,
    status: toCommodityStatus(metadata),
    afterHoursRadius: toPriceDecimal({
      ticks: metadata.commoditiesAfterHoursRadius,
      tickSize,
      baseLotsDecimals,
      quoteDecimals,
    }),
    lastKnownIndexPrice:
      lastKnownIndexPrice === null
        ? null
        : toPriceDecimal({
            ticks: lastKnownIndexPrice,
            tickSize,
            baseLotsDecimals,
            quoteDecimals,
          }),
    markPriceBand: toCommodityBand(afterHoursBand),
    executionPriceBand: toCommodityBand(afterHoursBand),
    lastIndexExpiryTimestamp:
      metadata.lastIndexExpiryTimestamp === 0n
        ? null
        : Number(metadata.lastIndexExpiryTimestamp),
  };
};

const toMarkPriceParameters = (
  metadata: PerpAssetMetadata
): ExchangeWsMarkPriceParameters => ({
  emaPeriodSlots:
    metadata.oraclePrice.markPrice.spotPriceComponent.emaPeriodSlots,
  emaDiffRadius:
    metadata.oraclePrice.markPrice.spotPriceComponent.emaDiffRadius,
  bookPriceRadius:
    metadata.oraclePrice.markPrice.bookPriceComponent.bookPriceRadius,
  commoditiesAfterHoursRadius: metadata.commoditiesAfterHoursRadius,
  commoditiesAfterHoursRadiusBps: BigInt(
    metadata.commoditiesAfterHoursRadiusBps
  ),
  adjustedExchangeSpotPriceWeight:
    metadata.oraclePrice.markPrice.spotPriceComponent.weight,
  bookPriceWeight: metadata.oraclePrice.markPrice.bookPriceComponent.weight,
  exchangePerpPriceWeight:
    metadata.oraclePrice.markPrice.perpPriceComponent.weight,
  spotPriceStaleThreshold:
    metadata.oraclePrice.markPrice.spotPriceComponent.staleThreshold,
  bookPriceStaleThreshold:
    metadata.oraclePrice.markPrice.bookPriceComponent.staleThreshold,
  bookHardStaleMultiplier: 0,
  perpPriceStaleThreshold:
    metadata.oraclePrice.markPrice.perpPriceComponent.staleThreshold,
  riskActionPriceValidityRules: decodeRiskActionPriceValidityRules(
    metadata.oraclePrice.markPrice.riskActionPriceValidityRules
  ),
  oracleDivergenceRadius:
    metadata.oraclePrice.markPrice.oracleParameters.oracleDivergenceRadius,
  oracleHardStaleMultiplier: 0,
  minOracleResponses:
    metadata.oraclePrice.markPrice.oracleParameters.minOracleResponses,
});

const toMarketSnapshot = async (
  metadata: PerpAssetMetadata,
  symbol: string,
  client: AccountFetcherClient,
  pdaClient: PhoenixPdaClient,
  quoteDecimals: number
): Promise<ExchangeMarketSnapshot> => {
  const header = await fetchOrderbookHeader({
    client,
    address: metadata.staticMarketParams.marketAccount,
  });
  const splinePubkey = await pdaClient.getSplineCollectionAddress({
    marketAccountAddress: metadata.staticMarketParams.marketAccount,
  });

  return {
    symbol,
    assetId: metadata.staticMarketParams.assetId,
    marketStatus: toMarketStatus(header.marketStatus),
    marketPubkey: metadata.staticMarketParams.marketAccount,
    splinePubkey,
    tickSize: Number(metadata.staticMarketParams.tickSize),
    baseLotsDecimals: metadata.staticMarketParams.baseLotDecimals,
    takerFee: header.defaultTakerFeeMicro / 1_000_000,
    makerFee: header.defaultMakerFeeMicro / 1_000_000,
    leverageTiers: metadata.riskParams.leverageTiers.map((tier) => ({
      maxLeverage: Number(tier.maxLeverage),
      maxSizeBaseLots: tier.upperBoundSize,
      limitOrderRiskFactor: Number(tier.limitOrderRiskFactor),
    })),
    riskFactors: {
      maintenance: metadata.riskParams.riskFactors[0] ?? 0,
      maintenanceBps: metadata.riskParams.riskFactors[0] ?? 0,
      backstop: metadata.riskParams.riskFactors[1] ?? 0,
      backstopBps: metadata.riskParams.riskFactors[1] ?? 0,
      highRisk: metadata.riskParams.riskFactors[2] ?? 0,
      highRiskBps: metadata.riskParams.riskFactors[2] ?? 0,
      upnl: Number(metadata.riskParams.upnlRiskFactor),
      upnlBps: Number(metadata.riskParams.upnlRiskFactor),
      upnlForWithdrawals: Number(
        metadata.riskParams.upnlRiskFactorForWithdrawals
      ),
      upnlForWithdrawalsBps: Number(
        metadata.riskParams.upnlRiskFactorForWithdrawals
      ),
      cancelOrder: metadata.riskParams.cancelOrderRiskFactor,
      cancelOrderBps: metadata.riskParams.cancelOrderRiskFactor,
    },
    fundingConfig: {
      fundingIntervalSeconds: Number(
        metadata.fundingAccumulator.fundingIntervalSeconds
      ),
      fundingPeriodSeconds: Number(
        metadata.fundingAccumulator.fundingPeriodSeconds
      ),
      maxFundingRatePerInterval: Number(
        metadata.fundingAccumulator.maxFundingRate
      ),
    },
    openInterestCapBaseLots: metadata.openInterestParams.openInterestCap,
    maxLiquidationSizeBaseLots: metadata.riskParams.maxLiquidationSize,
    isolatedOnly: metadata.riskParams.isolatedOnly !== 0,
    markPriceParameters: toMarkPriceParameters(metadata),
    commodityMetadata: toCommodityMetadata({
      metadata,
      tickSize: metadata.staticMarketParams.tickSize,
      baseLotsDecimals: metadata.staticMarketParams.baseLotDecimals,
      quoteDecimals,
    }),
  };
};

const areExchangeKeysEqual = (
  left: ExchangeStateSnapshot,
  right: ExchangeStateSnapshot
): boolean =>
  left.globalConfig === right.globalConfig &&
  left.canonicalMint === right.canonicalMint &&
  left.globalVault === right.globalVault &&
  left.perpAssetMap === right.perpAssetMap &&
  left.withdrawQueue === right.withdrawQueue &&
  left.programId === right.programId &&
  stringifyComparable(left.currentAuthorities) ===
    stringifyComparable(right.currentAuthorities) &&
  stringifyComparable(left.globalTraderIndex) ===
    stringifyComparable(right.globalTraderIndex) &&
  stringifyComparable(left.activeTraderBuffer) ===
    stringifyComparable(right.activeTraderBuffer);

const toComparableValue = (value: unknown): unknown => {
  if (typeof value === "bigint") {
    return value.toString();
  }
  if (Array.isArray(value)) {
    return value.map((item) => toComparableValue(item));
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(
        ([key, innerValue]) => [key, toComparableValue(innerValue)]
      )
    );
  }
  return value;
};

const stringifyComparable = (value: unknown): string =>
  JSON.stringify(toComparableValue(value)) ?? "undefined";

const toMarketComparable = (market: ExchangeMarketSnapshot): string =>
  stringifyComparable(market);

const determineMarketChangeKind = (
  previous: ExchangeMarketSnapshot,
  next: ExchangeMarketSnapshot
): ExchangeCacheMarketChangeKind | null => {
  if (previous.marketStatus !== next.marketStatus) return "status";
  if (
    stringifyComparable(previous.leverageTiers) !==
    stringifyComparable(next.leverageTiers)
  ) {
    return "leverageTiers";
  }
  if (previous.riskFactors.cancelOrder !== next.riskFactors.cancelOrder) {
    return "cancelRiskFactor";
  }
  if (previous.isolatedOnly !== next.isolatedOnly) return "isolatedOnly";
  if (previous.openInterestCapBaseLots !== next.openInterestCapBaseLots) {
    return "openInterestCap";
  }
  if (previous.riskFactors.upnl !== next.riskFactors.upnl) {
    return "upnlRiskFactor";
  }
  if (
    previous.riskFactors.upnlForWithdrawals !==
    next.riskFactors.upnlForWithdrawals
  ) {
    return "upnlRiskFactorForWithdrawals";
  }
  if (
    stringifyComparable(previous.fundingConfig) !==
    stringifyComparable(next.fundingConfig)
  ) {
    return "fundingParameters";
  }
  if (
    previous.takerFee !== next.takerFee ||
    previous.makerFee !== next.makerFee
  ) {
    return "marketFees";
  }
  if (
    stringifyComparable(previous.markPriceParameters) !==
    stringifyComparable(next.markPriceParameters)
  ) {
    return "markPriceParameters";
  }
  if (
    stringifyComparable(previous.commodityMetadata) !==
    stringifyComparable(next.commodityMetadata)
  ) {
    return "commodityMetadata";
  }
  if (
    stringifyComparable(previous.metadata) !==
    stringifyComparable(next.metadata)
  ) {
    return "metadata";
  }
  if (toMarketComparable(previous) !== toMarketComparable(next)) {
    return "status";
  }
  return null;
};

const loadGlobalConfigurationAndPerpAssetMap = async (params: {
  fetcher: PhoenixRpcAccountFetcherClient;
  globalConfigurationAddress: Address;
}): Promise<{
  slot: bigint;
  globalConfiguration: GlobalConfiguration;
  perpAssetMap: ReturnType<typeof decodePerpAssetMap>;
  lastRestartSlot: bigint;
}> => {
  const { fetcher, globalConfigurationAddress } = params;

  let perpAssetMapAddress = decodeGlobalConfiguration(
    (await fetcher.fetchAccount(globalConfigurationAddress)).data
  ).perpAssetMapKey;

  for (let attempt = 0; attempt < 2; attempt += 1) {
    const { slot, accounts } = await fetcher.fetchAccounts([
      globalConfigurationAddress,
      perpAssetMapAddress,
      SYSVAR_LAST_RESTART_SLOT_ADDRESS,
    ]);
    const globalConfiguration = decodeGlobalConfiguration(accounts[0].data);
    if (globalConfiguration.perpAssetMapKey !== perpAssetMapAddress) {
      perpAssetMapAddress = globalConfiguration.perpAssetMapKey;
      continue;
    }

    return {
      slot,
      globalConfiguration,
      perpAssetMap: decodePerpAssetMap(accounts[1].data),
      lastRestartSlot: lastRestartSlotDecoder.decode(accounts[2].data)
        .lastRestartSlot,
    };
  }

  throw new Error(
    "RPC GlobalConfiguration changed while loading PerpAssetMap; retry bootstrap"
  );
};

export const loadRpcSnapshot = async (
  rpcUrl: string,
  pdaClient: PhoenixPdaClient
): Promise<ExchangeSnapshotView> => {
  const fetcher = new PhoenixRpcAccountFetcherClient(rpcUrl);
  const globalConfigurationAddress =
    await pdaClient.getGlobalConfigurationAddress();
  const phoenixProgramAddress = pdaClient.getProgramAddress();
  const helperClient = {
    fetchAccount: fetcher.fetchAccount.bind(fetcher),
    addresses: {
      phoenixProgramAddress,
      globalConfigurationAddress,
    },
  };

  const { slot, globalConfiguration, perpAssetMap, lastRestartSlot } =
    await loadGlobalConfigurationAndPerpAssetMap({
      fetcher,
      globalConfigurationAddress,
    });
  const helperClientWithCache = {
    ...helperClient,
    _accountCache: new Map<string, unknown>([
      [`account:${globalConfigurationAddress}`, globalConfiguration],
      [`account:${globalConfiguration.perpAssetMapKey}`, perpAssetMap],
    ]),
  };

  const [withdrawQueueHeader, globalTraderIndex, activeTraderBuffer, markets] =
    await Promise.all([
      fetchWithdrawQueueHeader({
        client: helperClientWithCache,
        address: globalConfiguration.withdrawQueueKey,
      }).catch(() => null),
      getGlobalTraderIndexAddresses(helperClientWithCache),
      getActiveTraderBufferAddresses(helperClientWithCache),
      Promise.all(
        perpAssetMap.metadata.entries.map(({ key, value }) =>
          toMarketSnapshot(
            value,
            key,
            fetcher,
            pdaClient,
            globalConfiguration.quoteDecimals
          )
        )
      ),
    ]);

  return {
    version: 1,
    slot,
    slotIndex: 0,
    exchange: toExchangeStateSnapshot(
      globalConfiguration,
      withdrawQueueHeader,
      globalTraderIndex,
      activeTraderBuffer,
      phoenixProgramAddress,
      lastRestartSlot
    ),
    markets,
  };
};

const determineExchangeChange = (
  previous: ExchangeStateSnapshot,
  next: ExchangeStateSnapshot
): ExchangeCacheExchangeChangeKind | null => {
  if (
    previous.exchangeStatusBits !== next.exchangeStatusBits ||
    previous.active !== next.active ||
    previous.gated !== next.gated ||
    previous.withdrawalsAvailable !== next.withdrawalsAvailable
  ) {
    return "status";
  }
  if (!areExchangeKeysEqual(previous, next)) {
    return "keys";
  }
  return null;
};

export const emitExchangeDiffEvents = (params: {
  current: Readonly<ExchangeSnapshotView> | null;
  next: ExchangeSnapshotView;
  emit: (event: ExchangeCacheEvent) => void;
}): void => {
  const { current, next, emit } = params;
  if (!current) {
    return;
  }

  const slot = next.slot;
  const slotIndex = next.slotIndex;

  const exchangeChange = determineExchangeChange(
    current.exchange,
    next.exchange
  );
  if (exchangeChange) {
    emit({
      type: "exchangeUpdated",
      change: exchangeChange,
      slot,
      slotIndex,
    });
  }

  const currentMarkets = new Map(
    current.markets.map((market) => [market.symbol.toUpperCase(), market])
  );
  const nextMarkets = new Map(
    next.markets.map((market) => [market.symbol.toUpperCase(), market])
  );

  for (const [symbolKey, market] of nextMarkets) {
    const previous = currentMarkets.get(symbolKey);
    if (!previous) {
      emit({
        type: "marketAdded",
        symbol: market.symbol,
        slot,
        slotIndex,
      });
      continue;
    }

    const change = determineMarketChangeKind(previous, market);
    if (change) {
      emit({
        type: "marketUpdated",
        symbol: market.symbol,
        change,
        slot,
        slotIndex,
      });
    }
  }

  for (const [symbolKey, market] of currentMarkets) {
    if (nextMarkets.has(symbolKey)) continue;
    emit({
      type: "marketRemoved",
      symbol: market.symbol,
      assetId: market.assetId,
      slot,
      slotIndex,
    });
  }
};

export interface ExchangeMetadataSnapshotInstallParams {
  snapshot: ExchangeSnapshotView;
  source: ExchangeCacheSnapshotSource;
  active: Exclude<ExchangeMetadataSource, "none">;
}

export interface ExchangeMetadataSourceContext {
  readonly cacheStore: PhoenixExchangeCacheStore;
  readonly config: PhoenixExchangeMetadataConfig;
  readonly pdaClient: PhoenixPdaClient;
  isClosed(): boolean;
  canUseApi(): boolean;
  canUseRpc(): boolean;
  activeSource(): ExchangeMetadataSource;
  installSnapshot(params: ExchangeMetadataSnapshotInstallParams): void;
  markSourceSynced(): void;
  transitionHealth(
    current: ExchangeCacheHealth,
    reason: ExchangeCacheHealthReason
  ): void;
  onBackgroundError(error: unknown): void;
}
