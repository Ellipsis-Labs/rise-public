import { DISCRIMINANTS } from "@/core/discriminants";
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

export interface RegisterTraderParamsData {
  maxPositions: bigint;
  traderPdaIndex: number;
  subaccountIndex: number;
}

export const getRegisterTraderParamsEncoder =
  (): Encoder<RegisterTraderParamsData> =>
    getStructEncoder([
      ["maxPositions", getU64Encoder()],
      ["traderPdaIndex", getU8Encoder()],
      ["subaccountIndex", getU8Encoder()],
    ]);

export const getRegisterTraderParamsDecoder =
  (): Decoder<RegisterTraderParamsData> =>
    getStructDecoder([
      ["maxPositions", getU64Decoder()],
      ["traderPdaIndex", getU8Decoder()],
      ["subaccountIndex", getU8Decoder()],
    ]);

export const getRegisterTraderParamsCodec =
  (): Codec<RegisterTraderParamsData> =>
    combineCodec(
      getRegisterTraderParamsEncoder(),
      getRegisterTraderParamsDecoder()
    );

export const getRegisterTraderInstructionEncoder =
  (): Encoder<RegisterTraderParamsData> =>
    getHiddenPrefixEncoder(getRegisterTraderParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.REGISTER_TRADER),
    ]);

export const getRegisterTraderInstructionDecoder =
  (): Decoder<RegisterTraderParamsData> =>
    getHiddenPrefixDecoder(getRegisterTraderParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.REGISTER_TRADER),
    ]);

export const getRegisterTraderInstructionCodec =
  (): Codec<RegisterTraderParamsData> =>
    combineCodec(
      getRegisterTraderInstructionEncoder(),
      getRegisterTraderInstructionDecoder()
    );
