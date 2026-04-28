import { EMBER_DISCRIMINANTS } from "@/core/discriminants";
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

export const getEmberDepositEncoder = (): Encoder<bigint> =>
  getHiddenPrefixEncoder(getU64Encoder(), [
    getConstantEncoder(EMBER_DISCRIMINANTS.DEPOSIT),
  ]);

export const getEmberDepositDecoder = (): Decoder<bigint> =>
  getHiddenPrefixDecoder(getU64Decoder(), [
    getConstantDecoder(EMBER_DISCRIMINANTS.DEPOSIT),
  ]);

export const getEmberDepositCodec = (): Codec<bigint> =>
  combineCodec(getEmberDepositEncoder(), getEmberDepositDecoder());
