import z from "zod";
import type { TraderView } from "@/types/trader";
import { TraderViewSchema } from "@/types/trader";
import {
  createEscrowActionSchema,
  type EscrowAction,
} from "@/api/utils/escrowAction";

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

/**
 * PnL history request. When `resolution` is omitted the server clamps the
 * window to the account's creation and picks the finest resolution that fits
 * `limit` points.
 */
export interface PnlValuesRequest extends Omit<
  HistoricalValuesRequest,
  "resolution"
> {
  resolution?: string;
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

export type EscrowActionView = EscrowAction<number>;

export const EscrowActionViewSchema: z.ZodType<EscrowActionView> =
  createEscrowActionSchema(z.number());

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
