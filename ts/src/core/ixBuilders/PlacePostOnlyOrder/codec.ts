import { DISCRIMINANTS } from "@/core/discriminants";
import type { PostOnlyOrderPacket } from "@/primitives/OrderPacket";
import {
  getPostOnlyOrderPacketDecoder,
  getPostOnlyOrderPacketEncoder,
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

export const getPlacePostOnlyOrderEncoder = (): Encoder<PostOnlyOrderPacket> =>
  getHiddenPrefixEncoder(getPostOnlyOrderPacketEncoder(), [
    getConstantEncoder(DISCRIMINANTS.PLACE_LIMIT_ORDER),
    getConstantEncoder(new Uint8Array([0])),
  ]);

export const getPlacePostOnlyOrderDecoder = (): Decoder<PostOnlyOrderPacket> =>
  getHiddenPrefixDecoder(getPostOnlyOrderPacketDecoder(), [
    getConstantDecoder(DISCRIMINANTS.PLACE_LIMIT_ORDER),
    getConstantDecoder(new Uint8Array([0])),
  ]);

export const getPlacePostOnlyOrderCodec = (): Codec<PostOnlyOrderPacket> =>
  combineCodec(getPlacePostOnlyOrderEncoder(), getPlacePostOnlyOrderDecoder());
