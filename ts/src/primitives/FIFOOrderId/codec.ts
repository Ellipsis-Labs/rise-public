import {
  combineCodec,
  getStructDecoder,
  getStructEncoder,
  getU64Decoder,
  getU64Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import { getTicksDecoder, getTicksEncoder } from "../_numberTypes";
import type { FIFOOrderId } from "./types";

export const getFIFOOrderIdDecoder = (): Decoder<FIFOOrderId> =>
  getStructDecoder([
    ["priceInTicks", getTicksDecoder()],
    ["orderSequenceNumber", getU64Decoder()],
  ]);

export const getFIFOOrderIdEncoder = (): Encoder<FIFOOrderId> =>
  getStructEncoder([
    ["priceInTicks", getTicksEncoder()],
    ["orderSequenceNumber", getU64Encoder()],
  ]);

export const getFIFOOrderIdCodec = (): Codec<FIFOOrderId> =>
  combineCodec(getFIFOOrderIdEncoder(), getFIFOOrderIdDecoder());
