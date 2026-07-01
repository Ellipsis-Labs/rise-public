import { DISCRIMINANTS } from "@/core/discriminants";
import {
  combineCodec,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getStructDecoder,
  getStructEncoder,
  getU32Decoder,
  getU32Encoder,
  getU8Decoder,
  getU8Encoder,
  transformDecoder,
  transformEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";

export interface RegisterTraderParamsData {
  maxPositions: bigint;
  traderPreferenceBits?: number;
  traderPdaIndex: number;
  subaccountIndex: number;
}

export const getRegisterTraderParamsEncoder =
  (): Encoder<RegisterTraderParamsData> =>
    transformEncoder(
      getStructEncoder([
        ["maxPositions", getU32Encoder()],
        ["traderPreferenceBits", getU32Encoder()],
        ["traderPdaIndex", getU8Encoder()],
        ["subaccountIndex", getU8Encoder()],
      ]),
      (value) => ({
        ...value,
        maxPositions: Number(value.maxPositions),
        traderPreferenceBits: value.traderPreferenceBits ?? 0,
      })
    );

export const getRegisterTraderParamsDecoder =
  (): Decoder<RegisterTraderParamsData> =>
    transformDecoder(
      getStructDecoder([
        ["maxPositions", getU32Decoder()],
        ["traderPreferenceBits", getU32Decoder()],
        ["traderPdaIndex", getU8Decoder()],
        ["subaccountIndex", getU8Decoder()],
      ]),
      (value) => ({
        ...value,
        maxPositions: BigInt(value.maxPositions),
      })
    );

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
