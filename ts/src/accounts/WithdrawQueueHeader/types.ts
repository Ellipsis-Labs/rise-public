import type { QuoteLots } from "@/primitives/_numberTypes";
import type { SequenceNumber } from "../internal";

export interface WithdrawThrottle {
  maxBudget: QuoteLots;
  remainingBudget: QuoteLots;
  replenishAmountPerSlot: QuoteLots;
  lastUpdateSlot: bigint;
}

export interface WithdrawQueueHeader {
  sequenceNumber: SequenceNumber;
  withdrawThrottle: WithdrawThrottle;
  totalQueuedAmount: QuoteLots;
  withdrawalFee: QuoteLots;
  enqueuingFee: QuoteLots;
}
