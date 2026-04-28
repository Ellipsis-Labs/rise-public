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

export const getDelegateTraderEncoder = (): Encoder<void> =>
  fixEncoderSize(getConstantEncoder(DISCRIMINANTS.DELEGATE_TRADER), 8);

export const getDelegateTraderDecoder = (): Decoder<void> =>
  fixDecoderSize(getConstantDecoder(DISCRIMINANTS.DELEGATE_TRADER), 8);

export const getDelegateTraderCodec = (): Codec<void> =>
  combineCodec(getDelegateTraderEncoder(), getDelegateTraderDecoder());
