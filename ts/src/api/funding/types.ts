import { TokenAmountSchema, type TokenAmount } from "@/primitives/TokenAmount";
import z from "zod";

const toNumber = (value: unknown, fieldName: string): number => {
  if (typeof value === "number") return value;
  if (typeof value === "string") {
    if (value.trim().length > 0) {
      const parsed = Number(value);
      if (!Number.isNaN(parsed)) return parsed;
    }
    const dateParsed = Date.parse(value);
    if (!Number.isNaN(dateParsed)) return dateParsed;
  }

  throw new Error(`Invalid numeric value for ${fieldName}`);
};

const requireField = <T>(value: T | null | undefined, fieldName: string): T => {
  if (value === undefined || value === null)
    throw new Error(`Missing field ${fieldName} in funding response`);
  return value;
};

export interface FundingHourlyRequest {
  symbol?: string;
  limit?: number;
  cursor?: string;
  traderPdaIndex?: number;
}

export interface TraderFundingHistoryRequest {
  pdaIndex?: number;
  symbol?: string;
  startTime?: number;
  endTime?: number;
  limit?: number;
  cursor?: string;
  resolution?: string;
}

export interface FundingRateHistoryRequest {
  startTime?: number;
  endTime?: number;
  limit?: number;
}

export interface FundingOverviewRequest {
  startTime?: number;
  endTime?: number;
  perMarketLimit?: number;
}

export interface FundingHourlyEvent {
  timestamp: number;
  symbol: string;
  fundingPayment: string;
  fundingRatePercentage: string;
  positionSize: string;
  positionSide: string;
}

const RawFundingHourlyEventSchema = z
  .object({
    timestamp: z.union([z.number(), z.string()]),
    symbol: z.string(),
    fundingPayment: z.string(),
    fundingRatePercentage: z.string(),
    positionSize: z.string(),
    positionSide: z.string(),
  })
  .loose();

export const FundingHourlyEventSchema: z.ZodType<FundingHourlyEvent> =
  RawFundingHourlyEventSchema.transform((raw) => ({
    timestamp: toNumber(raw.timestamp, "fundingHourly.events.timestamp"),
    symbol: requireField(raw.symbol, "fundingHourly.events.symbol"),
    fundingPayment: requireField(
      raw.fundingPayment,
      "fundingHourly.events.fundingPayment"
    ),
    fundingRatePercentage: requireField(
      raw.fundingRatePercentage,
      "fundingHourly.events.fundingRatePercentage"
    ),
    positionSize: requireField(
      raw.positionSize,
      "fundingHourly.events.positionSize"
    ),
    positionSide: requireField(
      raw.positionSide,
      "fundingHourly.events.positionSide"
    ),
  }));

export interface FundingHourlyHistoryResponse {
  events: FundingHourlyEvent[];
  prevCursor: string | null;
  nextCursor: string | null;
  hasMore: boolean;
}

export type TraderFundingHistoryResponse = FundingHourlyHistoryResponse;

const RawFundingHourlyHistoryResponseSchema = z
  .object({
    events: z.array(FundingHourlyEventSchema),
    prevCursor: z.string().nullable().optional(),
    nextCursor: z.string().nullable().optional(),
    hasMore: z.boolean(),
  })
  .loose();

export const FundingHourlyHistoryResponseSchema: z.ZodType<FundingHourlyHistoryResponse> =
  RawFundingHourlyHistoryResponseSchema.transform((raw) => ({
    events: raw.events ?? [],
    prevCursor: raw.prevCursor ?? null,
    nextCursor: raw.nextCursor ?? null,
    hasMore: requireField(raw.hasMore, "fundingHourly.hasMore"),
  }));

export const TraderFundingHistoryResponseSchema: z.ZodType<TraderFundingHistoryResponse> =
  FundingHourlyHistoryResponseSchema;

export interface FundingRatePoint {
  timestamp: number;
  fundingRatePercentage: string;
}

const RawFundingRatePointSchema = z
  .object({
    timestamp: z.union([z.number(), z.string()]),
    fundingRatePercentage: z.string(),
  })
  .loose();

export const FundingRatePointSchema: z.ZodType<FundingRatePoint> =
  RawFundingRatePointSchema.transform((raw) => ({
    timestamp: toNumber(raw.timestamp, "fundingRates.rates.timestamp"),
    fundingRatePercentage: requireField(
      raw.fundingRatePercentage,
      "fundingRates.rates.fundingRatePercentage"
    ),
  }));

export interface FundingRateHistoryResponse {
  marketId: number;
  symbol: string;
  rates: FundingRatePoint[];
}

const RawFundingRateHistoryResponseSchema = z
  .object({
    marketId: z.union([z.number(), z.string()]),
    symbol: z.string(),
    rates: z.array(FundingRatePointSchema),
  })
  .loose();

export const FundingRateHistoryResponseSchema: z.ZodType<FundingRateHistoryResponse> =
  RawFundingRateHistoryResponseSchema.transform((raw) => ({
    marketId: toNumber(raw.marketId, "fundingRates.marketId"),
    symbol: requireField(raw.symbol, "fundingRates.symbol"),
    rates: raw.rates ?? [],
  }));

export interface FundingOverviewPoint {
  timestamp: number;
  fundingAmountPerUnit: string;
  markPrice: string;
  fundingRate: string;
}

const RawFundingOverviewPointSchema = z
  .object({
    timestamp: z.union([z.number(), z.string()]),
    fundingAmountPerUnit: z.string(),
    markPrice: z.string(),
    fundingRate: z.string(),
  })
  .loose();

export const FundingOverviewPointSchema: z.ZodType<FundingOverviewPoint> =
  RawFundingOverviewPointSchema.transform((raw) => ({
    timestamp: toNumber(raw.timestamp, "fundingOverview.points.timestamp"),
    fundingAmountPerUnit: requireField(
      raw.fundingAmountPerUnit,
      "fundingOverview.points.fundingAmountPerUnit"
    ),
    markPrice: requireField(raw.markPrice, "fundingOverview.points.markPrice"),
    fundingRate: requireField(
      raw.fundingRate,
      "fundingOverview.points.fundingRate"
    ),
  }));

export interface FundingOverviewSeries {
  marketId: number;
  symbol: string;
  points: FundingOverviewPoint[];
}

const RawFundingOverviewSeriesSchema = z
  .object({
    marketId: z.union([z.number(), z.string()]),
    symbol: z.string(),
    points: z.array(FundingOverviewPointSchema),
  })
  .loose();

export const FundingOverviewSeriesSchema: z.ZodType<FundingOverviewSeries> =
  RawFundingOverviewSeriesSchema.transform((raw) => ({
    marketId: toNumber(raw.marketId, "fundingOverview.series.marketId"),
    symbol: requireField(raw.symbol, "fundingOverview.series.symbol"),
    points: raw.points ?? [],
  }));

export interface FundingOverviewResponse {
  series: FundingOverviewSeries[];
}

const RawFundingOverviewResponseSchema = z
  .object({
    series: z.array(FundingOverviewSeriesSchema),
  })
  .loose();

export const FundingOverviewResponseSchema: z.ZodType<FundingOverviewResponse> =
  RawFundingOverviewResponseSchema.transform((raw) => ({
    series: raw.series ?? [],
  }));

// ---------------------------------------------------------------------------
// Global fee view
// ---------------------------------------------------------------------------
export interface GlobalFeeView {
  slot: number;
  quoteDecimals: number;
  totalQuoteFees: TokenAmount;
  unclaimedQuoteFees: TokenAmount;
  globalVault: string;
  globalVaultBalance: TokenAmount;
}

export const GlobalFeeViewSchema: z.ZodType<GlobalFeeView> = z.object({
  slot: z.number(),
  quoteDecimals: z.number(),
  totalQuoteFees: TokenAmountSchema,
  unclaimedQuoteFees: TokenAmountSchema,
  globalVault: z.string(),
  globalVaultBalance: TokenAmountSchema,
});
