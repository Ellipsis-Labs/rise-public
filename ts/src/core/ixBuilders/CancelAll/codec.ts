import { DISCRIMINANTS } from "@/core/discriminants";
import {
  combineCodec,
  fixDecoderSize,
  fixEncoderSize,
  getConstantDecoder,
  getConstantEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";

export type CancelAll = Record<string, never>;

export const getCancelAllEncoder = (): Encoder<void> =>
  fixEncoderSize(getConstantEncoder(DISCRIMINANTS.CANCEL_ALL), 8);

export const getCancelAllDecoder = (): Decoder<void> =>
  fixDecoderSize(getConstantDecoder(DISCRIMINANTS.CANCEL_ALL), 8);

export const getCancelAllCodec = (): Codec<void> =>
  combineCodec(getCancelAllEncoder(), getCancelAllDecoder());
