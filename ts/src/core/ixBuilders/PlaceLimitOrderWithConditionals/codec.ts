import { DISCRIMINANTS } from "@/core/discriminants";
import {
  getPlaceLimitOrderWithConditionalsParamsCodec,
  getPlaceLimitOrderWithConditionalsParamsDecoder,
  getPlaceLimitOrderWithConditionalsParamsEncoder,
} from "@/primitives/ConditionalOrder";
import type { PlaceLimitOrderWithConditionalsData } from "@/primitives";
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

export const getPlaceLimitOrderWithConditionalsEncoder =
  (): Encoder<PlaceLimitOrderWithConditionalsData> =>
    getHiddenPrefixEncoder(getPlaceLimitOrderWithConditionalsParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.PLACE_LIMIT_ORDER_WITH_CONDITIONALS),
    ]);

export const getPlaceLimitOrderWithConditionalsDecoder =
  (): Decoder<PlaceLimitOrderWithConditionalsData> =>
    getHiddenPrefixDecoder(getPlaceLimitOrderWithConditionalsParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.PLACE_LIMIT_ORDER_WITH_CONDITIONALS),
    ]);

export const getPlaceLimitOrderWithConditionalsCodec =
  (): Codec<PlaceLimitOrderWithConditionalsData> =>
    combineCodec(
      getPlaceLimitOrderWithConditionalsEncoder(),
      getPlaceLimitOrderWithConditionalsDecoder()
    );

export { getPlaceLimitOrderWithConditionalsParamsCodec };
