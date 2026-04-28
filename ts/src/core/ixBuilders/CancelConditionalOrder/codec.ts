import { DISCRIMINANTS } from "@/core/discriminants";
import {
  combineCodec,
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
import type { CancelConditionalOrderData } from "./types";

export const getCancelConditionalOrderParamsEncoder =
  (): Encoder<CancelConditionalOrderData> =>
    getStructEncoder([
      ["conditionalOrderIndex", getU8Encoder()],
      ["disableFirst", getBooleanEncoder()],
      ["disableSecond", getBooleanEncoder()],
    ]);

export const getCancelConditionalOrderParamsDecoder =
  (): Decoder<CancelConditionalOrderData> =>
    getStructDecoder([
      ["conditionalOrderIndex", getU8Decoder()],
      ["disableFirst", getBooleanDecoder()],
      ["disableSecond", getBooleanDecoder()],
    ]) as unknown as Decoder<CancelConditionalOrderData>;

export const getCancelConditionalOrderEncoder =
  (): Encoder<CancelConditionalOrderData> =>
    getHiddenPrefixEncoder(getCancelConditionalOrderParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.CANCEL_CONDITIONAL_ORDER),
    ]);

export const getCancelConditionalOrderDecoder =
  (): Decoder<CancelConditionalOrderData> =>
    getHiddenPrefixDecoder(getCancelConditionalOrderParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.CANCEL_CONDITIONAL_ORDER),
    ]);

export const getCancelConditionalOrderCodec =
  (): Codec<CancelConditionalOrderData> =>
    combineCodec(
      getCancelConditionalOrderEncoder(),
      getCancelConditionalOrderDecoder()
    );
