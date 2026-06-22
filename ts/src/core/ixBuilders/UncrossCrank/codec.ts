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
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type { UncrossCrankInstruction } from "./types";

export const getUncrossCrankInstructionEncoder =
  (): Encoder<UncrossCrankInstruction> =>
    getStructEncoder([["matchLimit", getU64Encoder()]]);

export const getUncrossCrankInstructionDecoder =
  (): Decoder<UncrossCrankInstruction> =>
    getStructDecoder([["matchLimit", getU64Decoder()]]);

export const getUncrossCrankInstructionCodec =
  (): Codec<UncrossCrankInstruction> =>
    combineCodec(
      getUncrossCrankInstructionEncoder(),
      getUncrossCrankInstructionDecoder()
    );

export const getUncrossCrankEncoder = (): Encoder<UncrossCrankInstruction> =>
  getHiddenPrefixEncoder(getUncrossCrankInstructionEncoder(), [
    getConstantEncoder(DISCRIMINANTS.UNCROSS_CRANK),
  ]);

export const getUncrossCrankDecoder = (): Decoder<UncrossCrankInstruction> =>
  getHiddenPrefixDecoder(getUncrossCrankInstructionDecoder(), [
    getConstantDecoder(DISCRIMINANTS.UNCROSS_CRANK),
  ]);

export const getUncrossCrankCodec = (): Codec<UncrossCrankInstruction> =>
  combineCodec(getUncrossCrankEncoder(), getUncrossCrankDecoder());
