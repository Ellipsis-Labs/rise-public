import { DISCRIMINANTS } from "@/core/discriminants";
import {
  getPlaceAttachedConditionalOrderParamsCodec,
  getPlaceAttachedConditionalOrderParamsDecoder,
  getPlaceAttachedConditionalOrderParamsEncoder,
} from "@/primitives/ConditionalOrder";
import {
  combineCodec,
  getConstantDecoder,
  getConstantEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type { PlaceAttachedConditionalOrderData } from "@/primitives";

export const getPlaceAttachedConditionalOrderEncoder =
  (): Encoder<PlaceAttachedConditionalOrderData> =>
    getHiddenPrefixEncoder(getPlaceAttachedConditionalOrderParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.PLACE_ATTACHED_CONDITIONAL_ORDER),
    ]);

export const getPlaceAttachedConditionalOrderDecoder =
  (): Decoder<PlaceAttachedConditionalOrderData> =>
    getHiddenPrefixDecoder(getPlaceAttachedConditionalOrderParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.PLACE_ATTACHED_CONDITIONAL_ORDER),
    ]);

export const getPlaceAttachedConditionalOrderCodec =
  (): Codec<PlaceAttachedConditionalOrderData> =>
    combineCodec(
      getPlaceAttachedConditionalOrderEncoder(),
      getPlaceAttachedConditionalOrderDecoder()
    );

export { getPlaceAttachedConditionalOrderParamsCodec };
