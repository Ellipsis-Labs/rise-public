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

export const getTransferCollateralEncoder = (): Encoder<bigint> =>
  getHiddenPrefixEncoder(getU64Encoder(), [
    getConstantEncoder(DISCRIMINANTS.TRANSFER_COLLATERAL),
  ]);

export const getTransferCollateralDecoder = (): Decoder<bigint> =>
  getHiddenPrefixDecoder(getU64Decoder(), [
    getConstantDecoder(DISCRIMINANTS.TRANSFER_COLLATERAL),
  ]);

export const getTransferCollateralCodec = (): Codec<bigint> =>
  combineCodec(getTransferCollateralEncoder(), getTransferCollateralDecoder());
