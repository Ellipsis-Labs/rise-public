import { DISCRIMINANTS } from "@/core/discriminants";
import type { ImmediateOrCancelOrderPacket } from "@/primitives/OrderPacket";
import {
  getImmediateOrCancelOrderPacketDecoder,
  getImmediateOrCancelOrderPacketEncoder,
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

export const getPlaceMarketOrderEncoder =
  (): Encoder<ImmediateOrCancelOrderPacket> =>
    getHiddenPrefixEncoder(getImmediateOrCancelOrderPacketEncoder(), [
      getConstantEncoder(DISCRIMINANTS.PLACE_MARKET_ORDER),
      getConstantEncoder(new Uint8Array([2])),
    ]);

export const getPlaceMarketOrderDecoder =
  (): Decoder<ImmediateOrCancelOrderPacket> =>
    getHiddenPrefixDecoder(getImmediateOrCancelOrderPacketDecoder(), [
      getConstantDecoder(DISCRIMINANTS.PLACE_MARKET_ORDER),
      getConstantDecoder(new Uint8Array([2])),
    ]);

export const getPlaceMarketOrderCodec =
  (): Codec<ImmediateOrCancelOrderPacket> =>
    combineCodec(getPlaceMarketOrderEncoder(), getPlaceMarketOrderDecoder());
