import type { FIFOOrderId } from "../FIFOOrderId";
import type {
  ImmediateOrCancelOrderPacket,
  LimitOrderPacket,
  PostOnlyOrderPacket,
} from "../OrderPacket";
import type { Side } from "../Side";
import type { Direction, StopLossOrderKind } from "../StopLoss";
import type { BaseLots, Ticks } from "../_numberTypes";

export interface TriggerOrderParams {
  triggerDirection: Direction;
  tradeSide: Side;
  orderKind: StopLossOrderKind;
  triggerPrice: Ticks;
  executionPrice: Ticks;
}

export type ConditionalOrderPacket =
  | ({ __kind: "PostOnly" } & PostOnlyOrderPacket)
  | ({ __kind: "Limit" } & LimitOrderPacket)
  | ({ __kind: "ImmediateOrCancel" } & ImmediateOrCancelOrderPacket);

export type ConditionalOrderIndex = number;

export type Percent = number;

export interface PlaceAttachedConditionalOrderData {
  orderId: FIFOOrderId;
  assetId: number;
  greaterTriggerOrder: TriggerOrderParams | null;
  lessTriggerOrder: TriggerOrderParams | null;
}

export interface PlacePositionConditionalOrderData {
  assetId: number;
  greaterTriggerOrder: TriggerOrderParams | null;
  lessTriggerOrder: TriggerOrderParams | null;
  sizeBaseLots: BaseLots | null;
  sizePercent: Percent | null;
}

export interface PlaceLimitOrderWithConditionalsData {
  orderPacket: ConditionalOrderPacket;
  slot: bigint;
  greaterTriggerOrder: TriggerOrderParams | null;
  lessTriggerOrder: TriggerOrderParams | null;
}
