import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import type { Decoder } from "@solana/kit";
import {
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getStructDecoder,
  getU16Decoder,
  getU64Decoder,
  getU8Decoder,
  transformDecoder,
} from "@solana/kit";
import {
  CONDITIONAL_ORDER_BITS_LEN,
  getFixedArrayDecoder,
  getOccupiedConditionalOrderIndices,
  getSequenceNumberDecoder,
  getShortEntriesDecoder,
  getTraderAddressDecoder,
  getTraderPositionDecoder,
  getTraderStateDecoder,
  getAuthorityDecoder,
  getOptionalNonZeroU32Decoder,
} from "../internal";
import type { Trader } from "./types";

export const getTraderDecoder = (): Decoder<Trader> =>
  transformDecoder(
    getHiddenPrefixDecoder(
      getStructDecoder([
        ["sequenceNumber", getSequenceNumberDecoder()],
        ["key", getTraderAddressDecoder()],
        ["authority", getAuthorityDecoder()],
        ["state", getTraderStateDecoder()],
        ["_padding0", getFixedArrayDecoder(getU8Decoder, 4)],
        ["withdrawQueueNode", getOptionalNonZeroU32Decoder()],
        ["maxPositions", getU64Decoder()],
        ["positionAuthority", getAuthorityDecoder()],
        ["numMarketsWithSplines", getU16Decoder()],
        ["traderPdaIndex", getU8Decoder()],
        ["traderSubaccountIndex", getU8Decoder()],
        ["fundingKey", getAuthorityDecoder()],
        ["_padding1", getFixedArrayDecoder(getU8Decoder, 4)],
        ["lastDepositSlot", getU64Decoder()],
        [
          "conditionalOrderBits",
          getFixedArrayDecoder(getU8Decoder, CONDITIONAL_ORDER_BITS_LEN),
        ],
        [
          "positions",
          getShortEntriesDecoder(getU64Decoder, getTraderPositionDecoder),
        ],
      ]),
      [getConstantDecoder(ACCOUNT_DISCRIMINANTS.TRADER)]
    ),
    ({ _padding0, _padding1, conditionalOrderBits, ...trader }) => ({
      ...trader,
      conditionalOrderBits,
      occupiedConditionalOrderIndices:
        getOccupiedConditionalOrderIndices(conditionalOrderBits),
    })
  );

export const decodeTrader = (
  bytes: Uint8Array | Readonly<Uint8Array>
): Trader => getTraderDecoder().decode(bytes);
