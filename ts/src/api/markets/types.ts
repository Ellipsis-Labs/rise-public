import { TokenAmountSchema, type TokenAmount } from "@/primitives/TokenAmount";
import {
  MarketUnitsSchema,
  MarketFeesSchema,
  LeverageTierSchema,
  MarketCommodityMetadataSchema,
  RiskFactorsSchema,
  type MarketCommodityMetadata,
  type MarketUnits,
  type MarketFees,
  type MarketLeverageTier,
  type RiskFactors,
} from "@/types/market";
import z from "zod";

// ---------------------------------------------------------------------------
// Re-exports from core types (already defined in rise)
// ---------------------------------------------------------------------------

export {
  type MarketUnits,
  MarketUnitsSchema,
  type MarketFees,
  MarketFeesSchema,
  type MarketLeverageTier,
  LeverageTierSchema,
  type RiskFactors,
  RiskFactorsSchema,
  type MarketSummary,
  MarketSummarySchema,
  type MarketsResponse,
  MarketsResponseSchema,
} from "@/types/market";

// ---------------------------------------------------------------------------
// Price Data
// ---------------------------------------------------------------------------

export interface PriceData {
  price: number;
  slot: number;
}

export const PriceDataSchema: z.ZodType<PriceData> = z.object({
  price: z.number(),
  slot: z.number(),
});

export type OrderbookLevel = [number, number];

export const OrderbookLevelSchema: z.ZodType<OrderbookLevel> = z.tuple([
  z.number(),
  z.number(),
]);

export interface L2Orderbook {
  bids: OrderbookLevel[];
  asks: OrderbookLevel[];
  mid: number | null;
}

export const L2OrderbookSchema: z.ZodType<L2Orderbook> = z.object({
  bids: z.array(OrderbookLevelSchema),
  asks: z.array(OrderbookLevelSchema),
  mid: z.number().nullable(),
});

// ---------------------------------------------------------------------------
// Market View (detailed single-market response)
// ---------------------------------------------------------------------------

export interface MarketView {
  symbol: string;
  assetId: number;
  marketStatus: string;
  commodityStatus?: string;
  commodityMetadata?: MarketCommodityMetadata | null;
  marketKey: string;
  units: MarketUnits;
  fees: MarketFees;
  riskActionPriceValidityRules: number[][][];
  openInterest: TokenAmount;
  leverageTiers: MarketLeverageTier[];
  riskFactors: RiskFactors;
  spotPrice: PriceData | null;
  markPrice: PriceData | null;
  fundingIntervalSeconds: number;
  fundingPeriodSeconds: number;
  fundingStartIntervalTimestamp: number;
  cumulativeFundingRate: number;
  maxFundingRatePerInterval: number;
  currentFundingRatePercentage: number;
  annualizedFundingRatePercentage: number;
  l2Orderbook: L2Orderbook;
}

export const MarketViewSchema: z.ZodType<MarketView> = z.object({
  symbol: z.string(),
  assetId: z.number(),
  marketStatus: z.string(),
  commodityStatus: z.string().optional(),
  commodityMetadata: MarketCommodityMetadataSchema.nullable().optional(),
  marketKey: z.string(),
  units: MarketUnitsSchema,
  fees: MarketFeesSchema,
  riskActionPriceValidityRules: z.array(z.array(z.array(z.number()))),
  openInterest: TokenAmountSchema,
  leverageTiers: z.array(LeverageTierSchema),
  riskFactors: RiskFactorsSchema,
  spotPrice: PriceDataSchema.nullable(),
  markPrice: PriceDataSchema.nullable(),
  fundingIntervalSeconds: z.number(),
  fundingPeriodSeconds: z.number(),
  fundingStartIntervalTimestamp: z.number(),
  cumulativeFundingRate: z.number(),
  maxFundingRatePerInterval: z.number(),
  currentFundingRatePercentage: z.number(),
  annualizedFundingRatePercentage: z.number(),
  l2Orderbook: L2OrderbookSchema,
});

export interface MarketResponse {
  slot: number;
  market: MarketView;
}

export const MarketResponseSchema: z.ZodType<MarketResponse> = z.object({
  slot: z.number(),
  market: MarketViewSchema,
});

export type MarketCalendarStateView = "open" | "afterHours";

export const MarketCalendarStateViewSchema: z.ZodType<MarketCalendarStateView> =
  z.enum(["open", "afterHours"]);

export interface MarketCalendarHoursRange {
  start: string;
  end: string;
}

export const MarketCalendarHoursRangeSchema: z.ZodType<MarketCalendarHoursRange> =
  z.object({
    start: z.string(),
    end: z.string(),
  });

export interface MarketCalendarDaySchedule {
  activeHours: MarketCalendarHoursRange[];
  inactiveHours: MarketCalendarHoursRange[];
}

export const MarketCalendarDayScheduleSchema: z.ZodType<MarketCalendarDaySchedule> =
  z.object({
    activeHours: z.array(MarketCalendarHoursRangeSchema),
    inactiveHours: z.array(MarketCalendarHoursRangeSchema),
  });

export interface MarketCalendarView {
  weeklySchedule: Record<string, MarketCalendarDaySchedule>;
  dateOverrides: Record<string, MarketCalendarDaySchedule>;
}

export const MarketCalendarViewSchema: z.ZodType<MarketCalendarView> = z.object(
  {
    weeklySchedule: z.record(z.string(), MarketCalendarDayScheduleSchema),
    dateOverrides: z.record(z.string(), MarketCalendarDayScheduleSchema),
  }
);

export type MarketCalendarKind = string;

export const MarketCalendarKindSchema: z.ZodType<MarketCalendarKind> =
  z.string();

export interface MarketCalendarResponse {
  market: string;
  marketCalendarId: string;
  kind: MarketCalendarKind;
  description: string;
  calendarUri: string;
  contentSha256: string;
  loadedAt: string;
  rawJson: string;
  calendar?: MarketCalendarView;
}

export const MarketCalendarResponseSchema: z.ZodType<MarketCalendarResponse> =
  z.object({
    market: z.string(),
    marketCalendarId: z.string(),
    kind: MarketCalendarKindSchema,
    description: z.string(),
    calendarUri: z.string(),
    contentSha256: z.string(),
    loadedAt: z.string(),
    rawJson: z.string(),
    calendar: MarketCalendarViewSchema.optional(),
  });

export interface MarketCalendarRecord {
  marketCalendarId: string;
  kind: MarketCalendarKind;
  description: string;
  s3Path: string;
  calendarUri: string;
  contentSha256: string;
  loadedAt: string;
  updatedAt: string;
  rawJson: string;
  calendar?: MarketCalendarView;
}

export const MarketCalendarRecordSchema: z.ZodType<MarketCalendarRecord> =
  z.object({
    marketCalendarId: z.string(),
    kind: MarketCalendarKindSchema,
    description: z.string(),
    s3Path: z.string(),
    calendarUri: z.string(),
    contentSha256: z.string(),
    loadedAt: z.string(),
    updatedAt: z.string(),
    rawJson: z.string(),
    calendar: MarketCalendarViewSchema.optional(),
  });

export interface MarketCalendarSummary {
  marketCalendarId: string;
  kind: MarketCalendarKind;
  description: string;
  s3Path: string;
  calendarUri: string;
  contentSha256: string;
  createdAt: string;
  updatedAt: string;
}

export const MarketCalendarSummarySchema: z.ZodType<MarketCalendarSummary> =
  z.object({
    marketCalendarId: z.string(),
    kind: MarketCalendarKindSchema,
    description: z.string(),
    s3Path: z.string(),
    calendarUri: z.string(),
    contentSha256: z.string(),
    createdAt: z.string(),
    updatedAt: z.string(),
  });

export interface MarketCalendarListResponse {
  calendars: MarketCalendarSummary[];
}

export const MarketCalendarListResponseSchema: z.ZodType<MarketCalendarListResponse> =
  z.object({
    calendars: z.array(MarketCalendarSummarySchema),
  });

export type CommodityMarketStateView = MarketCalendarStateView;

export const CommodityMarketStateViewSchema: z.ZodType<CommodityMarketStateView> =
  MarketCalendarStateViewSchema;

export type CommodityMarketHoursRange = MarketCalendarHoursRange;

export const CommodityMarketHoursRangeSchema: z.ZodType<CommodityMarketHoursRange> =
  MarketCalendarHoursRangeSchema;

/**
 * A single named session within a {@link CommodityMarketDaySchedule}.
 *
 * `mode` is the serialized calendar mode. The API erases the underlying Rust
 * enum to a plain string, so this stays a `string` for forward compatibility;
 * known values are the SCREAMING_SNAKE_CASE variants of the futures calendar
 * (`"EXTERNAL"`, `"INTERNAL"`) and the equities calendar (`"EXTERNAL"`,
 * `"INTERNAL"`, `"CASH_SESSION"`, `"PRE_MARKET"`, `"POST_MARKET"`,
 * `"OVERNIGHT"`, `"FUTURES_SESSION"`).
 */
export interface CommodityMarketSessionRange {
  start: string;
  end: string;
  mode: string;
}

export const CommodityMarketSessionRangeSchema: z.ZodType<CommodityMarketSessionRange> =
  z.object({
    start: z.string(),
    end: z.string(),
    mode: z.string(),
  });

export interface CommodityMarketDaySchedule {
  sessions: CommodityMarketSessionRange[];
}

export const CommodityMarketDayScheduleSchema: z.ZodType<CommodityMarketDaySchedule> =
  z.object({
    sessions: z.array(CommodityMarketSessionRangeSchema),
  });

export interface CommodityMarketCalendarView {
  weeklySchedule: Record<string, CommodityMarketDaySchedule>;
  dateOverrides: Record<string, CommodityMarketDaySchedule>;
}

export const CommodityMarketCalendarViewSchema: z.ZodType<CommodityMarketCalendarView> =
  z.object({
    weeklySchedule: z.record(z.string(), CommodityMarketDayScheduleSchema),
    dateOverrides: z.record(z.string(), CommodityMarketDayScheduleSchema),
  });

export interface CommodityMarketCalendarResponse {
  market: string;
  loadedAt: string;
  rawJson: string;
  calendar: CommodityMarketCalendarView;
}

export const CommodityMarketCalendarResponseSchema: z.ZodType<CommodityMarketCalendarResponse> =
  z.object({
    market: z.string(),
    loadedAt: z.string(),
    rawJson: z.string(),
    calendar: CommodityMarketCalendarViewSchema,
  });

export interface NextMarketCalendarTransition {
  market: string;
  marketCalendarId: string;
  calendarUri: string;
  loadedAt: string;
  utcNextTransition?: string | null;
  nextMarketState?: MarketCalendarStateView | null;
  currentState: MarketCalendarStateView;
}

export const NextMarketCalendarTransitionSchema: z.ZodType<NextMarketCalendarTransition> =
  z.object({
    market: z.string(),
    marketCalendarId: z.string(),
    calendarUri: z.string(),
    loadedAt: z.string(),
    utcNextTransition: z.string().nullish(),
    nextMarketState: MarketCalendarStateViewSchema.nullish(),
    currentState: MarketCalendarStateViewSchema,
  });

export interface NextCommodityMarketTransition {
  market: string;
  loadedAt: string;
  utcNextTransition?: string | null;
  nextMarketState?: CommodityMarketStateView | null;
  currentState: CommodityMarketStateView;
}

export const NextCommodityMarketTransitionSchema: z.ZodType<NextCommodityMarketTransition> =
  z.object({
    market: z.string(),
    loadedAt: z.string(),
    utcNextTransition: z.string().nullish(),
    nextMarketState: CommodityMarketStateViewSchema.nullish(),
    currentState: CommodityMarketStateViewSchema,
  });

// ---------------------------------------------------------------------------
// Query parameter helpers
// ---------------------------------------------------------------------------

export interface PriceHistoryParams {
  start_time?: string;
  end_time?: string;
  limit?: number;
  timeframe?: string;
}

export interface MarketStatsHistoryParams {
  start_time?: string;
  end_time?: string;
  limit?: number;
  timeframe?: string;
}

// ---------------------------------------------------------------------------
// Price History
// ---------------------------------------------------------------------------
export interface PricePoint {
  mark_price: number;
  timestamp: string;
  sequence_number: number;
  exchange_spot_price: number | null;
  exchange_perp_price: number | null;
  best_bid: number | null;
  best_ask: number | null;
  last_trade: number | null;
  mid_spot_diff_ema_ticks: number | null;
  cumulative_funding_rate: number | null;
  settled_contribution: number | null;
}

export const PricePointSchema: z.ZodType<PricePoint> = z.object({
  mark_price: z.number(),
  timestamp: z.string(),
  sequence_number: z.number(),
  exchange_spot_price: z.number().nullable(),
  exchange_perp_price: z.number().nullable(),
  best_bid: z.number().nullable(),
  best_ask: z.number().nullable(),
  last_trade: z.number().nullable(),
  mid_spot_diff_ema_ticks: z.number().nullable(),
  cumulative_funding_rate: z.number().nullable(),
  settled_contribution: z.number().nullable(),
});

export interface PriceHistoryResponse {
  market_id: number;
  symbol: string;
  timeframe: string | null;
  prices: PricePoint[];
}

export const PriceHistoryResponseSchema: z.ZodType<PriceHistoryResponse> =
  z.object({
    market_id: z.number(),
    symbol: z.string(),
    timeframe: z.string().nullable(),
    prices: z.array(PricePointSchema),
  });

// ---------------------------------------------------------------------------
// Market Stats History
// ---------------------------------------------------------------------------
export interface MarketStatsPoint {
  timestamp: string;
  open_interest: number;
  total_maker_fees: number | null;
  total_taker_fees: number | null;
  mark_price: number;
  spot_price: number;
  slot: number;
}

export const MarketStatsPointSchema: z.ZodType<MarketStatsPoint> = z.object({
  timestamp: z.string(),
  open_interest: z.number(),
  total_maker_fees: z.number().nullable(),
  total_taker_fees: z.number().nullable(),
  mark_price: z.number(),
  spot_price: z.number(),
  slot: z.number(),
});

export interface MarketStatsHistoryResponse {
  market_id: number;
  symbol: string;
  timeframe: string | null;
  stats: MarketStatsPoint[];
}

export const MarketStatsHistoryResponseSchema: z.ZodType<MarketStatsHistoryResponse> =
  z.object({
    market_id: z.number(),
    symbol: z.string(),
    timeframe: z.string().nullable(),
    stats: z.array(MarketStatsPointSchema),
  });
