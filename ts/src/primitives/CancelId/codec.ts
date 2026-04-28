import {
  combineCodec,
  getStructDecoder,
  getStructEncoder,
  getU32Decoder,
  getU32Encoder,
  transformEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import { getFIFOOrderIdDecoder, getFIFOOrderIdEncoder } from "../FIFOOrderId";
import type { CancelId } from "./types";

export const getCancelIdDecoder = (): Decoder<CancelId> =>
  getStructDecoder([
    ["nodePointer", getU32Decoder()],
    ["orderId", getFIFOOrderIdDecoder()],
  ]);

export const getCancelIdEncoder = (): Encoder<CancelId> =>
  transformEncoder(
    getStructEncoder([
      ["nodePointer", getU32Encoder()],
      ["orderId", getFIFOOrderIdEncoder()],
    ]),
    (value) => ({
      nodePointer: value.nodePointer ?? 0,
      orderId: value.orderId,
    })
  );

export const getCancelIdCodec = (): Codec<CancelId> =>
  combineCodec(getCancelIdEncoder(), getCancelIdDecoder());
