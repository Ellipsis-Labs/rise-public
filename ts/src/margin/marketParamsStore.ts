import type { MarketResponse } from "@/api/markets/types";
import type { CollateralAssetsResponse } from "@/api/collateral/types";
import type { MarketsResponse, MarketSummary } from "@/types";
import { createMarginCalculator, type MarginCalculator } from "./compute";
import type { MarketParamsBySymbol } from "./compute";
import type { MarketParams, SpotCollateralParams } from "./types";

export type MissingPriceBehavior = "error" | "skip" | "zero";

export interface MarketParamsBuildOptions {
  missingPriceBehavior?: MissingPriceBehavior;
  indexPricesBySymbol?: Record<string, number | string | null>;
}

export interface MarginMarketsClient {
  getMarkets(): Promise<MarketsResponse>;
  getMarket(symbol: string): Promise<MarketResponse>;
}

export interface MarginClientWithMarketsProperty {
  markets: MarginMarketsClient;
  collateral?: MarginCollateralClient | (() => MarginCollateralClient);
}

export interface MarginClientWithMarketsMethod {
  markets(): MarginMarketsClient;
  collateral?: MarginCollateralClient | (() => MarginCollateralClient);
}

export interface MarginCollateralClient {
  getAssets(): Promise<CollateralAssetsResponse>;
}

export type MarginClient =
  | MarginClientWithMarketsProperty
  | MarginClientWithMarketsMethod;

export interface MarketPriceSource<
  ClientType extends MarginClient = MarginClient,
> {
  getMarkPrices(
    symbols: string[],
    client: ClientType
  ): Promise<Record<string, number | string | null>>;
  getIndexPrices?(
    symbols: string[],
    client: ClientType
  ): Promise<Record<string, number | string | null>>;
  getPrices?(
    symbols: string[],
    client: ClientType
  ): Promise<{
    markPricesBySymbol: Record<string, number | string | null>;
    indexPricesBySymbol: Record<string, number | string | null>;
  }>;
}

export interface MarginMarketParamsSnapshot {
  paramsBySymbol: MarketParamsBySymbol;
  spotCollaterals: SpotCollateralParams[];
  updatedAt: number;
  slot?: number;
}

export interface MarginMarketParamsStoreOptions<
  ClientType extends MarginClient = MarginClient,
> {
  client: ClientType;
  ttlMs?: number;
  autoRefreshIntervalMs?: number;
  missingPriceBehavior?: MissingPriceBehavior;
  priceSource?: MarketPriceSource<ClientType>;
  now?: () => number;
}

const DEFAULT_TTL_MS = 30_000;

const QUOTE_LOTS_PER_USD = 1_000_000n;
const RISK_FACTOR_BPS_MAX = 10_000;

const resolveMarketsClient = (client: MarginClient): MarginMarketsClient => {
  if ("markets" in client) {
    return typeof client.markets === "function"
      ? client.markets()
      : client.markets;
  }

  throw new Error("Margin client does not expose public market endpoints");
};

const resolveCollateralClient = (
  client: MarginClient
): MarginCollateralClient | undefined => {
  const collateral = client.collateral;
  return typeof collateral === "function" ? collateral() : collateral;
};

const expandScientificNotation = (value: string): string => {
  const match = value
    .trim()
    .toLowerCase()
    .match(/^([+-]?)(\d+)(?:\.(\d*))?e([+-]?\d+)$/);

  if (!match) {
    return value;
  }

  const sign = match[1] ?? "";
  const integerPart = match[2] ?? "0";
  const fractionalPart = match[3] ?? "";
  const exponent = Number(match[4]);

  const digits = `${integerPart}${fractionalPart}`;
  const decimalIndex = integerPart.length + exponent;

  if (decimalIndex <= 0) {
    return `${sign}0.${"0".repeat(-decimalIndex)}${digits}`;
  }

  if (decimalIndex >= digits.length) {
    return `${sign}${digits}${"0".repeat(decimalIndex - digits.length)}`;
  }

  return `${sign}${digits.slice(0, decimalIndex)}.${digits.slice(decimalIndex)}`;
};

const parseDecimalRatio = (
  value: number | string
): { numerator: bigint; denominator: bigint } => {
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error("mark price is not finite");
    }
    return parseDecimalRatio(value.toString());
  }

  const normalized = expandScientificNotation(value);
  const trimmed = normalized.trim();
  if (!trimmed) {
    throw new Error("mark price is empty");
  }

  let sign = 1n;
  let digits = trimmed;
  if (digits.startsWith("+")) {
    digits = digits.slice(1);
  } else if (digits.startsWith("-")) {
    sign = -1n;
    digits = digits.slice(1);
  }

  const match = digits.match(/^(\d+)(?:\.(\d*))?$/);
  if (!match) {
    throw new Error(`invalid mark price: ${value}`);
  }
  const whole = match[1] ?? "0";
  const fraction = match[2] ?? "";
  const denominator = 10n ** BigInt(fraction.length);
  const numerator = BigInt(`${whole}${fraction}` || "0");
  return { numerator: sign * numerator, denominator };
};

export const priceUsdToTicks = (
  priceUsd: number | string,
  units: { baseLotsDecimals: number; tickSizeInQuoteLotsPerBaseLot: number }
): string => {
  const price = parseDecimalRatio(priceUsd);
  if (price.numerator < 0n) {
    throw new Error("mark price must be non-negative");
  }
  const tickSize = BigInt(units.tickSizeInQuoteLotsPerBaseLot);
  if (tickSize === 0n) {
    throw new Error("tick size denominator is zero");
  }

  const scale = 10n ** BigInt(Math.abs(units.baseLotsDecimals));
  let numerator = price.numerator * QUOTE_LOTS_PER_USD;
  let denominator = price.denominator * tickSize;

  if (units.baseLotsDecimals >= 0) {
    denominator *= scale;
  } else {
    numerator *= scale;
  }

  return ((2n * numerator + denominator) / (2n * denominator)).toString();
};

const riskFactorBpsToMarginBps = (value: number, field: string): string => {
  if (!Number.isFinite(value)) {
    throw new Error(`${field} risk factor is not finite`);
  }
  if (value < 0) {
    throw new Error(`${field} risk factor must be non-negative`);
  }

  const basisPoints = value;
  if (basisPoints > RISK_FACTOR_BPS_MAX) {
    throw new Error(
      `${field} risk factor must be at most ${RISK_FACTOR_BPS_MAX} bps`
    );
  }

  const rounded = Math.round(basisPoints);
  if (Math.abs(rounded - basisPoints) > 1e-9) {
    throw new Error(`${field} risk factor must resolve to whole bps`);
  }

  return rounded.toString();
};

export const buildMarketParamsFromSummary = (
  market: MarketSummary,
  markPriceUsd: number | string | null | undefined,
  opts?: MarketParamsBuildOptions
): MarketParams | null => {
  const missingPriceBehavior = opts?.missingPriceBehavior ?? "error";

  if (markPriceUsd === null || markPriceUsd === undefined) {
    if (missingPriceBehavior === "skip") {
      return null;
    }
    if (missingPriceBehavior === "zero") {
      markPriceUsd = 0;
    } else {
      throw new Error(`Missing mark price for ${String(market.symbol)}`);
    }
  }

  return {
    symbol: market.symbol,
    assetId: market.assetId,
    markPriceTicks: priceUsdToTicks(markPriceUsd, market.units),
    ...(opts?.indexPricesBySymbol?.[String(market.symbol)] == null
      ? {}
      : {
          indexPriceTicks: priceUsdToTicks(
            opts.indexPricesBySymbol[String(market.symbol)]!,
            market.units
          ),
        }),
    tickSize: market.units.tickSizeInQuoteLotsPerBaseLot.toString(),
    baseLotDecimals: market.units.baseLotsDecimals,
    leverageTiers: market.leverageTiers.map((tier) => ({
      upperBoundSize: tier.maxSizeBaseLots.toString(),
      maxLeverage: tier.maxLeverage.toString(),
      limitOrderRiskFactorBps: riskFactorBpsToMarginBps(
        tier.limitOrderRiskFactor,
        "leverage_tier.limit_order_risk_factor"
      ),
    })),
    riskFactors: {
      maintenanceMarginFactorBps: riskFactorBpsToMarginBps(
        market.riskFactors.maintenance,
        "maintenance"
      ),
      backstopMarginFactorBps: riskFactorBpsToMarginBps(
        market.riskFactors.backstop,
        "backstop"
      ),
      highRiskMarginFactorBps: riskFactorBpsToMarginBps(
        market.riskFactors.highRisk,
        "high_risk"
      ),
    },
    cancelOrderRiskFactorBps: riskFactorBpsToMarginBps(
      market.riskFactors.cancelOrder,
      "cancel_order"
    ),
    upnlRiskFactor: riskFactorBpsToMarginBps(market.riskFactors.upnl, "upnl"),
    upnlRiskFactorForWithdrawals: riskFactorBpsToMarginBps(
      market.riskFactors.upnlForWithdrawals,
      "upnl_for_withdrawals"
    ),
    isolatedOnly: market.isolatedOnly,
  };
};

export const buildMarketParamsBySymbol = (
  markets: MarketSummary[],
  markPricesBySymbol: Record<string, number | string | null>,
  opts?: MarketParamsBuildOptions
): MarketParamsBySymbol => {
  const paramsBySymbol: MarketParamsBySymbol = {};
  for (const market of markets) {
    const symbol = String(market.symbol);
    const params = buildMarketParamsFromSummary(
      market,
      markPricesBySymbol[symbol],
      opts
    );
    if (!params) {
      continue;
    }
    paramsBySymbol[symbol] = params;
  }
  return paramsBySymbol;
};

const getDefaultPrices = async (
  symbols: string[],
  client: MarginClient
): Promise<{
  markPricesBySymbol: Record<string, number | string | null>;
  indexPricesBySymbol: Record<string, number | string | null>;
}> => {
  const marketsClient = resolveMarketsClient(client);
  const entries = await Promise.all(
    symbols.map(async (symbol) => {
      const market = await marketsClient.getMarket(symbol);
      return [
        symbol,
        market.market.markPrice?.price ?? null,
        market.market.spotPrice?.price ?? null,
      ] as const;
    })
  );
  return {
    markPricesBySymbol: Object.fromEntries(
      entries.map(([symbol, markPrice]) => [symbol, markPrice])
    ),
    indexPricesBySymbol: Object.fromEntries(
      entries.map(([symbol, , indexPrice]) => [symbol, indexPrice])
    ),
  };
};

/**
 * The margin engine's view of the `/v1/collateral/assets` registry.
 *
 * This is the **canonical** DTO → `SpotCollateralParams` conversion: the two
 * shapes deliberately differ (wire fields the margin engine never reads, and
 * `perpSymbol`, which is a join through the market map rather than a wire
 * field), and every consumer should go through here so they cannot drift
 * apart silently. Assets that do not currently count toward margin (the quote
 * asset, inactive spot assets, or ones not yet bound to a perp market) are
 * skipped; an *active* asset referencing an unknown market throws — that is a
 * live server contradicting itself, not an old one.
 */
export const spotCollateralParamsFromAssets = (
  spotCollaterals: CollateralAssetsResponse,
  symbolsByAssetId: ReadonlyMap<number, string>
): SpotCollateralParams[] =>
  spotCollaterals.assets.flatMap((asset) => {
    const spot = asset.spot;
    if (!spot?.isActive || spot.perpAssetIndex == null) {
      return [];
    }
    const perpSymbol = symbolsByAssetId.get(spot.perpAssetIndex);
    if (!perpSymbol) {
      throw new Error(
        `Collateral ${asset.symbol} references unknown perp asset ${spot.perpAssetIndex}`
      );
    }
    return [
      {
        assetIndex: asset.assetIndex,
        symbol: asset.symbol,
        perpSymbol,
        decimals: asset.decimals,
        maxPerTraderBalance: spot.maxPerTraderBalance,
        maxGlobalBalance: spot.maxGlobalBalance,
        currGlobalBalance: spot.currGlobalBalance,
        minMarginDiscountBps: spot.minMarginDiscountBps,
        maxMarginDiscountBps: spot.maxMarginDiscountBps,
      },
    ];
  });

const defaultPriceSource: MarketPriceSource = {
  async getMarkPrices(symbols, client) {
    return (await getDefaultPrices(symbols, client)).markPricesBySymbol;
  },
  async getIndexPrices(symbols, client) {
    return (await getDefaultPrices(symbols, client)).indexPricesBySymbol;
  },
  async getPrices(symbols, client) {
    return getDefaultPrices(symbols, client);
  },
};

export class MarginMarketParamsStore<
  ClientType extends MarginClient = MarginClient,
> {
  private client: ClientType;
  private ttlMs: number;
  private autoRefreshIntervalMs?: number;
  private priceSource: MarketPriceSource<ClientType>;
  private missingPriceBehavior: MissingPriceBehavior;
  private now: () => number;
  private refreshPromise: Promise<MarginMarketParamsSnapshot> | null = null;
  private lastSnapshot: MarginMarketParamsSnapshot | null = null;
  private lastMarkets: MarketSummary[] | null = null;
  private lastIndexPrices: Record<string, number | string | null> = {};
  private lastSpotCollaterals: SpotCollateralParams[] = [];
  private listeners = new Set<(snapshot: MarginMarketParamsSnapshot) => void>();
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor(opts: MarginMarketParamsStoreOptions<ClientType>) {
    this.client = opts.client;
    this.ttlMs = opts.ttlMs ?? DEFAULT_TTL_MS;
    this.autoRefreshIntervalMs = opts.autoRefreshIntervalMs;
    this.priceSource = opts.priceSource ?? defaultPriceSource;
    this.missingPriceBehavior = opts.missingPriceBehavior ?? "error";
    this.now = opts.now ?? Date.now;

    if (this.autoRefreshIntervalMs !== undefined) {
      this.startAutoRefresh();
    }
  }

  subscribe(
    listener: (snapshot: MarginMarketParamsSnapshot) => void
  ): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async getMarketParamsBySymbol(): Promise<MarketParamsBySymbol> {
    const snapshot = await this.getSnapshot();
    return snapshot.paramsBySymbol;
  }

  async getCalculator(): Promise<MarginCalculator> {
    const snapshot = await this.getSnapshot();
    return createMarginCalculator(
      Object.values(snapshot.paramsBySymbol),
      snapshot.spotCollaterals
    );
  }

  async getSnapshot(): Promise<MarginMarketParamsSnapshot> {
    if (this.lastSnapshot && !this.isExpired()) {
      return this.lastSnapshot;
    }
    return this.refresh();
  }

  async refresh(): Promise<MarginMarketParamsSnapshot> {
    if (this.refreshPromise) {
      return this.refreshPromise;
    }

    this.refreshPromise = this.refreshInternal().finally(() => {
      this.refreshPromise = null;
    });

    return this.refreshPromise;
  }

  updateMarkPrices(
    markPricesBySymbol: Record<string, number | string | null>
  ): void {
    if (!this.lastMarkets) {
      throw new Error("Market summaries have not been loaded yet");
    }

    const paramsBySymbol = buildMarketParamsBySymbol(
      this.lastMarkets,
      markPricesBySymbol,
      {
        missingPriceBehavior: this.missingPriceBehavior,
        indexPricesBySymbol: this.lastIndexPrices,
      }
    );
    this.updateSnapshot(
      paramsBySymbol,
      this.lastSpotCollaterals,
      this.lastSnapshot?.slot
    );
  }

  startAutoRefresh(): void {
    if (this.timer || this.autoRefreshIntervalMs === undefined) {
      return;
    }
    this.timer = setInterval(() => {
      void this.refresh().catch(() => undefined);
    }, this.autoRefreshIntervalMs);
  }

  stopAutoRefresh(): void {
    if (!this.timer) {
      return;
    }
    clearInterval(this.timer);
    this.timer = null;
  }

  private isExpired(): boolean {
    if (!this.lastSnapshot) {
      return true;
    }
    if (!Number.isFinite(this.ttlMs)) {
      return false;
    }
    return this.now() - this.lastSnapshot.updatedAt > this.ttlMs;
  }

  private async refreshInternal(): Promise<MarginMarketParamsSnapshot> {
    const marketsClient = resolveMarketsClient(this.client);
    const marketsResponse = await marketsClient.getMarkets();
    const markets = marketsResponse.markets;
    const symbols = markets.map((market) => String(market.symbol));
    const collateralClient = resolveCollateralClient(this.client);
    const prices = this.priceSource.getPrices
      ? this.priceSource.getPrices(symbols, this.client)
      : Promise.all([
          this.priceSource.getMarkPrices(symbols, this.client),
          this.priceSource.getIndexPrices?.(symbols, this.client) ??
            Promise.resolve({}),
        ]).then(([markPricesBySymbol, indexPricesBySymbol]) => ({
          markPricesBySymbol,
          indexPricesBySymbol,
        }));
    const [{ markPricesBySymbol, indexPricesBySymbol }, spotCollaterals] =
      await Promise.all([
        prices,
        // Servers that predate /v1/collateral/assets have no spot collateral
        // to value, so a failed fetch degrades to empty params (spot valued at
        // zero — the feature's fail-open-to-under-valuation rule) instead of
        // taking the whole market-params refresh down with it.
        collateralClient
          ?.getAssets()
          .catch((): CollateralAssetsResponse => ({ assets: [] })) ??
          Promise.resolve<CollateralAssetsResponse>({ assets: [] }),
      ]);

    this.lastMarkets = markets;
    this.lastIndexPrices = indexPricesBySymbol;
    this.lastSpotCollaterals = spotCollateralParamsFromAssets(
      spotCollaterals,
      new Map(markets.map((market) => [market.assetId, String(market.symbol)]))
    );
    const paramsBySymbol = buildMarketParamsBySymbol(
      markets,
      markPricesBySymbol,
      {
        missingPriceBehavior: this.missingPriceBehavior,
        indexPricesBySymbol,
      }
    );
    return this.updateSnapshot(
      paramsBySymbol,
      this.lastSpotCollaterals,
      marketsResponse.slot
    );
  }

  private updateSnapshot(
    paramsBySymbol: MarketParamsBySymbol,
    spotCollaterals: SpotCollateralParams[],
    slot: number | undefined
  ): MarginMarketParamsSnapshot {
    const snapshot = {
      paramsBySymbol,
      spotCollaterals,
      updatedAt: this.now(),
      slot,
    };
    this.lastSnapshot = snapshot;
    for (const listener of this.listeners) {
      listener(snapshot);
    }
    return snapshot;
  }
}
