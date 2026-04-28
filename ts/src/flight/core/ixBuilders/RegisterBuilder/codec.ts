import {
  combineCodec,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getStructDecoder,
  getStructEncoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import { FLIGHT_DISCRIMINANTS } from "../../discriminants.js";

export interface RegisterBuilderParamsData {
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  feeBps: bigint;
}

export const getRegisterBuilderParamsEncoder =
  (): Encoder<RegisterBuilderParamsData> =>
    getStructEncoder([
      ["traderPdaIndex", getU8Encoder()],
      ["traderSubaccountIndex", getU8Encoder()],
      ["feeBps", getU64Encoder()],
    ]);

export const getRegisterBuilderParamsDecoder =
  (): Decoder<RegisterBuilderParamsData> =>
    getStructDecoder([
      ["traderPdaIndex", getU8Decoder()],
      ["traderSubaccountIndex", getU8Decoder()],
      ["feeBps", getU64Decoder()],
    ]);

export const getRegisterBuilderParamsCodec =
  (): Codec<RegisterBuilderParamsData> =>
    combineCodec(
      getRegisterBuilderParamsEncoder(),
      getRegisterBuilderParamsDecoder()
    );

export const getRegisterBuilderInstructionEncoder =
  (): Encoder<RegisterBuilderParamsData> =>
    getHiddenPrefixEncoder(getRegisterBuilderParamsEncoder(), [
      getConstantEncoder(FLIGHT_DISCRIMINANTS.REGISTER_BUILDER),
    ]);

export const getRegisterBuilderInstructionDecoder =
  (): Decoder<RegisterBuilderParamsData> =>
    getHiddenPrefixDecoder(getRegisterBuilderParamsDecoder(), [
      getConstantDecoder(FLIGHT_DISCRIMINANTS.REGISTER_BUILDER),
    ]);

export const getRegisterBuilderInstructionCodec =
  (): Codec<RegisterBuilderParamsData> =>
    combineCodec(
      getRegisterBuilderInstructionEncoder(),
      getRegisterBuilderInstructionDecoder()
    );
