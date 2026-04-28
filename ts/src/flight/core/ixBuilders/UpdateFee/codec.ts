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
import { FLIGHT_DISCRIMINANTS } from "../../discriminants.js";

export type UpdateFeeParamsData = bigint;

export const getUpdateFeeParamsEncoder = (): Encoder<UpdateFeeParamsData> =>
  getU64Encoder();

export const getUpdateFeeParamsDecoder = (): Decoder<UpdateFeeParamsData> =>
  getU64Decoder();

export const getUpdateFeeParamsCodec = (): Codec<UpdateFeeParamsData> =>
  combineCodec(getUpdateFeeParamsEncoder(), getUpdateFeeParamsDecoder());

export const getUpdateFeeInstructionEncoder =
  (): Encoder<UpdateFeeParamsData> =>
    getHiddenPrefixEncoder(getUpdateFeeParamsEncoder(), [
      getConstantEncoder(FLIGHT_DISCRIMINANTS.UPDATE_FEE),
    ]);

export const getUpdateFeeInstructionDecoder =
  (): Decoder<UpdateFeeParamsData> =>
    getHiddenPrefixDecoder(getUpdateFeeParamsDecoder(), [
      getConstantDecoder(FLIGHT_DISCRIMINANTS.UPDATE_FEE),
    ]);

export const getUpdateFeeInstructionCodec = (): Codec<UpdateFeeParamsData> =>
  combineCodec(
    getUpdateFeeInstructionEncoder(),
    getUpdateFeeInstructionDecoder()
  );
