import type { Codec, Decoder, Encoder } from "@solana/kit";
import {
  combineCodec,
  getI64Decoder,
  getI64Encoder,
  getU64Decoder,
  getU64Encoder,
  transformDecoder,
} from "@solana/kit";
import type {
  BaseLots,
  BaseLotsPerTick,
  QuoteLots,
  SignedBaseLots,
  SignedQuoteLots,
  Ticks,
} from "./types";

export const getTicksDecoder = (): Decoder<Ticks> =>
  transformDecoder(getU64Decoder(), (v) => v as Ticks);

export const getTicksEncoder = (): Encoder<Ticks> => getU64Encoder();

export const getTicksCodec = (): Codec<Ticks> =>
  combineCodec(getTicksEncoder(), getTicksDecoder());

export const getBaseLotsDecoder = (): Decoder<BaseLots> =>
  transformDecoder(getU64Decoder(), (v) => v as BaseLots);

export const getBaseLotsEncoder = (): Encoder<BaseLots> => getU64Encoder();

export const getBaseLotsCodec = (): Codec<BaseLots> =>
  combineCodec(getBaseLotsEncoder(), getBaseLotsDecoder());

export const getBaseLotsPerTickDecoder = (): Decoder<BaseLotsPerTick> =>
  transformDecoder(getU64Decoder(), (v) => v as BaseLotsPerTick);

export const getBaseLotsPerTickEncoder = (): Encoder<BaseLotsPerTick> =>
  getU64Encoder();

export const getBaseLotsPerTickCodec = (): Codec<BaseLotsPerTick> =>
  combineCodec(getBaseLotsPerTickEncoder(), getBaseLotsPerTickDecoder());

export const getQuoteLotsDecoder = (): Decoder<QuoteLots> =>
  transformDecoder(getU64Decoder(), (v) => v as QuoteLots);

export const getQuoteLotsEncoder = (): Encoder<QuoteLots> => getU64Encoder();

export const getQuoteLotsCodec = (): Codec<QuoteLots> =>
  combineCodec(getQuoteLotsEncoder(), getQuoteLotsDecoder());

export const getSignedBaseLotsDecoder = (): Decoder<SignedBaseLots> =>
  transformDecoder(getI64Decoder(), (v) => v as SignedBaseLots);

export const getSignedBaseLotsEncoder = (): Encoder<SignedBaseLots> =>
  getI64Encoder();

export const getSignedBaseLotsCodec = (): Codec<SignedBaseLots> =>
  combineCodec(getSignedBaseLotsEncoder(), getSignedBaseLotsDecoder());

export const getSignedQuoteLotsDecoder = (): Decoder<SignedQuoteLots> =>
  transformDecoder(getI64Decoder(), (v) => v as SignedQuoteLots);

export const getSignedQuoteLotsEncoder = (): Encoder<SignedQuoteLots> =>
  getI64Encoder();

export const getSignedQuoteLotsCodec = (): Codec<SignedQuoteLots> =>
  combineCodec(getSignedQuoteLotsEncoder(), getSignedQuoteLotsDecoder());
