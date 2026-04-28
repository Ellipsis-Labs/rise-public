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

export const getDepositFundsEncoder = (): Encoder<bigint> =>
  getHiddenPrefixEncoder(getU64Encoder(), [
    getConstantEncoder(DISCRIMINANTS.DEPOSIT_FUNDS),
  ]);

export const getDepositFundsDecoder = (): Decoder<bigint> =>
  getHiddenPrefixDecoder(getU64Decoder(), [
    getConstantDecoder(DISCRIMINANTS.DEPOSIT_FUNDS),
  ]);

export const getDepositFundsCodec = (): Codec<bigint> =>
  combineCodec(getDepositFundsEncoder(), getDepositFundsDecoder());
