import type { BaseLots, Ticks } from "@/primitives/_numberTypes";
import type { SequenceNumber } from "../internal";
import type { Direction, StopLossOrderKind } from "@/primitives/StopLoss";
import type { Side } from "@/primitives/Side";
import type { Address } from "@solana/kit";

export interface StopLoss {
  sequenceNumber: bigint;
  triggerPrice: Ticks;
  executionPrice: Ticks;
  tradeSize: BaseLots;
  slot: bigint;
  positionSequenceNumber: number;
  isActive: boolean;
  executionDirection: Direction;
  tradeSide: Side;
  orderKind: StopLossOrderKind;
}

export interface StopLosses {
  stopLosses: [StopLoss, StopLoss];
  isInitialized: boolean;
  sequenceNumber: SequenceNumber;
  fundingKey: Address;
  traderKey: Address;
  assetId: bigint;
}
