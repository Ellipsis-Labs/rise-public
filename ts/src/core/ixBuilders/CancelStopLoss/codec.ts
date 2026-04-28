import { DISCRIMINANTS } from "@/core/discriminants";
import {
  getDirectionDecoder,
  getDirectionEncoder,
} from "@/primitives/StopLoss";
import {
  combineCodec,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getStructDecoder,
  getStructEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type { CancelStopLossData } from "./types";

export const getCancelStopLossDataEncoder = (): Encoder<CancelStopLossData> =>
  getStructEncoder([["executionDirection", getDirectionEncoder()]]);

export const getCancelStopLossDataDecoder = (): Decoder<CancelStopLossData> =>
  getStructDecoder([
    ["executionDirection", getDirectionDecoder()],
  ]) as unknown as Decoder<CancelStopLossData>;

export const getCancelStopLossEncoder = (): Encoder<CancelStopLossData> =>
  getHiddenPrefixEncoder(getCancelStopLossDataEncoder(), [
    getConstantEncoder(DISCRIMINANTS.CANCEL_STOP_LOSS),
  ]);

export const getCancelStopLossDecoder = (): Decoder<CancelStopLossData> =>
  getHiddenPrefixDecoder(getCancelStopLossDataDecoder(), [
    getConstantDecoder(DISCRIMINANTS.CANCEL_STOP_LOSS),
  ]);

export const getCancelStopLossCodec = (): Codec<CancelStopLossData> =>
  combineCodec(getCancelStopLossEncoder(), getCancelStopLossDecoder());
