import { DISCRIMINANTS } from "@/core/discriminants";
import {
  combineCodec,
  getArrayDecoder,
  getArrayEncoder,
  getBooleanDecoder,
  getBooleanEncoder,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getStructDecoder,
  getStructEncoder,
  getU8Decoder,
  getU8Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type {
  TraderPreferenceToggle,
  UpdateTraderPreferenceBitsData,
} from "./types";

const getTraderPreferenceToggleEncoder = (): Encoder<TraderPreferenceToggle> =>
  getStructEncoder([
    ["target", getU8Encoder()],
    ["set", getBooleanEncoder()],
  ]);

const getTraderPreferenceToggleDecoder = (): Decoder<TraderPreferenceToggle> =>
  getStructDecoder([
    ["target", getU8Decoder()],
    ["set", getBooleanDecoder()],
  ]);

export const getUpdateTraderPreferenceBitsDataEncoder =
  (): Encoder<UpdateTraderPreferenceBitsData> =>
    getStructEncoder([
      ["toggles", getArrayEncoder(getTraderPreferenceToggleEncoder())],
    ]);

export const getUpdateTraderPreferenceBitsDataDecoder =
  (): Decoder<UpdateTraderPreferenceBitsData> =>
    getStructDecoder([
      ["toggles", getArrayDecoder(getTraderPreferenceToggleDecoder())],
    ]);

export const getUpdateTraderPreferenceBitsEncoder =
  (): Encoder<UpdateTraderPreferenceBitsData> =>
    getHiddenPrefixEncoder(getUpdateTraderPreferenceBitsDataEncoder(), [
      getConstantEncoder(DISCRIMINANTS.UPDATE_TRADER_PREFERENCE_BITS),
    ]);

export const getUpdateTraderPreferenceBitsDecoder =
  (): Decoder<UpdateTraderPreferenceBitsData> =>
    getHiddenPrefixDecoder(getUpdateTraderPreferenceBitsDataDecoder(), [
      getConstantDecoder(DISCRIMINANTS.UPDATE_TRADER_PREFERENCE_BITS),
    ]);

export const getUpdateTraderPreferenceBitsCodec =
  (): Codec<UpdateTraderPreferenceBitsData> =>
    combineCodec(
      getUpdateTraderPreferenceBitsEncoder(),
      getUpdateTraderPreferenceBitsDecoder()
    );
