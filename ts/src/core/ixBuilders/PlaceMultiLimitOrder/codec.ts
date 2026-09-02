import { DISCRIMINANTS } from "@/core/discriminants";
import {
  getMultipleOrderPacketDecoder,
  getMultipleOrderPacketEncoder,
  getMultipleOrderPacketV2Decoder,
  getMultipleOrderPacketV2Encoder,
  type MultipleOrderPacket,
  type MultipleOrderPacketV2,
} from "@/primitives/OrderPacket";
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

export const getPlaceMultiLimitOrderV2Encoder =
  (): Encoder<MultipleOrderPacketV2> =>
    getHiddenPrefixEncoder(getMultipleOrderPacketV2Encoder(), [
      getConstantEncoder(DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER_V2),
    ]);

export const getPlaceMultiLimitOrderV2Decoder =
  (): Decoder<MultipleOrderPacketV2> =>
    getHiddenPrefixDecoder(getMultipleOrderPacketV2Decoder(), [
      getConstantDecoder(DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER_V2),
    ]);

export const getPlaceMultiLimitOrderV2Codec =
  (): Codec<MultipleOrderPacketV2> =>
    combineCodec(
      getPlaceMultiLimitOrderV2Encoder(),
      getPlaceMultiLimitOrderV2Decoder()
    );
