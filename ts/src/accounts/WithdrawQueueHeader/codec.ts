import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import { getQuoteLotsDecoder } from "@/primitives/_numberTypes";
import { getSequenceNumberDecoder } from "../internal";
import {
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getStructDecoder,
  getU64Decoder,
  transformDecoder,
  type Decoder,
} from "@solana/kit";
import type { WithdrawQueueHeader, WithdrawThrottle } from "./types";

const getWithdrawThrottleDecoder = (): Decoder<WithdrawThrottle> =>
  getStructDecoder([
    ["maxBudget", getQuoteLotsDecoder()],
    ["remainingBudget", getQuoteLotsDecoder()],
    ["replenishAmountPerSlot", getQuoteLotsDecoder()],
    ["lastUpdateSlot", getU64Decoder()],
  ]);

interface WithdrawQueueHeaderInternal extends WithdrawQueueHeader {
  _reserved0: bigint;
  _reserved1: bigint;
  _reserved2: bigint;
  _reserved3: bigint;
  _reserved4: bigint;
}

const getWithdrawQueueHeaderInternalDecoder =
  (): Decoder<WithdrawQueueHeaderInternal> =>
    getHiddenPrefixDecoder(
      getStructDecoder([
        ["sequenceNumber", getSequenceNumberDecoder()],
        ["withdrawThrottle", getWithdrawThrottleDecoder()],
        ["totalQueuedAmount", getQuoteLotsDecoder()],
        ["withdrawalFee", getQuoteLotsDecoder()],
        ["enqueuingFee", getQuoteLotsDecoder()],
        ["_reserved0", getU64Decoder()],
        ["_reserved1", getU64Decoder()],
        ["_reserved2", getU64Decoder()],
        ["_reserved3", getU64Decoder()],
        ["_reserved4", getU64Decoder()],
      ]),
      [getConstantDecoder(ACCOUNT_DISCRIMINANTS.WITHDRAW_QUEUE_HEADER)]
    );

export const getWithdrawQueueHeaderDecoder = (): Decoder<WithdrawQueueHeader> =>
  transformDecoder(
    getWithdrawQueueHeaderInternalDecoder(),
    ({
      _reserved0,
      _reserved1,
      _reserved2,
      _reserved3,
      _reserved4,
      ...header
    }): WithdrawQueueHeader => header
  );

export const decodeWithdrawQueueHeader = (
  bytes: Uint8Array | Readonly<Uint8Array>
): WithdrawQueueHeader => getWithdrawQueueHeaderDecoder().decode(bytes);
