import { DISCRIMINANTS } from "@/core/discriminants";
import { getOptionToNullDecoder, getOptionToNullEncoder } from "@/core/utils";
import { getSideDecoder, getSideEncoder } from "@/primitives";
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
import type { CancelUpToInstruction } from "./types";

export const getCancelUpToInstructionEncoder =
  (): Encoder<CancelUpToInstruction> =>
    getStructEncoder([
      ["side", getSideEncoder()],
      ["numOrdersToCancel", getOptionToNullEncoder(getU64Encoder())],
      ["tickLimit", getOptionToNullEncoder(getU64Encoder())],
    ]);

export const getCancelUpToInstructionDecoder =
  (): Decoder<CancelUpToInstruction> =>
    getStructDecoder([
      ["side", getSideDecoder()],
      ["numOrdersToCancel", getOptionToNullDecoder(getU64Decoder())],
      ["tickLimit", getOptionToNullDecoder(getU64Decoder())],
    ]);

export const getCancelUpToInstructionCodec = (): Codec<CancelUpToInstruction> =>
  combineCodec(
    getCancelUpToInstructionEncoder(),
    getCancelUpToInstructionDecoder()
  );

export const getCancelUpToEncoder = (): Encoder<CancelUpToInstruction> =>
  getHiddenPrefixEncoder(getCancelUpToInstructionEncoder(), [
    getConstantEncoder(DISCRIMINANTS.CANCEL_UP_TO),
  ]);

export const getCancelUpToDecoder = (): Decoder<CancelUpToInstruction> =>
  getHiddenPrefixDecoder(getCancelUpToInstructionDecoder(), [
    getConstantDecoder(DISCRIMINANTS.CANCEL_UP_TO),
  ]);

export const getCancelUpToCodec = (): Codec<CancelUpToInstruction> =>
  combineCodec(getCancelUpToEncoder(), getCancelUpToDecoder());
