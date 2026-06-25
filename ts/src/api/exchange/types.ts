import z from "zod";
import { TokenAmountSchema, type TokenAmount } from "@/primitives/TokenAmount";
import { numericBigint } from "@/ws/numericSchemas";

// ---------------------------------------------------------------------------
// Authority Set
// ---------------------------------------------------------------------------

export interface AuthoritySet {
  rootAuthority: string;
  riskAuthority: string;
  marketAuthority: string;
  oracleAuthority: string;
  adlAuthority: string;
  cancelAuthority: string;
  backstopAuthority: string;
}

export const AuthoritySetSchema: z.ZodType<AuthoritySet> = z.object({
  rootAuthority: z.string(),
  riskAuthority: z.string(),
  marketAuthority: z.string(),
  oracleAuthority: z.string(),
  adlAuthority: z.string(),
  cancelAuthority: z.string(),
  backstopAuthority: z.string(),
});

// ---------------------------------------------------------------------------
// Exchange Keys
// ---------------------------------------------------------------------------

export interface ExchangeKeys {
  globalConfig: string;
  currentAuthorities: AuthoritySet;
  pendingAuthorities: AuthoritySet;
  canonicalMint: string;
  globalVault: string;
  perpAssetMap: string;
  globalTraderIndex: string[];
  activeTraderBuffer: string[];
  withdrawQueue: string;
}

export const ExchangeKeysSchema: z.ZodType<ExchangeKeys> = z.object({
  globalConfig: z.string(),
  currentAuthorities: AuthoritySetSchema,
  pendingAuthorities: AuthoritySetSchema,
  canonicalMint: z.string(),
  globalVault: z.string(),
  perpAssetMap: z.string(),
  globalTraderIndex: z.array(z.string()),
  activeTraderBuffer: z.array(z.string()),
  withdrawQueue: z.string(),
});

// ---------------------------------------------------------------------------
// Exchange Status View
// ---------------------------------------------------------------------------

export interface ExchangeStatusView {
  active: boolean;
  gated: boolean;
  withdrawalsAvailable: boolean;
}

export const ExchangeStatusViewSchema: z.ZodType<ExchangeStatusView> = z.object(
  {
    active: z.boolean(),
    gated: z.boolean(),
    withdrawalsAvailable: z.boolean().default(true),
  }
);

// ---------------------------------------------------------------------------
// Exchange Market Config
// ---------------------------------------------------------------------------

export interface ExchangeLeverageTier {
  maxLeverage: number;
  maxSizeBaseLots: number;
  /** Limit order risk factor as a percentage (e.g. 60 = 60%). */
  limitOrderRiskFactor: number;
  /** Limit order risk factor in basis points (e.g. 6000 = 60%). */
  limitOrderRiskFactorBps?: number;
}

export const ExchangeLeverageTierSchema: z.ZodType<ExchangeLeverageTier> =
  z.object({
    maxLeverage: z.number(),
    maxSizeBaseLots: z.number(),
    limitOrderRiskFactor: z.number(),
    limitOrderRiskFactorBps: z.number().optional(),
  });

export interface ExchangeRiskFactors {
  /** Maintenance margin risk factor as a percentage (e.g. 50 = 50%). */
  maintenance: number;
  /** Maintenance margin risk factor in basis points (e.g. 5000 = 50%). */
  maintenanceBps?: number;
  /** Backstop liquidation risk factor as a percentage. */
  backstop: number;
  /** Backstop liquidation risk factor in basis points. */
  backstopBps?: number;
  /** High-risk threshold as a percentage. */
  highRisk: number;
  /** High-risk threshold in basis points. */
  highRiskBps?: number;
  /** Positive unrealized PnL discount factor as a percentage. */
  upnl: number;
  /** Positive unrealized PnL discount factor in basis points. */
  upnlBps?: number;
  /** Positive unrealized PnL discount factor for withdrawals as a percentage. */
  upnlForWithdrawals: number;
  /** Positive unrealized PnL discount factor for withdrawals in basis points. */
  upnlForWithdrawalsBps?: number;
  /** Cancel-order risk factor as a percentage. */
  cancelOrder: number;
  /** Cancel-order risk factor in basis points. */
  cancelOrderBps?: number;
}

export const ExchangeRiskFactorsSchema: z.ZodType<ExchangeRiskFactors> =
  z.object({
    maintenance: z.number(),
    maintenanceBps: z.number().optional(),
    backstop: z.number(),
    backstopBps: z.number().optional(),
    highRisk: z.number(),
    highRiskBps: z.number().optional(),
    upnl: z.number(),
    upnlBps: z.number().optional(),
    upnlForWithdrawals: z.number(),
    upnlForWithdrawalsBps: z.number().optional(),
    cancelOrder: z.number(),
    cancelOrderBps: z.number().optional(),
  });

export interface MarketCalendar {
  id: string;
  description: string;
  calendarUri: string;
  contentSha256: string;
  nextMarketTransitionUtc?: string | null;
}

export const MarketCalendarSchema: z.ZodType<MarketCalendar> = z.object({
  id: z.string(),
  description: z.string(),
  calendarUri: z.string(),
  contentSha256: z.string(),
  nextMarketTransitionUtc: z.string().nullable().optional(),
});

export interface MarketPublicMetadata {
  name?: string | null;
  description?: string | null;
  logoUri?: string | null;
  coinGeckoId?: string | null;
  coinMarketCapId?: number | null;
  tokensXyzAssetId?: string | null;
  calendar?: MarketCalendar | null;
  displayColor?: string | null;
}

export const MarketPublicMetadataSchema: z.ZodType<MarketPublicMetadata> =
  z.object({
    name: z.string().nullable().optional(),
    description: z.string().nullable().optional(),
    logoUri: z.string().nullable().optional(),
    coinGeckoId: z.string().nullable().optional(),
    coinMarketCapId: z.number().nullable().optional(),
    tokensXyzAssetId: z.string().nullable().optional(),
    calendar: MarketCalendarSchema.nullable().optional(),
    displayColor: z.string().nullable().optional(),
  });

export interface MarketStatsSnapshot {
  slot: number;
  slotIndex: number;
  openInterestBaseLots: string;
  fundingStartIntervalTimestamp: string;
  cumulativeFundingRate: string;
}

export const MarketStatsSnapshotSchema: z.ZodType<MarketStatsSnapshot> =
  z.object({
    slot: z.number(),
    slotIndex: z.number(),
    openInterestBaseLots: z.union([z.string(), z.number()]).transform(String),
    fundingStartIntervalTimestamp: z
      .union([z.string(), z.number()])
      .transform(String),
    cumulativeFundingRate: z.union([z.string(), z.number()]).transform(String),
  });

export interface ExchangeMarketConfig {
  symbol: string;
  assetId: number;
  marketStatus: string;
  commodityStatus?: string;
  commodityMetadata?: ExchangeViewCommodityMetadata | null;
  metadata?: MarketPublicMetadata | null;
  marketPubkey: string;
  splinePubkey: string;
  tickSize: number;
  baseLotsDecimals: number;
  takerFee: number;
  makerFee: number;
  leverageTiers: ExchangeLeverageTier[];
  riskFactors: ExchangeRiskFactors;
  fundingIntervalSeconds: number;
  fundingPeriodSeconds: number;
  maxFundingRatePerInterval: number;
  maxFundingRatePerIntervalPercentage: number;
  openInterestCapBaseLots: string;
  maxLiquidationSizeBaseLots: string;
  isolatedOnly: boolean;
  statsSnapshot?: MarketStatsSnapshot;
}

export const ExchangeMarketConfigSchema: z.ZodType<ExchangeMarketConfig> =
  z.object({
    symbol: z.string(),
    assetId: z.number(),
    marketStatus: z.string(),
    commodityStatus: z.string().optional(),
    commodityMetadata: z
      .lazy(() => ExchangeViewCommodityMetadataSchema)
      .nullable()
      .optional(),
    metadata: MarketPublicMetadataSchema.nullable().optional(),
    marketPubkey: z.string(),
    splinePubkey: z.string(),
    tickSize: z.number(),
    baseLotsDecimals: z.number(),
    takerFee: z.number(),
    makerFee: z.number(),
    leverageTiers: z.array(ExchangeLeverageTierSchema),
    riskFactors: ExchangeRiskFactorsSchema,
    fundingIntervalSeconds: z.number(),
    fundingPeriodSeconds: z.number(),
    maxFundingRatePerInterval: z.number(),
    maxFundingRatePerIntervalPercentage: z.number(),
    openInterestCapBaseLots: z
      .union([z.string(), z.number()])
      .transform(String),
    maxLiquidationSizeBaseLots: z
      .union([z.string(), z.number()])
      .transform(String),
    isolatedOnly: z.boolean(),
    statsSnapshot: MarketStatsSnapshotSchema.optional(),
  });

export interface ExchangeViewMarketPriceBand {
  min: TokenAmount;
  max: TokenAmount;
}

export const ExchangeViewMarketPriceBandSchema: z.ZodType<ExchangeViewMarketPriceBand> =
  z.object({
    min: TokenAmountSchema,
    max: TokenAmountSchema,
  });

export interface ExchangeViewCommodityMetadata {
  isCommodity: boolean;
  isReopen: boolean;
  isAfterHours: boolean;
  status: string;
  afterHoursRadius: TokenAmount;
  lastKnownIndexPrice?: TokenAmount | null;
  markPriceBand?: ExchangeViewMarketPriceBand | null;
  executionPriceBand?: ExchangeViewMarketPriceBand | null;
  lastIndexExpiryTimestamp?: number | null;
}

export const ExchangeViewCommodityMetadataSchema: z.ZodType<ExchangeViewCommodityMetadata> =
  z.object({
    isCommodity: z.boolean(),
    isReopen: z.boolean(),
    isAfterHours: z.boolean(),
    status: z.string(),
    afterHoursRadius: TokenAmountSchema,
    lastKnownIndexPrice: TokenAmountSchema.nullable().optional(),
    markPriceBand: ExchangeViewMarketPriceBandSchema.nullable().optional(),
    executionPriceBand: ExchangeViewMarketPriceBandSchema.nullable().optional(),
    lastIndexExpiryTimestamp: z.number().nullable().optional(),
  });

// ---------------------------------------------------------------------------
// Exchange View (full config response)
// ---------------------------------------------------------------------------

export interface ExchangeConfig {
  keys: ExchangeKeys;
  markets: ExchangeMarketConfig[];
}

export const ExchangeConfigSchema: z.ZodType<ExchangeConfig> = z.object({
  keys: ExchangeKeysSchema,
  markets: z.array(ExchangeMarketConfigSchema),
});

// ---------------------------------------------------------------------------
// Exchange Snapshot / Cache Types
// ---------------------------------------------------------------------------

export interface ExchangeWsLeverageTier {
  maxLeverage: number;
  maxSizeBaseLots: bigint;
  /** Limit order risk factor in basis points (e.g. 6000 = 60%). */
  limitOrderRiskFactor: number;
}

export const ExchangeWsLeverageTierSchema: z.ZodType<ExchangeWsLeverageTier> =
  z.object({
    maxLeverage: z.number(),
    maxSizeBaseLots: numericBigint("maxSizeBaseLots"),
    limitOrderRiskFactor: z.number(),
  });

export interface ExchangeWsFundingConfig {
  fundingIntervalSeconds: number;
  fundingPeriodSeconds: number;
  maxFundingRatePerInterval: number;
}

export const ExchangeWsFundingConfigSchema: z.ZodType<ExchangeWsFundingConfig> =
  z.object({
    fundingIntervalSeconds: z.number(),
    fundingPeriodSeconds: z.number(),
    maxFundingRatePerInterval: z.number(),
  });

export interface ExchangeWsFeeConfig {
  takerFee: number;
  makerFee: number;
}

export const ExchangeWsFeeConfigSchema: z.ZodType<ExchangeWsFeeConfig> =
  z.object({
    takerFee: z.number(),
    makerFee: z.number(),
  });

export type ExchangeWsValidationRule = "ignore" | "require" | "forbid";

export const ExchangeWsValidationRuleSchema: z.ZodType<ExchangeWsValidationRule> =
  z.enum(["ignore", "require", "forbid"]);

export type ExchangeWsRiskActionPriceValidityRules =
  ExchangeWsValidationRule[][][];

export const ExchangeWsRiskActionPriceValidityRulesSchema: z.ZodType<ExchangeWsRiskActionPriceValidityRules> =
  z.array(z.array(z.array(ExchangeWsValidationRuleSchema)));

export interface ExchangeWsMarkPriceParameters {
  emaPeriodSlots: bigint;
  emaDiffRadius: bigint;
  bookPriceRadius: bigint;
  commoditiesAfterHoursRadius: bigint;
  commoditiesAfterHoursRadiusBps: bigint;
  adjustedExchangeSpotPriceWeight: bigint;
  bookPriceWeight: bigint;
  exchangePerpPriceWeight: bigint;
  spotPriceStaleThreshold: bigint;
  bookPriceStaleThreshold: bigint;
  bookHardStaleMultiplier?: number;
  perpPriceStaleThreshold: bigint;
  riskActionPriceValidityRules: ExchangeWsRiskActionPriceValidityRules;
  oracleDivergenceRadius: number;
  oracleHardStaleMultiplier?: number;
  minOracleResponses: number;
}

export const ExchangeWsMarkPriceParametersSchema: z.ZodType<ExchangeWsMarkPriceParameters> =
  z.object({
    emaPeriodSlots: numericBigint("emaPeriodSlots"),
    emaDiffRadius: numericBigint("emaDiffRadius"),
    bookPriceRadius: numericBigint("bookPriceRadius"),
    commoditiesAfterHoursRadius: numericBigint("commoditiesAfterHoursRadius"),
    commoditiesAfterHoursRadiusBps: numericBigint(
      "commoditiesAfterHoursRadiusBps"
    ).default(0n),
    adjustedExchangeSpotPriceWeight: numericBigint(
      "adjustedExchangeSpotPriceWeight"
    ),
    bookPriceWeight: numericBigint("bookPriceWeight"),
    exchangePerpPriceWeight: numericBigint("exchangePerpPriceWeight"),
    spotPriceStaleThreshold: numericBigint("spotPriceStaleThreshold"),
    bookPriceStaleThreshold: numericBigint("bookPriceStaleThreshold"),
    bookHardStaleMultiplier: z.number().default(0),
    perpPriceStaleThreshold: numericBigint("perpPriceStaleThreshold"),
    riskActionPriceValidityRules: ExchangeWsRiskActionPriceValidityRulesSchema,
    oracleDivergenceRadius: z.number(),
    oracleHardStaleMultiplier: z.number().default(0),
    minOracleResponses: z.number(),
  });

export interface ExchangeWsMarketPriceBand {
  lower: TokenAmount;
  upper: TokenAmount;
}

export const ExchangeWsMarketPriceBandSchema: z.ZodType<ExchangeWsMarketPriceBand> =
  z.object({
    lower: TokenAmountSchema,
    upper: TokenAmountSchema,
  });

export interface ExchangeWsCommodityMetadata {
  isCommodity: boolean;
  isReopen: boolean;
  isAfterHours: boolean;
  status: string;
  afterHoursRadius: TokenAmount;
  lastKnownIndexPrice?: TokenAmount | null;
  markPriceBand?: ExchangeWsMarketPriceBand | null;
  executionPriceBand?: ExchangeWsMarketPriceBand | null;
  lastIndexExpiryTimestamp?: number | null;
}

export const ExchangeWsCommodityMetadataSchema: z.ZodType<ExchangeWsCommodityMetadata> =
  z.object({
    isCommodity: z.boolean(),
    isReopen: z.boolean(),
    isAfterHours: z.boolean(),
    status: z.string(),
    afterHoursRadius: TokenAmountSchema,
    lastKnownIndexPrice: TokenAmountSchema.nullable().optional(),
    markPriceBand: ExchangeWsMarketPriceBandSchema.nullable().optional(),
    executionPriceBand: ExchangeWsMarketPriceBandSchema.nullable().optional(),
    lastIndexExpiryTimestamp: z.number().nullable().optional(),
  });

export interface ExchangeStateSnapshot {
  programId: string;
  globalConfig: string;
  currentAuthorities: AuthoritySet;
  canonicalMint: string;
  usdcMint: string;
  globalVault: string;
  perpAssetMap: string;
  globalTraderIndex: string[];
  activeTraderBuffer: string[];
  withdrawQueue: string;
  exchangeStatusBits: number;
  exchangeStatusFeatures: string[];
  active: boolean;
  gated: boolean;
  withdrawalsAvailable: boolean;
}

export const ExchangeStateSnapshotSchema: z.ZodType<ExchangeStateSnapshot> =
  z.object({
    programId: z.string(),
    globalConfig: z.string(),
    currentAuthorities: AuthoritySetSchema,
    canonicalMint: z.string(),
    usdcMint: z.string(),
    globalVault: z.string(),
    perpAssetMap: z.string(),
    globalTraderIndex: z.array(z.string()),
    activeTraderBuffer: z.array(z.string()),
    withdrawQueue: z.string(),
    exchangeStatusBits: z.number(),
    exchangeStatusFeatures: z.array(z.string()),
    active: z.boolean(),
    gated: z.boolean(),
    withdrawalsAvailable: z.boolean().default(true),
  });

export interface ExchangeMarketSnapshot {
  symbol: string;
  assetId: number;
  marketStatus: string;
  marketPubkey: string;
  splinePubkey: string;
  tickSize: number;
  baseLotsDecimals: number;
  takerFee: number;
  makerFee: number;
  leverageTiers: ExchangeWsLeverageTier[];
  riskFactors: ExchangeRiskFactors;
  fundingConfig: ExchangeWsFundingConfig;
  openInterestCapBaseLots: bigint;
  maxLiquidationSizeBaseLots: bigint;
  isolatedOnly: boolean;
  markPriceParameters: ExchangeWsMarkPriceParameters;
  commodityMetadata?: ExchangeWsCommodityMetadata | null;
  metadata?: MarketPublicMetadata | null;
}

export const ExchangeMarketSnapshotSchema: z.ZodType<ExchangeMarketSnapshot> =
  z.object({
    symbol: z.string(),
    assetId: z.number(),
    marketStatus: z.string(),
    marketPubkey: z.string(),
    splinePubkey: z.string(),
    tickSize: z.number(),
    baseLotsDecimals: z.number(),
    takerFee: z.number(),
    makerFee: z.number(),
    leverageTiers: z.array(ExchangeWsLeverageTierSchema),
    riskFactors: ExchangeRiskFactorsSchema,
    fundingConfig: ExchangeWsFundingConfigSchema,
    openInterestCapBaseLots: numericBigint("openInterestCapBaseLots"),
    maxLiquidationSizeBaseLots: numericBigint("maxLiquidationSizeBaseLots"),
    isolatedOnly: z.boolean(),
    markPriceParameters: ExchangeWsMarkPriceParametersSchema,
    commodityMetadata: ExchangeWsCommodityMetadataSchema.nullable().optional(),
    metadata: MarketPublicMetadataSchema.nullable().optional(),
  });

export type ExchangeSnapshotEncoding = "json" | "base64+zstd";

export const ExchangeSnapshotEncodingSchema: z.ZodType<ExchangeSnapshotEncoding> =
  z.union([z.literal("json"), z.literal("base64+zstd")]);

export interface ExchangeSnapshotView {
  version: number;
  sequenceNumber?: bigint;
  slot: bigint;
  slotIndex: number;
  exchange: ExchangeStateSnapshot;
  markets: ExchangeMarketSnapshot[];
}

export const ExchangeSnapshotViewSchema: z.ZodType<ExchangeSnapshotView> =
  z.object({
    version: z.number(),
    sequenceNumber: numericBigint("sequenceNumber").optional(),
    slot: numericBigint("slot"),
    slotIndex: z.number(),
    exchange: ExchangeStateSnapshotSchema,
    markets: z.array(ExchangeMarketSnapshotSchema),
  });
