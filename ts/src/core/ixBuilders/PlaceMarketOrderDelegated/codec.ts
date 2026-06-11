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

export const getPlaceMarketOrderDelegatedEncoder =
  (): Encoder<ImmediateOrCancelOrderPacket> =>
    getHiddenPrefixEncoder(getImmediateOrCancelOrderPacketEncoder(), [
      getConstantEncoder(DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED),
      getConstantEncoder(new Uint8Array([2])),
    ]);

export const getPlaceMarketOrderDelegatedDecoder =
  (): Decoder<ImmediateOrCancelOrderPacket> =>
    getHiddenPrefixDecoder(getImmediateOrCancelOrderPacketDecoder(), [
      getConstantDecoder(DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED),
      getConstantDecoder(new Uint8Array([2])),
    ]);

export const getPlaceMarketOrderDelegatedCodec =
  (): Codec<ImmediateOrCancelOrderPacket> =>
    combineCodec(
      getPlaceMarketOrderDelegatedEncoder(),
      getPlaceMarketOrderDelegatedDecoder()
    );
