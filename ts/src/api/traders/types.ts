import z from "zod";
import type { TraderView } from "@/types/trader";
import { TraderViewSchema } from "@/types/trader";

// ---------------------------------------------------------------------------
// Re-exports from core types (already defined in rise)
// ---------------------------------------------------------------------------

export {
  RiskState,
  RiskTier,
  type Position,
  PositionSchema,
  type LimitOrder,
  LimitOrderSchema,
  type CapabilityAccess,
  type TraderCapabilities,
  TraderCapabilitiesSchema,
  type TraderView,
  TraderViewSchema,
} from "@/types/trader";

// ---------------------------------------------------------------------------
// Trader Capabilities Metadata
// ---------------------------------------------------------------------------

export interface TraderCapabilityDescriptor {
  key: string;
  displayName: string;
  description: string;
  deprecated?: boolean;
}

export const TraderCapabilityDescriptorSchema: z.ZodType<TraderCapabilityDescriptor> =
  z.object({
    key: z.string(),
    displayName: z.string(),
    description: z.string(),
    deprecated: z.boolean().optional().default(false),
  });

export interface TraderCapabilitiesMetadata {
  capabilities: TraderCapabilityDescriptor[];
}

export const TraderCapabilitiesMetadataSchema: z.ZodType<TraderCapabilitiesMetadata> =
  z.object({
    capabilities: z.array(TraderCapabilityDescriptorSchema),
  });

// ---------------------------------------------------------------------------
// Active Traders
// ---------------------------------------------------------------------------

export interface ActiveTraderView {
  slot: number;
  traders: TraderView[];
}

export const ActiveTraderViewSchema: z.ZodType<ActiveTraderView> = z.object({
  slot: z.number(),
  traders: z.array(TraderViewSchema),
});

export interface TraderStateResponse {
  slot: number;
  slotIndex: number;
  authority: string;
  pdaIndex: number;
  traders: TraderView[];
}

export const TraderStateResponseSchema: z.ZodType<TraderStateResponse> =
  z.object({
    slot: z.number(),
    slotIndex: z.number(),
    authority: z.string(),
    pdaIndex: z.number(),
    traders: z.array(TraderViewSchema),
  });

// ---------------------------------------------------------------------------
// Historical Values
// ---------------------------------------------------------------------------

export interface HistoricalValuesRequest {
  resolution: string;
  startTime?: number;
  endTime?: number;
  limit?: number;
  includeEarliest?: boolean;
  includeLatest?: boolean;
}

export type TimeWeightedReturnsResolution =
  | "5m"
  | "15m"
  | "1h"
  | "4h"
  | "1d"
  | "1w"
  | "1M";

export interface TimeWeightedReturnsRequest extends Pick<
  HistoricalValuesRequest,
  "startTime" | "endTime" | "limit"
> {
  resolution: TimeWeightedReturnsResolution;
}

export interface MarketPositionSnapshot {
  baseLots: string;
  quoteLots: string;
  positionValue: string;
  initialMargin: string;
}

export const MarketPositionSnapshotSchema: z.ZodType<MarketPositionSnapshot> =
  z.object({
    baseLots: z.string(),
    quoteLots: z.string(),
    positionValue: z.string(),
    initialMargin: z.string(),
  });

export interface SpotCollateralValue {
  assetIndex: number;
  balance: number;
  notional: number;
  discounted: number;
}

export const SpotCollateralValueSchema: z.ZodType<SpotCollateralValue> =
  z.object({
    assetIndex: z.number(),
    balance: z.number(),
    notional: z.number(),
    discounted: z.number(),
  });

export interface PortfolioValueDataPoint {
  timestamp: number;
  startTime: number;
  endTime: number;
  value: number;
  positions?: Record<string, MarketPositionSnapshot> | null;
  spotCollaterals?: Record<string, SpotCollateralValue> | null;
}

export const PortfolioValueDataPointSchema: z.ZodType<PortfolioValueDataPoint> =
  z.object({
    timestamp: z.number(),
    startTime: z.number(),
    endTime: z.number(),
    value: z.number(),
    positions: z
      .record(z.string(), MarketPositionSnapshotSchema)
      .nullable()
      .optional(),
    spotCollaterals: z
      .record(z.string(), SpotCollateralValueSchema)
      .nullable()
      .optional(),
  });

export interface TimeWeightedReturnPoint {
  timestamp: string;
  startTime: string;
  endTime: string;
  periodReturn: number | null;
  cumulativeReturn: number | null;
  netExternalFlow: number | null;
  qualityFlags: string[];
}

const jsonSafeIntegerSchema = z
  .union([z.string(), z.number().int()])
  .transform(String);
const unixTimestampSchema = jsonSafeIntegerSchema;

export const TimeWeightedReturnPointSchema: z.ZodType<TimeWeightedReturnPoint> =
  z.object({
    timestamp: unixTimestampSchema,
    startTime: unixTimestampSchema,
    endTime: unixTimestampSchema,
    periodReturn: z.number().nullable(),
    cumulativeReturn: z.number().nullable(),
    netExternalFlow: z.number().nullable(),
    qualityFlags: z.array(z.string()),
  });

export interface TimeWeightedReturnScope {
  traderPubkey: string;
  userId: string;
  traderPdaIndex: number;
}

export const TimeWeightedReturnScopeSchema: z.ZodType<TimeWeightedReturnScope> =
  z.object({
    traderPubkey: z.string(),
    userId: jsonSafeIntegerSchema,
    traderPdaIndex: z.number().int().nonnegative(),
  });

export type TimeWeightedReturnCompleteness =
  | "complete"
  | "partial"
  | "unavailable";

export const TimeWeightedReturnCompletenessSchema: z.ZodType<TimeWeightedReturnCompleteness> =
  z.enum(["complete", "partial", "unavailable"]);

export interface TimeWeightedReturnWindow {
  requestedStartTime: string;
  requestedEndTime: string;
  calculationEndTime: string;
  dataStartTime: string | null;
  dataEndTime: string | null;
  truncatedByLimit: boolean;
}

export const TimeWeightedReturnWindowSchema: z.ZodType<TimeWeightedReturnWindow> =
  z.object({
    requestedStartTime: unixTimestampSchema,
    requestedEndTime: unixTimestampSchema,
    calculationEndTime: unixTimestampSchema,
    dataStartTime: unixTimestampSchema.nullable(),
    dataEndTime: unixTimestampSchema.nullable(),
    truncatedByLimit: z.boolean(),
  });

export interface TimeWeightedReturnQuality {
  coverageStart: string | null;
  coverageEnd: string | null;
  requestedStartCovered: boolean;
  containsGaps: boolean;
  resetCount: string;
  refreshedAt: string | null;
  completeness: TimeWeightedReturnCompleteness;
}

export const TimeWeightedReturnQualitySchema: z.ZodType<TimeWeightedReturnQuality> =
  z.object({
    coverageStart: unixTimestampSchema.nullable(),
    coverageEnd: unixTimestampSchema.nullable(),
    requestedStartCovered: z.boolean(),
    containsGaps: z.boolean(),
    resetCount: jsonSafeIntegerSchema,
    refreshedAt: unixTimestampSchema.nullable(),
    completeness: TimeWeightedReturnCompletenessSchema,
  });

export interface TimeWeightedReturnsResponse {
  method: "time_weighted";
  scope: TimeWeightedReturnScope;
  valuationIntervalSeconds: number;
  exactFlowBoundaryValuations: boolean;
  totalReturn: number | null;
  returnStartTime: string | null;
  equityDefinition: string;
  window: TimeWeightedReturnWindow;
  points: TimeWeightedReturnPoint[];
  quality: TimeWeightedReturnQuality;
}

export const TimeWeightedReturnsResponseSchema: z.ZodType<TimeWeightedReturnsResponse> =
  z.object({
    method: z.literal("time_weighted"),
    scope: TimeWeightedReturnScopeSchema,
    valuationIntervalSeconds: z.number().int().positive(),
    exactFlowBoundaryValuations: z.boolean(),
    totalReturn: z.number().nullable(),
    returnStartTime: unixTimestampSchema.nullable(),
    equityDefinition: z.string(),
    window: TimeWeightedReturnWindowSchema,
    points: z.array(TimeWeightedReturnPointSchema),
    quality: TimeWeightedReturnQualitySchema,
  });

export interface PnlDataPoint {
  timestamp: number;
  startTime: number;
  endTime: number;
  cumulativePnl: number;
  unrealizedPnl: number;
  cumulativeFundingPayment: number;
  cumulativeTakerFee: number;
  cumulativeMakerFee: number;
}

export const PnlDataPointSchema: z.ZodType<PnlDataPoint> = z.object({
  timestamp: z.number(),
  startTime: z.number(),
  endTime: z.number(),
  cumulativePnl: z.number(),
  unrealizedPnl: z.number(),
  cumulativeFundingPayment: z.number(),
  cumulativeTakerFee: z.number(),
  cumulativeMakerFee: z.number(),
});

// ---------------------------------------------------------------------------
// Trader Market PnL
// ---------------------------------------------------------------------------

export interface TraderMarketPnLQueryParams {
  resolution: string;
  startTime?: number;
  endTime?: number;
  limit?: number;
  symbols?: string[];
}

export interface TraderMarketPnLPoint {
  timestamp: number;
  startTime: number;
  endTime: number;
  realizedPnl: number;
  cumulativePnl: number;
  cumulativeFundingPayment: number;
  totalTakerFee: number;
  cumulativeTakerFee: number;
  unrealizedPnl: number;
  baseLots: number;
  virtualQuoteLots: number;
  markPrice: number;
}

export const TraderMarketPnLPointSchema: z.ZodType<TraderMarketPnLPoint> =
  z.object({
    timestamp: z.number(),
    startTime: z.number(),
    endTime: z.number(),
    realizedPnl: z.number(),
    cumulativePnl: z.number(),
    cumulativeFundingPayment: z.number(),
    totalTakerFee: z.number(),
    cumulativeTakerFee: z.number(),
    unrealizedPnl: z.number(),
    baseLots: z.number(),
    virtualQuoteLots: z.number(),
    markPrice: z.number(),
  });

export interface TraderMarketPnLSeries {
  marketId: number;
  symbol: string;
  tickSize: number;
  baseDecimals: number;
  points: TraderMarketPnLPoint[];
}

export const TraderMarketPnLSeriesSchema: z.ZodType<TraderMarketPnLSeries> =
  z.object({
    marketId: z.number(),
    symbol: z.string(),
    tickSize: z.number(),
    baseDecimals: z.number(),
    points: z.array(TraderMarketPnLPointSchema),
  });

// ---------------------------------------------------------------------------
// Escrow: pending requests
// ---------------------------------------------------------------------------

export type EscrowActionView =
  | { kind: "noop" }
  | { kind: "cash"; amount: number };

export const EscrowActionViewSchema: z.ZodType<EscrowActionView> =
  z.discriminatedUnion("kind", [
    z.object({ kind: z.literal("noop") }),
    z.object({ kind: z.literal("cash"), amount: z.number() }),
  ]);

export interface PendingEscrowRequestView {
  sequenceNumber: number;
  senderAuthority: string;
  senderPdaIndex: number;
  senderSubaccountIndex: number;
  receiverPdaIndex: number;
  receiverSubaccountIndex: number;
  expirationOffset: number | null;
  initialSlot: number;
  actions: EscrowActionView[];
}

export const PendingEscrowRequestViewSchema: z.ZodType<PendingEscrowRequestView> =
  z.object({
    sequenceNumber: z.number(),
    senderAuthority: z.string(),
    senderPdaIndex: z.number(),
    senderSubaccountIndex: z.number(),
    receiverPdaIndex: z.number(),
    receiverSubaccountIndex: z.number(),
    expirationOffset: z.number().nullable(),
    initialSlot: z.number(),
    actions: z.array(EscrowActionViewSchema),
  });
