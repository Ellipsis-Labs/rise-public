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

export const getWithdrawToTargetLeverageEncoder = (): Encoder<bigint> =>
  getHiddenPrefixEncoder(getU64Encoder(), [
    getConstantEncoder(DISCRIMINANTS.WITHDRAW_TO_TARGET_LEVERAGE),
  ]);

export const getWithdrawToTargetLeverageDecoder = (): Decoder<bigint> =>
  getHiddenPrefixDecoder(getU64Decoder(), [
    getConstantDecoder(DISCRIMINANTS.WITHDRAW_TO_TARGET_LEVERAGE),
  ]);

export const getWithdrawToTargetLeverageCodec = (): Codec<bigint> =>
  combineCodec(
    getWithdrawToTargetLeverageEncoder(),
    getWithdrawToTargetLeverageDecoder()
  );
