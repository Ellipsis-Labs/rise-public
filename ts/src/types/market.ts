import z from "zod";
import { type TokenAmount, TokenAmountSchema } from "@/primitives/TokenAmount";
import { type Symbol, symbol } from "@/primitives/Symbol";

const zSymbol = z.string().transform(symbol);

// ---------------------------------------------------------------------------
// Market sub-types
// ---------------------------------------------------------------------------

export interface MarketUnits {
  tickSizeInQuoteLotsPerBaseLot: number;
  baseLotsDecimals: number;
}

export const MarketUnitsSchema: z.ZodType<MarketUnits> = z.object({
  tickSizeInQuoteLotsPerBaseLot: z.number(),
  baseLotsDecimals: z.number(),
});

export interface MarketFees {
  takerFeeMicro: number;
  makerFeeMicro: number;
}

export const MarketFeesSchema: z.ZodType<MarketFees> = z.object({
  takerFeeMicro: z.number(),
  // Older payloads may omit maker fees; default to 0 for compatibility
  makerFeeMicro: z.number().default(0).catch(0),
});

export interface MarketLeverageTier {
  maxLeverage: number;
  maxSizeBaseLots: number;
  /** Limit order risk factor in basis points (e.g. 6000 = 60%). */
  limitOrderRiskFactor: number;
}

export const LeverageTierSchema: z.ZodType<MarketLeverageTier> = z.object({
  maxLeverage: z.number(),
  maxSizeBaseLots: z.number(),
  // Older API payloads may omit this; default to 0 for backward compatibility.
  limitOrderRiskFactor: z.number().default(0).catch(0),
});

export interface RiskFactors {
  /** Maintenance margin risk factor in basis points (e.g. 5000 = 50%). */
  maintenance: number;
  /** Backstop liquidation risk factor in basis points. */
  backstop: number;
  /** High-risk threshold in basis points. */
  highRisk: number;
  /** Positive unrealized PnL discount factor in basis points. */
  upnl: number;
  /** Positive unrealized PnL discount factor for withdrawals in basis points. */
  upnlForWithdrawals: number;
  /** Cancel-order risk factor in basis points. */
  cancelOrder: number;
}

export const RiskFactorsSchema: z.ZodType<RiskFactors> = z.object({
  maintenance: z.number(),
  backstop: z.number(),
  highRisk: z.number(),
  // Added fields default to 0 to tolerate older server responses/fixtures
  upnl: z.number().default(0).catch(0),
  upnlForWithdrawals: z.number().default(0).catch(0),
  cancelOrder: z.number().default(0).catch(0),
});

export interface MarketPriceBand {
  min: TokenAmount;
  max: TokenAmount;
}

export const MarketPriceBandSchema: z.ZodType<MarketPriceBand> = z.object({
  min: TokenAmountSchema,
  max: TokenAmountSchema,
});

export interface MarketCommodityMetadata {
  isCommodity: boolean;
  isReopen: boolean;
  isAfterHours: boolean;
  status: string;
  afterHoursRadius: TokenAmount;
  lastKnownIndexPrice?: TokenAmount | null;
  markPriceBand?: MarketPriceBand | null;
  executionPriceBand?: MarketPriceBand | null;
  lastIndexExpiryTimestamp?: number | null;
}

export const MarketCommodityMetadataSchema: z.ZodType<MarketCommodityMetadata> =
  z.object({
    isCommodity: z.boolean(),
    isReopen: z.boolean(),
    isAfterHours: z.boolean(),
    status: z.string(),
    afterHoursRadius: TokenAmountSchema,
    lastKnownIndexPrice: TokenAmountSchema.nullable().optional(),
    markPriceBand: MarketPriceBandSchema.nullable().optional(),
    executionPriceBand: MarketPriceBandSchema.nullable().optional(),
    lastIndexExpiryTimestamp: z.number().nullable().optional(),
  });

// ---------------------------------------------------------------------------
// MarketSummary (matches ts/sdk MarketSummary)
// ---------------------------------------------------------------------------

export interface MarketSummary {
  symbol: Symbol;
  assetId: number;
  marketStatus: string;
  commodityStatus?: string;
  commodityMetadata?: MarketCommodityMetadata | null;
  units: MarketUnits;
  fees: MarketFees;
  openInterest: TokenAmount;
  openInterestCap: TokenAmount;
  leverageTiers: MarketLeverageTier[];
  fundingIntervalInSlots: number;
  fundingPeriodInSlots: number;
  fundingStartIntervalSlot: number;
  cumulativeFundingRate: number;
  maxLiquidationSize: TokenAmount;
  riskFactors: RiskFactors;
  isolatedOnly: boolean;
}

export const MarketSummarySchema: z.ZodType<MarketSummary> = z.object({
  symbol: zSymbol,
  assetId: z.number(),
  marketStatus: z.string(),
  commodityStatus: z.string().optional(),
  commodityMetadata: MarketCommodityMetadataSchema.nullable().optional(),
  units: MarketUnitsSchema,
  fees: MarketFeesSchema,
  openInterest: TokenAmountSchema,
  openInterestCap: TokenAmountSchema,
  leverageTiers: z.array(LeverageTierSchema),
  fundingIntervalInSlots: z.number(),
  fundingPeriodInSlots: z.number(),
  fundingStartIntervalSlot: z.number(),
  cumulativeFundingRate: z.number(),
  maxLiquidationSize: TokenAmountSchema,
  riskFactors: RiskFactorsSchema,
  isolatedOnly: z.boolean(),
});

// ---------------------------------------------------------------------------
// MarketsResponse (matches ts/sdk MarketsResponse)
// ---------------------------------------------------------------------------

export interface MarketsResponse {
  slot: number;
  markets: MarketSummary[];
}

export const MarketsResponseSchema: z.ZodType<MarketsResponse> = z.object({
  slot: z.number(),
  markets: z.array(MarketSummarySchema),
});
