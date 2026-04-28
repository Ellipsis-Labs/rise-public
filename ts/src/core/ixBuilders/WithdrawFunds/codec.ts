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

export const getWithdrawFundsEncoder = (): Encoder<bigint> =>
  getHiddenPrefixEncoder(getU64Encoder(), [
    getConstantEncoder(DISCRIMINANTS.WITHDRAW_FUNDS),
  ]);

export const getWithdrawFundsDecoder = (): Decoder<bigint> =>
  getHiddenPrefixDecoder(getU64Decoder(), [
    getConstantDecoder(DISCRIMINANTS.WITHDRAW_FUNDS),
  ]);

export const getWithdrawFundsCodec = (): Codec<bigint> =>
  combineCodec(getWithdrawFundsEncoder(), getWithdrawFundsDecoder());
