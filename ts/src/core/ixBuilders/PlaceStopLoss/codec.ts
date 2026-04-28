import { DISCRIMINANTS } from "@/core/discriminants";
import { getSideDecoder, getSideEncoder } from "@/primitives/Side";
import {
  getDirectionDecoder,
  getDirectionEncoder,
  getStopLossOrderKindDecoder,
  getStopLossOrderKindEncoder,
  type Direction,
  type StopLossOrderKind,
} from "@/primitives/StopLoss";
import type { Side } from "@/primitives/Side";
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

export interface PlaceStopLossData {
  triggerPrice: bigint;
  executionPrice: bigint;
  tradeSize: bigint;
  tradeSide: Side;
  executionDirection: Direction;
  orderKind: StopLossOrderKind;
}

export const getPlaceStopLossDataEncoder = (): Encoder<PlaceStopLossData> =>
  getStructEncoder([
    ["triggerPrice", getU64Encoder()],
    ["executionPrice", getU64Encoder()],
    ["tradeSize", getU64Encoder()],
    ["tradeSide", getSideEncoder()],
    ["executionDirection", getDirectionEncoder()],
    ["orderKind", getStopLossOrderKindEncoder()],
  ]);

export const getPlaceStopLossDataDecoder = (): Decoder<PlaceStopLossData> =>
  getStructDecoder([
    ["triggerPrice", getU64Decoder()],
    ["executionPrice", getU64Decoder()],
    ["tradeSize", getU64Decoder()],
    ["tradeSide", getSideDecoder()],
    ["executionDirection", getDirectionDecoder()],
    ["orderKind", getStopLossOrderKindDecoder()],
  ]) as unknown as Decoder<PlaceStopLossData>;

export const getPlaceStopLossEncoder = (): Encoder<PlaceStopLossData> =>
  getHiddenPrefixEncoder(getPlaceStopLossDataEncoder(), [
    getConstantEncoder(DISCRIMINANTS.PLACE_STOP_LOSS),
  ]);

export const getPlaceStopLossDecoder = (): Decoder<PlaceStopLossData> =>
  getHiddenPrefixDecoder(getPlaceStopLossDataDecoder(), [
    getConstantDecoder(DISCRIMINANTS.PLACE_STOP_LOSS),
  ]);

export const getPlaceStopLossCodec = (): Codec<PlaceStopLossData> =>
  combineCodec(getPlaceStopLossEncoder(), getPlaceStopLossDecoder());
