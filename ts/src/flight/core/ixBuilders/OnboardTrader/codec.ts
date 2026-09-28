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
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import { FLIGHT_DISCRIMINANTS } from "../../discriminants.js";

export interface OnboardTraderParamsData {
  maxPositions: number;
  traderPreferenceBits: number;
}

export const getOnboardTraderParamsEncoder =
  (): Encoder<OnboardTraderParamsData> =>
    getStructEncoder([
      ["maxPositions", getU32Encoder()],
      ["traderPreferenceBits", getU32Encoder()],
    ]);

export const getOnboardTraderParamsDecoder =
  (): Decoder<OnboardTraderParamsData> =>
    getStructDecoder([
      ["maxPositions", getU32Decoder()],
      ["traderPreferenceBits", getU32Decoder()],
    ]);

export const getOnboardTraderParamsCodec = (): Codec<OnboardTraderParamsData> =>
  combineCodec(
    getOnboardTraderParamsEncoder(),
    getOnboardTraderParamsDecoder()
  );

export const getOnboardTraderInstructionEncoder =
  (): Encoder<OnboardTraderParamsData> =>
    getHiddenPrefixEncoder(getOnboardTraderParamsEncoder(), [
      getConstantEncoder(FLIGHT_DISCRIMINANTS.ONBOARD_TRADER),
    ]);

export const getOnboardTraderInstructionDecoder =
  (): Decoder<OnboardTraderParamsData> =>
    getHiddenPrefixDecoder(getOnboardTraderParamsDecoder(), [
      getConstantDecoder(FLIGHT_DISCRIMINANTS.ONBOARD_TRADER),
    ]);

export const getOnboardTraderInstructionCodec =
  (): Codec<OnboardTraderParamsData> =>
    combineCodec(
      getOnboardTraderInstructionEncoder(),
      getOnboardTraderInstructionDecoder()
    );
