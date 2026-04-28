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

export const getAcceptEscrowRequestEncoder = (): Encoder<void> =>
  fixEncoderSize(getConstantEncoder(DISCRIMINANTS.ACCEPT_ESCROW_REQUEST), 8);

export const getAcceptEscrowRequestDecoder = (): Decoder<void> =>
  fixDecoderSize(getConstantDecoder(DISCRIMINANTS.ACCEPT_ESCROW_REQUEST), 8);

export const getAcceptEscrowRequestCodec = (): Codec<void> =>
  combineCodec(
    getAcceptEscrowRequestEncoder(),
    getAcceptEscrowRequestDecoder()
  );
