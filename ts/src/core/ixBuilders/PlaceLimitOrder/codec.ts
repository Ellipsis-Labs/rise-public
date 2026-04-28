import { DISCRIMINANTS } from "@/core/discriminants";
import type { LimitOrderPacket } from "@/primitives/OrderPacket";
import {
  getLimitOrderPacketDecoder,
  getLimitOrderPacketEncoder,
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

export const getPlaceLimitOrderEncoder = (): Encoder<LimitOrderPacket> =>
  getHiddenPrefixEncoder(getLimitOrderPacketEncoder(), [
    getConstantEncoder(DISCRIMINANTS.PLACE_LIMIT_ORDER),
    getConstantEncoder(new Uint8Array([1])), // OrderPacketKind::Limit discriminant
  ]);

export const getPlaceLimitOrderDecoder = (): Decoder<LimitOrderPacket> =>
  getHiddenPrefixDecoder(getLimitOrderPacketDecoder(), [
    getConstantDecoder(DISCRIMINANTS.PLACE_LIMIT_ORDER),
    getConstantDecoder(new Uint8Array([1])), // OrderPacketKind::Limit discriminant
  ]);

export const getPlaceLimitOrderCodec = (): Codec<LimitOrderPacket> =>
  combineCodec(getPlaceLimitOrderEncoder(), getPlaceLimitOrderDecoder());
