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

export const getCreateEscrowAccountEncoder = (): Encoder<bigint> =>
  getHiddenPrefixEncoder(getU64Encoder(), [
    getConstantEncoder(DISCRIMINANTS.CREATE_ESCROW_ACCOUNT),
  ]);

export const getCreateEscrowAccountDecoder = (): Decoder<bigint> =>
  getHiddenPrefixDecoder(getU64Decoder(), [
    getConstantDecoder(DISCRIMINANTS.CREATE_ESCROW_ACCOUNT),
  ]);

export const getCreateEscrowAccountCodec = (): Codec<bigint> =>
  combineCodec(
    getCreateEscrowAccountEncoder(),
    getCreateEscrowAccountDecoder()
  );
