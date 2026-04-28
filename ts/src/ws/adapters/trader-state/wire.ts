import z from "zod";
import {
  TraderStateServerMessageSchema,
  TraderStateSnapshotResponseSchema,
  TraderStateCapabilitiesSchema,
  TraderStateSubaccountDeltaSchema,
  TraderStateSubaccountSnapshotSchema,
  type CooldownStatus,
  type TraderActivityState,
  type TraderStateCapabilities,
  type TraderStateConditionalStopLossTrigger,
  type TraderStateConditionalTakeProfitTrigger,
  type TraderStateMarketLimitOrderRow,
  type TraderStateOrderHistoryDelta,
  type TraderStatePositionDelta,
  type TraderStatePositionRow,
  type TraderStatePositionSnapshot,
  type TraderStateRequest,
  type TraderStateRowChangeKind,
  type TraderStateServerMessage,
  type TraderStateSide,
  type TraderStateSnapshot,
  type TraderStateSnapshotResponse,
  type TraderStateSplineDelta,
  type TraderStateSplineRow,
  type TraderStateSplineSnapshot,
  type TraderStateStopLossOrderKind,
  type TraderStateStopLossTrigger,
  type TraderStateSubaccountDelta,
  type TraderStateSubaccountSnapshot,
  type TraderStateTakeProfitTrigger,
  type TraderStateTradeHistoryDelta,
  type TraderStateTrigger,
  type TraderStateTriggerDelta,
  type TraderStateTriggerRow,
  type TraderStateTriggerSnapshot,
} from "@/api/traders/traderState";
import { numericBigint } from "@/ws/numericSchemas";

export type {
  CooldownStatus,
  TraderActivityState,
  TraderStateCapabilities,
  TraderStateConditionalStopLossTrigger,
  TraderStateConditionalTakeProfitTrigger,
  TraderStateMarketLimitOrderRow,
  TraderStateOrderHistoryDelta,
  TraderStatePositionDelta,
  TraderStatePositionRow,
  TraderStatePositionSnapshot,
  TraderStateRequest,
  TraderStateRowChangeKind,
  TraderStateServerMessage,
  TraderStateSide,
  TraderStateSnapshot,
  TraderStateSnapshotResponse,
  TraderStateSplineDelta,
  TraderStateSplineRow,
  TraderStateSplineSnapshot,
  TraderStateStopLossOrderKind,
  TraderStateStopLossTrigger,
  TraderStateSubaccountDelta,
  TraderStateSubaccountSnapshot,
  TraderStateTakeProfitTrigger,
  TraderStateTradeHistoryDelta,
  TraderStateTrigger,
  TraderStateTriggerDelta,
  TraderStateTriggerRow,
  TraderStateTriggerSnapshot,
};

export { TraderStateServerMessageSchema, TraderStateSnapshotResponseSchema };

export interface TraderSnapshotMessage {
  authority: string;
  traderPdaIndex: number;
  slot: number | bigint;
  messageType: "snapshot";
  version?: number;
  capabilities?: TraderStateCapabilities;
  makerFeeOverrideMultiplier?: number;
  takerFeeOverrideMultiplier?: number;
  subaccounts?: TraderStateSubaccountSnapshot[];
}

export interface TraderStateUpdate {
  authority: string;
  traderPdaIndex: number;
  slot: bigint;
  messageType: "snapshot" | "delta";
  version?: number;
  capabilities?: TraderStateCapabilities;
  makerFeeOverrideMultiplier?: number;
  takerFeeOverrideMultiplier?: number;
  subaccounts: TraderStateSubaccountSnapshot[];
  deltas: TraderStateSubaccountDelta[];
}

const TraderStateUpdateObjectSchema: z.ZodType<TraderStateUpdate> = z.object({
  authority: z.string(),
  traderPdaIndex: z.number(),
  slot: numericBigint("slot"),
  messageType: z.enum(["snapshot", "delta"]),
  version: z.number().optional(),
  capabilities: TraderStateCapabilitiesSchema.optional(),
  makerFeeOverrideMultiplier: z.number().optional(),
  takerFeeOverrideMultiplier: z.number().optional(),
  subaccounts: z.array(TraderStateSubaccountSnapshotSchema).default([]),
  deltas: z.array(TraderStateSubaccountDeltaSchema).default([]),
});

export const TraderStateServerMessageToUpdateSchema: z.ZodType<TraderStateUpdate> =
  TraderStateServerMessageSchema.transform((message) => ({
    authority: message.authority,
    traderPdaIndex: message.traderPdaIndex,
    slot: message.slot,
    messageType: message.messageType,
    version: message.version,
    capabilities: message.capabilities,
    makerFeeOverrideMultiplier: message.makerFeeOverrideMultiplier,
    takerFeeOverrideMultiplier: message.takerFeeOverrideMultiplier,
    subaccounts: message.subaccounts ?? [],
    deltas: message.deltas ?? [],
  }));

export const TraderStateUpdateSchema: z.ZodType<TraderStateUpdate> = z.union([
  TraderStateUpdateObjectSchema,
  TraderStateServerMessageToUpdateSchema,
]);
