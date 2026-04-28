import { getOptionToNullDecoder, getOptionToNullEncoder } from "@/core";
import type { Decoder, Encoder } from "@solana/kit";
import {
  getArrayDecoder,
  getArrayEncoder,
  getBooleanDecoder,
  getBooleanEncoder,
  getEnumDecoder,
  getEnumEncoder,
  getStructDecoder,
  getStructEncoder,
  getU128Decoder,
  getU128Encoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
} from "@solana/kit";
import {
  getBaseLotsDecoder,
  getBaseLotsEncoder,
  getQuoteLotsDecoder,
  getQuoteLotsEncoder,
  getTicksDecoder,
  getTicksEncoder,
} from "../_numberTypes";
import { getSideDecoder, getSideEncoder } from "../Side";
import {
  type OrderFlags,
  SelfTradeBehavior,
  type CondensedOrder,
  type ImmediateOrCancelOrderPacket,
  type LimitOrderPacket,
  type MultipleOrderPacket,
  type PostOnlyOrderPacket,
} from "./types";

export const getOrderFlagsDecoder = (): Decoder<OrderFlags> => {
  const u8Decoder = getU8Decoder();
  return {
    ...u8Decoder,
    decode: (bytes, offset = 0) =>
      u8Decoder.decode(bytes, offset) as OrderFlags,
  };
};

export const getOrderFlagsEncoder = (): Encoder<OrderFlags> => {
  const u8Encoder = getU8Encoder();
  return {
    ...u8Encoder,
    encode: (value: OrderFlags) => u8Encoder.encode(value),
  };
};

export const getSelfTradeBehaviorDecoder = (): Decoder<SelfTradeBehavior> =>
  getEnumDecoder(SelfTradeBehavior);
export const getSelfTradeBehaviorEncoder = (): Encoder<SelfTradeBehavior> =>
  getEnumEncoder(SelfTradeBehavior);

export const getPostOnlyOrderPacketDecoder = (): Decoder<PostOnlyOrderPacket> =>
  getStructDecoder([
    ["side", getSideDecoder()],
    ["priceInTicks", getTicksDecoder()],
    ["numBaseLots", getBaseLotsDecoder()],
    ["clientOrderId", getU128Decoder()],
    ["slide", getBooleanDecoder()],
    ["lastValidSlot", getOptionToNullDecoder(getU64Decoder())],
    ["orderFlags", getOrderFlagsDecoder()],
    ["cancelExisting", getBooleanDecoder()],
  ]);

export const getPostOnlyOrderPacketEncoder = (): Encoder<PostOnlyOrderPacket> =>
  getStructEncoder([
    ["side", getSideEncoder()],
    ["priceInTicks", getTicksEncoder()],
    ["numBaseLots", getBaseLotsEncoder()],
    ["clientOrderId", getU128Encoder()],
    ["slide", getBooleanEncoder()],
    ["lastValidSlot", getOptionToNullEncoder(getU64Encoder())],
    ["orderFlags", getOrderFlagsEncoder()],
    ["cancelExisting", getBooleanEncoder()],
  ]);

export const getLimitOrderPacketDecoder = (): Decoder<LimitOrderPacket> =>
  getStructDecoder([
    ["side", getSideDecoder()],
    ["priceInTicks", getTicksDecoder()],
    ["numBaseLots", getBaseLotsDecoder()],
    ["selfTradeBehavior", getSelfTradeBehaviorDecoder()],
    ["matchLimit", getOptionToNullDecoder(getU64Decoder())],
    ["clientOrderId", getU128Decoder()],
    ["lastValidSlot", getOptionToNullDecoder(getU64Decoder())],
    ["orderFlags", getOrderFlagsDecoder()],
    ["cancelExisting", getBooleanDecoder()],
  ]);

export const getLimitOrderPacketEncoder = (): Encoder<LimitOrderPacket> =>
  getStructEncoder([
    ["side", getSideEncoder()],
    ["priceInTicks", getTicksEncoder()],
    ["numBaseLots", getBaseLotsEncoder()],
    ["selfTradeBehavior", getSelfTradeBehaviorEncoder()],
    ["matchLimit", getOptionToNullEncoder(getU64Encoder())],
    ["clientOrderId", getU128Encoder()],
    ["lastValidSlot", getOptionToNullEncoder(getU64Encoder())],
    ["orderFlags", getOrderFlagsEncoder()],
    ["cancelExisting", getBooleanEncoder()],
  ]);

export const getImmediateOrCancelOrderPacketDecoder =
  (): Decoder<ImmediateOrCancelOrderPacket> =>
    getStructDecoder([
      ["side", getSideDecoder()],
      ["priceInTicks", getOptionToNullDecoder(getTicksDecoder())],
      ["numBaseLots", getBaseLotsDecoder()],
      ["numQuoteLots", getOptionToNullDecoder(getQuoteLotsDecoder())],
      ["minBaseLotsToFill", getBaseLotsDecoder()],
      ["minQuoteLotsToFill", getQuoteLotsDecoder()],
      ["selfTradeBehavior", getSelfTradeBehaviorDecoder()],
      ["matchLimit", getOptionToNullDecoder(getU64Decoder())],
      ["clientOrderId", getU128Decoder()],
      ["lastValidSlot", getOptionToNullDecoder(getU64Decoder())],
      ["orderFlags", getOrderFlagsDecoder()],
      ["cancelExisting", getBooleanDecoder()],
    ]);

export const getImmediateOrCancelOrderPacketEncoder =
  (): Encoder<ImmediateOrCancelOrderPacket> =>
    getStructEncoder([
      ["side", getSideEncoder()],
      ["priceInTicks", getOptionToNullEncoder(getTicksEncoder())],
      ["numBaseLots", getBaseLotsEncoder()],
      ["numQuoteLots", getOptionToNullEncoder(getQuoteLotsEncoder())],
      ["minBaseLotsToFill", getBaseLotsEncoder()],
      ["minQuoteLotsToFill", getQuoteLotsEncoder()],
      ["selfTradeBehavior", getSelfTradeBehaviorEncoder()],
      ["matchLimit", getOptionToNullEncoder(getU64Encoder())],
      ["clientOrderId", getU128Encoder()],
      ["lastValidSlot", getOptionToNullEncoder(getU64Encoder())],
      ["orderFlags", getOrderFlagsEncoder()],
      ["cancelExisting", getBooleanEncoder()],
    ]);

export const getCondensedOrderDecoder = (): Decoder<CondensedOrder> =>
  getStructDecoder([
    ["priceInTicks", getU64Decoder()],
    ["sizeInBaseLots", getU64Decoder()],
    ["lastValidSlot", getOptionToNullDecoder(getU64Decoder())],
  ]);

export const getCondensedOrderEncoder = (): Encoder<CondensedOrder> =>
  getStructEncoder([
    ["priceInTicks", getU64Encoder()],
    ["sizeInBaseLots", getU64Encoder()],
    ["lastValidSlot", getOptionToNullEncoder(getU64Encoder())],
  ]);

export const getMultipleOrderPacketDecoder = (): Decoder<MultipleOrderPacket> =>
  getStructDecoder([
    ["bids", getArrayDecoder(getCondensedOrderDecoder())],
    ["asks", getArrayDecoder(getCondensedOrderDecoder())],
    ["clientOrderId", getOptionToNullDecoder(getU128Decoder())],
    ["slide", getBooleanDecoder()],
  ]);

export const getMultipleOrderPacketEncoder = (): Encoder<MultipleOrderPacket> =>
  getStructEncoder([
    ["bids", getArrayEncoder(getCondensedOrderEncoder())],
    ["asks", getArrayEncoder(getCondensedOrderEncoder())],
    ["clientOrderId", getOptionToNullEncoder(getU128Encoder())],
    ["slide", getBooleanEncoder()],
  ]);
