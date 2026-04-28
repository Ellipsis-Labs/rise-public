import type { Codec, Decoder, Encoder } from "@solana/kit";
import { combineCodec, getEnumDecoder, getEnumEncoder } from "@solana/kit";
import { Direction, StopLossOrderKind } from "./types";

export const getDirectionEncoder = (): Encoder<Direction> =>
  getEnumEncoder(Direction);

export const getDirectionDecoder = (): Decoder<Direction> =>
  getEnumDecoder(Direction);

export const getDirectionCodec = (): Codec<Direction> =>
  combineCodec(getDirectionEncoder(), getDirectionDecoder());

export const getStopLossOrderKindEncoder = (): Encoder<StopLossOrderKind> =>
  getEnumEncoder(StopLossOrderKind);

export const getStopLossOrderKindDecoder = (): Decoder<StopLossOrderKind> =>
  getEnumDecoder(StopLossOrderKind);

export const getStopLossOrderKindCodec = (): Codec<StopLossOrderKind> =>
  combineCodec(getStopLossOrderKindEncoder(), getStopLossOrderKindDecoder());
