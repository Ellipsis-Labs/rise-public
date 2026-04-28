import { DISCRIMINANTS } from "@/core/discriminants";
import {
  getPlacePositionConditionalOrderParamsCodec,
  getPlacePositionConditionalOrderParamsDecoder,
  getPlacePositionConditionalOrderParamsEncoder,
} from "@/primitives/ConditionalOrder";
import type { PlacePositionConditionalOrderData } from "@/primitives";
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

export const getPlacePositionConditionalOrderEncoder =
  (): Encoder<PlacePositionConditionalOrderData> =>
    getHiddenPrefixEncoder(getPlacePositionConditionalOrderParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.PLACE_POSITION_CONDITIONAL_ORDER),
    ]);

export const getPlacePositionConditionalOrderDecoder =
  (): Decoder<PlacePositionConditionalOrderData> =>
    getHiddenPrefixDecoder(getPlacePositionConditionalOrderParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.PLACE_POSITION_CONDITIONAL_ORDER),
    ]);

export const getPlacePositionConditionalOrderCodec =
  (): Codec<PlacePositionConditionalOrderData> =>
    combineCodec(
      getPlacePositionConditionalOrderEncoder(),
      getPlacePositionConditionalOrderDecoder()
    );

export { getPlacePositionConditionalOrderParamsCodec };
