import { DISCRIMINANTS } from "@/core/discriminants";
import {
  getMultipleOrderPacketDecoder,
  getMultipleOrderPacketEncoder,
  type MultipleOrderPacket,
} from "@/primitives";
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

export const getPlaceMultiLimitOrderEncoder =
  (): Encoder<MultipleOrderPacket> =>
    getHiddenPrefixEncoder(getMultipleOrderPacketEncoder(), [
      getConstantEncoder(DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER),
    ]);

export const getPlaceMultiLimitOrderDecoder =
  (): Decoder<MultipleOrderPacket> =>
    getHiddenPrefixDecoder(getMultipleOrderPacketDecoder(), [
      getConstantDecoder(DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER),
    ]);

export const getPlaceMultiLimitOrderCodec = (): Codec<MultipleOrderPacket> =>
  combineCodec(
    getPlaceMultiLimitOrderEncoder(),
    getPlaceMultiLimitOrderDecoder()
  );
