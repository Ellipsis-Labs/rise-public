import { DISCRIMINANTS } from "@/core/discriminants";
import { getCancelIdDecoder, getCancelIdEncoder } from "@/primitives/CancelId";
import {
  combineCodec,
  getArrayDecoder,
  getArrayEncoder,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getStructDecoder,
  getStructEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type { CancelOrdersById } from "./types";

export const getCancelOrdersByIdEncoder = (): Encoder<CancelOrdersById> =>
  getHiddenPrefixEncoder(
    getStructEncoder([["orderIds", getArrayEncoder(getCancelIdEncoder())]]),
    [getConstantEncoder(DISCRIMINANTS.CANCEL_ORDERS_BY_ID)]
  );

export const getCancelOrdersByIdDecoder = (): Decoder<CancelOrdersById> =>
  getHiddenPrefixDecoder(
    getStructDecoder([["orderIds", getArrayDecoder(getCancelIdDecoder())]]),
    [getConstantDecoder(DISCRIMINANTS.CANCEL_ORDERS_BY_ID)]
  ) as Decoder<CancelOrdersById>;

export const getCancelOrdersByIdCodec = (): Codec<CancelOrdersById> =>
  combineCodec(getCancelOrdersByIdEncoder(), getCancelOrdersByIdDecoder());
