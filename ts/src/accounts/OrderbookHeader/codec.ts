import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import { getOrderbookHeaderDecoder as getInternalOrderbookHeaderDecoder } from "../internal";
import {
  getConstantDecoder,
  getHiddenPrefixDecoder,
  type Decoder,
} from "@solana/kit";
import type { OrderbookHeader } from "./types";

export const getOrderbookHeaderDecoder = (): Decoder<OrderbookHeader> =>
  getHiddenPrefixDecoder(getInternalOrderbookHeaderDecoder(), [
    getConstantDecoder(ACCOUNT_DISCRIMINANTS.ORDERBOOK_HEADER),
  ]);

export const decodeOrderbookHeader = (
  bytes: Uint8Array | Readonly<Uint8Array>
): OrderbookHeader => getOrderbookHeaderDecoder().decode(bytes);
