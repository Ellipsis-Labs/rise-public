import {
  combineCodec,
  getAddressDecoder,
  getAddressEncoder,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import { FLIGHT_DISCRIMINANTS } from "../../discriminants.js";

export type SetOnboarderSignerParamsData = Address;

export const getSetOnboarderSignerParamsEncoder =
  (): Encoder<SetOnboarderSignerParamsData> => getAddressEncoder();

export const getSetOnboarderSignerParamsDecoder =
  (): Decoder<SetOnboarderSignerParamsData> => getAddressDecoder();

export const getSetOnboarderSignerParamsCodec =
  (): Codec<SetOnboarderSignerParamsData> =>
    combineCodec(
      getSetOnboarderSignerParamsEncoder(),
      getSetOnboarderSignerParamsDecoder()
    );

export const getSetOnboarderSignerInstructionEncoder =
  (): Encoder<SetOnboarderSignerParamsData> =>
    getHiddenPrefixEncoder(getSetOnboarderSignerParamsEncoder(), [
      getConstantEncoder(FLIGHT_DISCRIMINANTS.SET_ONBOARDER_SIGNER),
    ]);

export const getSetOnboarderSignerInstructionDecoder =
  (): Decoder<SetOnboarderSignerParamsData> =>
    getHiddenPrefixDecoder(getSetOnboarderSignerParamsDecoder(), [
      getConstantDecoder(FLIGHT_DISCRIMINANTS.SET_ONBOARDER_SIGNER),
    ]);

export const getSetOnboarderSignerInstructionCodec =
  (): Codec<SetOnboarderSignerParamsData> =>
    combineCodec(
      getSetOnboarderSignerInstructionEncoder(),
      getSetOnboarderSignerInstructionDecoder()
    );
