import type { FIFOOrderId } from "@/primitives/FIFOOrderId";
import type { Direction, StopLossOrderKind } from "@/primitives/StopLoss";
import type { Side } from "@/primitives/Side";
import type { BaseLots, Ticks } from "@/primitives/_numberTypes";
import type { SequenceNumber } from "../internal";
import type { Address } from "@solana/kit";

export interface ConditionalOrderCollectionHeader {
  traderKey: Address;
  fundingKey: Address;
  sequenceNumber: SequenceNumber;
  len: number;
  capacity: number;
}

export interface ConditionalOrderTrigger {
  triggerPrice: Ticks;
  executionPrice: Ticks;
  positionSequenceNumber: number;
  isActive: boolean;
  executionDirection: Direction;
  tradeSide: Side;
  orderKind: StopLossOrderKind;
}

export interface ConditionalOrder {
  sequenceNumber: bigint;
  orderId: FIFOOrderId | null;
  maxSize: BaseLots;
  fillableSize: BaseLots;
  filledSize: BaseLots;
  slot: bigint;
  assetId: number;
  usePercent: boolean;
  percent: number;
  greaterTriggerOrder: ConditionalOrderTrigger;
  lessTriggerOrder: ConditionalOrderTrigger;
  isActive: boolean;
}

export interface ConditionalOrderCollection {
  header: ConditionalOrderCollectionHeader;
  orders: ConditionalOrder[];
  activeOrderIndices: number[];
}
