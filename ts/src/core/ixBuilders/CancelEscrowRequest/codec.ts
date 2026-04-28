import { DISCRIMINANTS } from "@/core/discriminants";
import {
  combineCodec,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getU64Decoder,
  getU64Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";

export const getCancelEscrowRequestEncoder = (): Encoder<bigint> =>
  getHiddenPrefixEncoder(getU64Encoder(), [
    getConstantEncoder(DISCRIMINANTS.CANCEL_ESCROW_REQUEST),
  ]);

export const getCancelEscrowRequestDecoder = (): Decoder<bigint> =>
  getHiddenPrefixDecoder(getU64Decoder(), [
    getConstantDecoder(DISCRIMINANTS.CANCEL_ESCROW_REQUEST),
  ]);

export const getCancelEscrowRequestCodec = (): Codec<bigint> =>
  combineCodec(
    getCancelEscrowRequestEncoder(),
    getCancelEscrowRequestDecoder()
  );
