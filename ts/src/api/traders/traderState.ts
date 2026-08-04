import z from "zod";

import { TraderCapabilitiesSchema, type TraderCapabilities } from "./types";

const numericBigint = (field: string) =>
  z
    .union([z.bigint(), z.number().int(), z.string()])
    .transform((value, ctx) => {
      try {
        return BigInt(value);
      } catch {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: `${field} must be a bigint-compatible number`,
        });
        return z.NEVER;
      }
    });

export type TraderStateRowChangeKind = "updated" | "closed";
export type TraderStateSide = "bid" | "ask";
export type TraderStateStopLossOrderKind = "ioc" | "limit";
export type TraderActivityState =
  | "uninitialized"
  | "cold"
  | "active"
  | "reduceOnly"
  | "frozen";

export interface TraderStateCapabilities {
  flags: number;
  state: TraderActivityState;
  capabilities: TraderCapabilities;
}

export interface CooldownStatus {
  lastDepositSlot: number;
  cooldownPeriodInSlots: number;
}

export interface TraderStateTrigger {
  triggerPriceTicks: string;
  executionPriceTicks: string;
  side: TraderStateSide;
  kind: TraderStateStopLossOrderKind;
}

export interface TraderStateTakeProfitTrigger {
  takeProfitId: string;
  trigger: TraderStateTrigger;
  status: string;
}

export interface TraderStateStopLossTrigger {
  stopLossId: string;
  trigger: TraderStateTrigger;
  status: string;
}

export interface TraderStateConditionalTrigger extends TraderStateTrigger {
  attachedOrderSequenceNumber?: string | null;
  maxSizeLots: string;
  fillableSizeLots: string;
  filledSizeLots: string;
  usePercent: boolean;
  percent: number;
}

export interface TraderStateConditionalTakeProfitTrigger {
  conditionalTakeProfitId: string;
  trigger: TraderStateConditionalTrigger;
  status: string;
}

export interface TraderStateConditionalStopLossTrigger {
  conditionalStopLossId: string;
  trigger: TraderStateConditionalTrigger;
  status: string;
}

export interface TraderStatePositionRow {
  positionSequenceNumber: string;
  basePositionLots: string;
  basePositionUnits?: string;
  entryPriceTicks: string;
  entryPriceUsd?: string;
  virtualQuotePositionLots: string;
  unsettledFundingQuoteLots: string;
  accumulatedFundingQuoteLots: string;
  takeProfitTriggers: TraderStateTakeProfitTrigger[];
  stopLossTriggers: TraderStateStopLossTrigger[];
  conditionalTakeProfitTriggers: TraderStateConditionalTakeProfitTrigger[];
  conditionalStopLossTriggers: TraderStateConditionalStopLossTrigger[];
}

export interface TraderStatePositionSnapshot extends TraderStatePositionRow {
  symbol: string;
}

export interface TraderStateMarketLimitOrderRow {
  change?: TraderStateRowChangeKind;
  orderSequenceNumber: string;
  side: TraderStateSide;
  orderType: string;
  conditionalKind?: string | null;
  priceTicks: string;
  priceUsd: string;
  sizeRemainingLots: string;
  sizeRemainingUnits?: string;
  initialSizeLots: string;
  reduceOnly: boolean;
  isStopLoss?: boolean;
  isStopLossDirection?: boolean;
  isConditionalOrder?: boolean;
  status: string;
}

export interface TraderStateLimitOrderEvent {
  symbol: string;
  orders: TraderStateMarketLimitOrderRow[];
}

export interface TraderStatePositionDelta {
  symbol: string;
  change: TraderStateRowChangeKind;
  position?: TraderStatePositionRow;
}

export interface TraderStateTickRegion {
  startPriceUsd: string;
  endPriceUsd: string;
  densityLotsPerTick: string;
  totalSizeLots: string;
  filledSizeLots: string;
}

export interface TraderStateSplineRow {
  midPriceTicks: string;
  midPriceUsd: string;
  bidFilledAmountLots: string;
  askFilledAmountLots: string;
  bidRegions: TraderStateTickRegion[];
  askRegions: TraderStateTickRegion[];
}

export interface TraderStateSplineSnapshot extends TraderStateSplineRow {
  symbol: string;
}

export interface TraderStateSplineDelta {
  symbol: string;
  change: TraderStateRowChangeKind;
  spline?: TraderStateSplineRow;
}

export interface TraderStateTriggerRow {
  takeProfitTriggers: TraderStateTakeProfitTrigger[];
  stopLossTriggers: TraderStateStopLossTrigger[];
  conditionalTakeProfitTriggers: TraderStateConditionalTakeProfitTrigger[];
  conditionalStopLossTriggers: TraderStateConditionalStopLossTrigger[];
}

export interface TraderStateTriggerSnapshot extends TraderStateTriggerRow {
  symbol: string;
}

export interface TraderStateTriggerDelta {
  symbol: string;
  change: TraderStateRowChangeKind;
  triggers?: TraderStateTriggerRow;
}

export interface TraderStateTradeHistoryDelta {
  timestamp: number;
  slot: number;
  slotIndex: number;
  instructionIndex: number;
  eventIndex: number;
  market: string;
  instructionType: string;
  tradeType: "limit" | "market" | "liquidation" | "adl";
  fillId?: string | null;
  baseQtyBefore: string;
  baseQtyAfter: string;
  size: string;
  liquidity: "maker" | "taker";
  price: string;
  fee: string;
  realizedPnl: string;
  signature: string | null;
}

export interface TraderStateOrderHistoryDelta {
  timestamp: number;
  slot: number;
  slotIndex: number;
  instructionIndex: number;
  eventIndex: number;
  market: string;
  instructionType: string;
  orderType: string;
  status: string;
  size: string;
  price: string;
  filledSize: string;
}

export interface TraderStateSpotCollateral {
  assetIndex: number;
  symbol: string;
  /** Balance in the asset's native units (lamports for SOL), decimal integer string. */
  balance: string;
}

export interface TraderStateSubaccountSnapshot {
  subaccountIndex: number;
  sequence: number;
  /** Quote collateral balance. */
  collateral: string;
  spotCollaterals?: TraderStateSpotCollateral[];
  capabilities?: TraderStateCapabilities;
  cooldownStatus?: CooldownStatus;
  positions: TraderStatePositionSnapshot[];
  orders: TraderStateLimitOrderEvent[];
  splines: TraderStateSplineSnapshot[];
  triggers: TraderStateTriggerSnapshot[];
}

export interface TraderStateSubaccountDelta {
  subaccountIndex: number;
  sequence: number;
  /** Quote collateral balance. Carries the full current value, not a diff. */
  collateral: string;
  spotCollaterals?: TraderStateSpotCollateral[];
  capabilities?: TraderStateCapabilities;
  cooldownStatus?: CooldownStatus;
  positions: TraderStatePositionDelta[];
  orders: TraderStateLimitOrderEvent[];
  splines: TraderStateSplineDelta[];
  triggers: TraderStateTriggerDelta[];
  tradeHistory: TraderStateTradeHistoryDelta[];
  orderHistory: TraderStateOrderHistoryDelta[];
}

export interface TraderStateSnapshot {
  version: number;
  capabilities: TraderStateCapabilities;
  makerFeeOverrideMultiplier: number;
  takerFeeOverrideMultiplier: number;
  subaccounts: TraderStateSubaccountSnapshot[];
}

export interface TraderStateSnapshotResponse {
  authority: string;
  traderPdaIndex: number;
  slot: number;
  slotIndex: number;
  snapshot: TraderStateSnapshot;
}

export interface TraderStateRequest {
  traderPdaIndex?: number;
}

export interface TraderStateServerMessage {
  channel: "traderState";
  authority: string;
  traderPdaIndex: number;
  slot: bigint;
  messageType: "snapshot" | "delta";
  version?: number;
  capabilities?: TraderStateCapabilities;
  makerFeeOverrideMultiplier?: number;
  takerFeeOverrideMultiplier?: number;
  subaccounts?: TraderStateSubaccountSnapshot[];
  deltas?: TraderStateSubaccountDelta[];
}

const TraderStateTriggerSchema: z.ZodType<TraderStateTrigger> = z.object({
  triggerPriceTicks: z.string(),
  executionPriceTicks: z.string(),
  side: z.enum(["bid", "ask"]),
  kind: z.enum(["ioc", "limit"]),
});

const TraderStateTakeProfitTriggerSchema: z.ZodType<TraderStateTakeProfitTrigger> =
  z.object({
    takeProfitId: z.string(),
    trigger: TraderStateTriggerSchema,
    status: z.string(),
  });

const TraderStateStopLossTriggerSchema: z.ZodType<TraderStateStopLossTrigger> =
  z.object({
    stopLossId: z.string(),
    trigger: TraderStateTriggerSchema,
    status: z.string(),
  });

const TraderStateConditionalTriggerSchema: z.ZodType<TraderStateConditionalTrigger> =
  TraderStateTriggerSchema.and(
    z.object({
      attachedOrderSequenceNumber: z.string().nullable().optional(),
      maxSizeLots: z.string(),
      fillableSizeLots: z.string(),
      filledSizeLots: z.string(),
      usePercent: z.boolean(),
      percent: z.number(),
    })
  );

const TraderStateConditionalTakeProfitTriggerSchema: z.ZodType<TraderStateConditionalTakeProfitTrigger> =
  z.object({
    conditionalTakeProfitId: z.string(),
    trigger: TraderStateConditionalTriggerSchema,
    status: z.string(),
  });

const TraderStateConditionalStopLossTriggerSchema: z.ZodType<TraderStateConditionalStopLossTrigger> =
  z.object({
    conditionalStopLossId: z.string(),
    trigger: TraderStateConditionalTriggerSchema,
    status: z.string(),
  });

const TraderStateCapabilitiesSchema: z.ZodType<TraderStateCapabilities> =
  z.object({
    flags: z.number(),
    state: z.enum(["uninitialized", "cold", "active", "reduceOnly", "frozen"]),
    capabilities: TraderCapabilitiesSchema,
  });

const CooldownStatusSchema: z.ZodType<CooldownStatus> = z.object({
  lastDepositSlot: z.number(),
  cooldownPeriodInSlots: z.number(),
});

const TraderStatePositionRowSchema: z.ZodType<TraderStatePositionRow> =
  z.object({
    positionSequenceNumber: z.string(),
    basePositionLots: z.string(),
    basePositionUnits: z.string().optional(),
    entryPriceTicks: z.string(),
    entryPriceUsd: z.string().optional(),
    virtualQuotePositionLots: z.string(),
    unsettledFundingQuoteLots: z.string(),
    accumulatedFundingQuoteLots: z.string(),
    takeProfitTriggers: z.array(TraderStateTakeProfitTriggerSchema),
    stopLossTriggers: z.array(TraderStateStopLossTriggerSchema),
    conditionalTakeProfitTriggers: z.array(
      TraderStateConditionalTakeProfitTriggerSchema
    ),
    conditionalStopLossTriggers: z.array(
      TraderStateConditionalStopLossTriggerSchema
    ),
  });

const TraderStatePositionSnapshotSchema: z.ZodType<TraderStatePositionSnapshot> =
  z.object({
    symbol: z.string(),
    positionSequenceNumber: z.string(),
    basePositionLots: z.string(),
    basePositionUnits: z.string().optional(),
    entryPriceTicks: z.string(),
    entryPriceUsd: z.string().optional(),
    virtualQuotePositionLots: z.string(),
    unsettledFundingQuoteLots: z.string(),
    accumulatedFundingQuoteLots: z.string(),
    takeProfitTriggers: z.array(TraderStateTakeProfitTriggerSchema),
    stopLossTriggers: z.array(TraderStateStopLossTriggerSchema),
    conditionalTakeProfitTriggers: z.array(
      TraderStateConditionalTakeProfitTriggerSchema
    ),
    conditionalStopLossTriggers: z.array(
      TraderStateConditionalStopLossTriggerSchema
    ),
  });

const TraderStateMarketLimitOrderRowSchema: z.ZodType<TraderStateMarketLimitOrderRow> =
  z.object({
    change: z.enum(["updated", "closed"]).optional(),
    orderSequenceNumber: z.string(),
    side: z.enum(["bid", "ask"]),
    orderType: z.string(),
    conditionalKind: z.string().nullable().optional(),
    priceTicks: z.string(),
    priceUsd: z.string(),
    sizeRemainingLots: z.string(),
    sizeRemainingUnits: z.string().optional(),
    initialSizeLots: z.string(),
    reduceOnly: z.boolean(),
    isStopLoss: z.boolean().optional().default(false),
    isStopLossDirection: z.boolean().optional().default(false),
    isConditionalOrder: z.boolean().optional().default(false),
    status: z.string(),
  });

const TraderStateLimitOrderEventSchema: z.ZodType<TraderStateLimitOrderEvent> =
  z.object({
    symbol: z.string(),
    orders: z.array(TraderStateMarketLimitOrderRowSchema).default([]),
  });

const TraderStatePositionDeltaSchema: z.ZodType<TraderStatePositionDelta> =
  z.object({
    symbol: z.string(),
    change: z.enum(["updated", "closed"]),
    position: TraderStatePositionRowSchema.optional(),
  });

const TraderStateTickRegionSchema: z.ZodType<TraderStateTickRegion> = z.object({
  startPriceUsd: z.string().optional().default("0"),
  endPriceUsd: z.string().optional().default("0"),
  densityLotsPerTick: z.string(),
  totalSizeLots: z.string(),
  filledSizeLots: z.string(),
});

const TraderStateSplineRowSchema: z.ZodType<TraderStateSplineRow> = z.object({
  midPriceTicks: z.string(),
  midPriceUsd: z.string().optional().default("0"),
  bidFilledAmountLots: z.string(),
  askFilledAmountLots: z.string(),
  bidRegions: z.array(TraderStateTickRegionSchema).default([]),
  askRegions: z.array(TraderStateTickRegionSchema).default([]),
});

const TraderStateSplineSnapshotSchema: z.ZodType<TraderStateSplineSnapshot> =
  z.object({
    symbol: z.string(),
    midPriceTicks: z.string(),
    midPriceUsd: z.string().optional().default("0"),
    bidFilledAmountLots: z.string(),
    askFilledAmountLots: z.string(),
    bidRegions: z.array(TraderStateTickRegionSchema).default([]),
    askRegions: z.array(TraderStateTickRegionSchema).default([]),
  });

const TraderStateSplineDeltaSchema: z.ZodType<TraderStateSplineDelta> =
  z.object({
    symbol: z.string(),
    change: z.enum(["updated", "closed"]),
    spline: TraderStateSplineRowSchema.optional(),
  });

const TraderStateTriggerRowSchema: z.ZodType<TraderStateTriggerRow> = z.object({
  takeProfitTriggers: z.array(TraderStateTakeProfitTriggerSchema).default([]),
  stopLossTriggers: z.array(TraderStateStopLossTriggerSchema).default([]),
  conditionalTakeProfitTriggers: z
    .array(TraderStateConditionalTakeProfitTriggerSchema)
    .default([]),
  conditionalStopLossTriggers: z
    .array(TraderStateConditionalStopLossTriggerSchema)
    .default([]),
});

const TraderStateTriggerSnapshotSchema: z.ZodType<TraderStateTriggerSnapshot> =
  z.object({
    symbol: z.string(),
    takeProfitTriggers: z.array(TraderStateTakeProfitTriggerSchema).default([]),
    stopLossTriggers: z.array(TraderStateStopLossTriggerSchema).default([]),
    conditionalTakeProfitTriggers: z
      .array(TraderStateConditionalTakeProfitTriggerSchema)
      .default([]),
    conditionalStopLossTriggers: z
      .array(TraderStateConditionalStopLossTriggerSchema)
      .default([]),
  });

const TraderStateTriggerDeltaSchema: z.ZodType<TraderStateTriggerDelta> =
  z.object({
    symbol: z.string(),
    change: z.enum(["updated", "closed"]),
    triggers: TraderStateTriggerRowSchema.optional(),
  });

const TraderStateTradeHistoryDeltaSchema: z.ZodType<TraderStateTradeHistoryDelta> =
  z.object({
    timestamp: z.number(),
    slot: z.number(),
    slotIndex: z.number(),
    instructionIndex: z.number(),
    eventIndex: z.number(),
    market: z.string(),
    instructionType: z.string(),
    tradeType: z.enum(["limit", "market", "liquidation", "adl"]),
    fillId: z.string().nullable().optional().default(null),
    baseQtyBefore: z.string(),
    baseQtyAfter: z.string(),
    size: z.string(),
    liquidity: z.enum(["maker", "taker"]),
    price: z.string(),
    fee: z.string(),
    realizedPnl: z.string(),
    signature: z.string().nullable().optional().default(null),
  });

const TraderStateOrderHistoryDeltaSchema: z.ZodType<TraderStateOrderHistoryDelta> =
  z.object({
    timestamp: z.number(),
    slot: z.number(),
    slotIndex: z.number(),
    instructionIndex: z.number(),
    eventIndex: z.number(),
    market: z.string(),
    instructionType: z.string(),
    orderType: z.string(),
    status: z.string(),
    size: z.string(),
    price: z.string(),
    filledSize: z.string(),
  });

const TraderStateSpotCollateralSchema: z.ZodType<TraderStateSpotCollateral> =
  z.object({
    assetIndex: z.number(),
    symbol: z.string(),
    balance: z.string(),
  });

const TraderStateSubaccountSnapshotSchema: z.ZodType<TraderStateSubaccountSnapshot> =
  z.object({
    subaccountIndex: z.number(),
    sequence: z.number(),
    collateral: z.string(),
    spotCollaterals: z.array(TraderStateSpotCollateralSchema).default([]),
    capabilities: TraderStateCapabilitiesSchema.optional(),
    cooldownStatus: CooldownStatusSchema.optional(),
    positions: z.array(TraderStatePositionSnapshotSchema).default([]),
    orders: z.array(TraderStateLimitOrderEventSchema).default([]),
    splines: z.array(TraderStateSplineSnapshotSchema).default([]),
    triggers: z.array(TraderStateTriggerSnapshotSchema).default([]),
  });

const TraderStateSubaccountDeltaSchema: z.ZodType<TraderStateSubaccountDelta> =
  z.object({
    subaccountIndex: z.number(),
    sequence: z.number(),
    collateral: z.string(),
    spotCollaterals: z.array(TraderStateSpotCollateralSchema).default([]),
    capabilities: TraderStateCapabilitiesSchema.optional(),
    cooldownStatus: CooldownStatusSchema.optional(),
    positions: z.array(TraderStatePositionDeltaSchema).default([]),
    orders: z.array(TraderStateLimitOrderEventSchema).default([]),
    splines: z.array(TraderStateSplineDeltaSchema).default([]),
    triggers: z.array(TraderStateTriggerDeltaSchema).default([]),
    tradeHistory: z.array(TraderStateTradeHistoryDeltaSchema).default([]),
    orderHistory: z.array(TraderStateOrderHistoryDeltaSchema).default([]),
  });

export const TraderStateSnapshotSchema: z.ZodType<TraderStateSnapshot> =
  z.object({
    version: z.number(),
    capabilities: TraderStateCapabilitiesSchema,
    makerFeeOverrideMultiplier: z.number(),
    takerFeeOverrideMultiplier: z.number(),
    subaccounts: z.array(TraderStateSubaccountSnapshotSchema),
  });

export const TraderStateSnapshotResponseSchema: z.ZodType<TraderStateSnapshotResponse> =
  z.object({
    authority: z.string(),
    traderPdaIndex: z.number(),
    slot: z.number(),
    slotIndex: z.number(),
    snapshot: TraderStateSnapshotSchema,
  });

const TraderStateServerMessageObject = z.object({
  channel: z.literal("traderState"),
  authority: z.string(),
  traderPdaIndex: z.number(),
  slot: numericBigint("slot"),
  messageType: z.enum(["snapshot", "delta"]),
  version: z.number().optional(),
  capabilities: TraderStateCapabilitiesSchema.optional(),
  makerFeeOverrideMultiplier: z.number().optional(),
  takerFeeOverrideMultiplier: z.number().optional(),
  subaccounts: z.array(TraderStateSubaccountSnapshotSchema).optional(),
  deltas: z.array(TraderStateSubaccountDeltaSchema).optional(),
});

export const TraderStateServerMessageSchema: z.ZodType<TraderStateServerMessage> =
  TraderStateServerMessageObject.superRefine((value, ctx) => {
    if (value.messageType === "snapshot") {
      if (value.version === undefined) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "snapshot payloads must include version",
          path: ["version"],
        });
      }
      if (value.capabilities === undefined) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "snapshot payloads must include capabilities",
          path: ["capabilities"],
        });
      }
      if (value.makerFeeOverrideMultiplier === undefined) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "snapshot payloads must include makerFeeOverrideMultiplier",
          path: ["makerFeeOverrideMultiplier"],
        });
      }
      if (value.takerFeeOverrideMultiplier === undefined) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "snapshot payloads must include takerFeeOverrideMultiplier",
          path: ["takerFeeOverrideMultiplier"],
        });
      }
      if (value.subaccounts === undefined) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          message: "snapshot payloads must include subaccounts",
          path: ["subaccounts"],
        });
      }
    } else if (value.deltas === undefined) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: "delta payloads must include deltas",
        path: ["deltas"],
      });
    }
  });

export {
  CooldownStatusSchema,
  TraderStateCapabilitiesSchema,
  TraderStateSubaccountDeltaSchema,
  TraderStateSubaccountSnapshotSchema,
};
