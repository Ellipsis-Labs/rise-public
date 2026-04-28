import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import type { Decoder } from "@solana/kit";
import {
  createDecoder,
  getConstantDecoder,
  getHiddenPrefixDecoder,
  transformDecoder,
} from "@solana/kit";
import {
  ORDERBOOK_CAPACITY,
  getOrderbookHeaderDecoder,
  getOrderbookRestingOrderDecoder,
  getStaticRedBlackTreeEntriesDecoder,
} from "../internal";
import { getFIFOOrderIdDecoder } from "@/primitives/FIFOOrderId";
import type { Orderbook } from "./types";

export const getOrderbookDecoder = (): Decoder<Orderbook> =>
  transformDecoder(
    getHiddenPrefixDecoder(
      createDecoder({
        read: (bytes, offset) => {
          const [header, afterHeader] = getOrderbookHeaderDecoder().read(
            bytes,
            offset
          );
          const treeDecoder = getStaticRedBlackTreeEntriesDecoder(
            getFIFOOrderIdDecoder,
            getOrderbookRestingOrderDecoder,
            { maxSize: ORDERBOOK_CAPACITY }
          );
          const [bids, afterBids] = treeDecoder.read(bytes, afterHeader);
          const [asks, afterAsks] = treeDecoder.read(bytes, afterBids);
          return [{ header, bids, asks }, afterAsks];
        },
      }),
      [getConstantDecoder(ACCOUNT_DISCRIMINANTS.ORDERBOOK)]
    ),
    ({ header, bids, asks }): Orderbook => ({
      header,
      bids: bids.map(({ key, value }) => ({ orderId: key, order: value })),
      asks: asks.map(({ key, value }) => ({ orderId: key, order: value })),
    })
  );

export const decodeOrderbook = (
  bytes: Uint8Array | Readonly<Uint8Array>
): Orderbook => getOrderbookDecoder().decode(bytes);
