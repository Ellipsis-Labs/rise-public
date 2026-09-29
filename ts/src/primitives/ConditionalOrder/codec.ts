import {
  getOptionToNullDecoder,
  getOptionToNullEncoder,
} from "@/core/utils/optionCodec";
import {
  getFIFOOrderIdDecoder,
  getFIFOOrderIdEncoder,
} from "@/primitives/FIFOOrderId";
import {
  getOrderPacketCodec,
  getOrderPacketDecoder,
  getOrderPacketEncoder,
} from "@/primitives/OrderPacket";
import { getSideDecoder, getSideEncoder } from "@/primitives/Side";
import {
  getDirectionDecoder,
  getDirectionEncoder,
  getStopLossOrderKindDecoder,
  getStopLossOrderKindEncoder,
} from "@/primitives/StopLoss";
import {
  getBaseLotsDecoder,
  getBaseLotsEncoder,
  getTicksDecoder,
  getTicksEncoder,
} from "@/primitives/_numberTypes";
import {
  combineCodec,
  getStructDecoder,
  getStructEncoder,
  getU32Decoder,
  getU32Encoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type {
  PlaceAttachedConditionalOrderData,
  PlaceLimitOrderWithConditionalsData,
  PlacePositionConditionalOrderData,
  TriggerOrderParams,
} from "./types";

export const getTriggerOrderParamsEncoder = (): Encoder<TriggerOrderParams> =>
  getStructEncoder([
    ["triggerDirection", getDirectionEncoder()],
    ["tradeSide", getSideEncoder()],
    ["orderKind", getStopLossOrderKindEncoder()],
    ["triggerPrice", getTicksEncoder()],
    ["executionPrice", getTicksEncoder()],
  ]);

export const getTriggerOrderParamsDecoder = (): Decoder<TriggerOrderParams> =>
  getStructDecoder([
    ["triggerDirection", getDirectionDecoder()],
    ["tradeSide", getSideDecoder()],
    ["orderKind", getStopLossOrderKindDecoder()],
    ["triggerPrice", getTicksDecoder()],
    ["executionPrice", getTicksDecoder()],
  ]) as unknown as Decoder<TriggerOrderParams>;

export const getTriggerOrderParamsCodec = (): Codec<TriggerOrderParams> =>
  combineCodec(getTriggerOrderParamsEncoder(), getTriggerOrderParamsDecoder());

export const getConditionalOrderPacketEncoder: typeof getOrderPacketEncoder =
  getOrderPacketEncoder;
export const getConditionalOrderPacketDecoder: typeof getOrderPacketDecoder =
  getOrderPacketDecoder;
export const getConditionalOrderPacketCodec: typeof getOrderPacketCodec =
  getOrderPacketCodec;

export const getPlaceAttachedConditionalOrderParamsEncoder =
  (): Encoder<PlaceAttachedConditionalOrderData> =>
    getStructEncoder([
      ["orderId", getFIFOOrderIdEncoder()],
      ["assetId", getU32Encoder()],
      [
        "greaterTriggerOrder",
        getOptionToNullEncoder(getTriggerOrderParamsEncoder()),
      ],
      [
        "lessTriggerOrder",
        getOptionToNullEncoder(getTriggerOrderParamsEncoder()),
      ],
    ]);

export const getPlaceAttachedConditionalOrderParamsDecoder =
  (): Decoder<PlaceAttachedConditionalOrderData> =>
    getStructDecoder([
      ["orderId", getFIFOOrderIdDecoder()],
      ["assetId", getU32Decoder()],
      [
        "greaterTriggerOrder",
        getOptionToNullDecoder(getTriggerOrderParamsDecoder()),
      ],
      [
        "lessTriggerOrder",
        getOptionToNullDecoder(getTriggerOrderParamsDecoder()),
      ],
    ]) as unknown as Decoder<PlaceAttachedConditionalOrderData>;

export const getPlaceAttachedConditionalOrderParamsCodec =
  (): Codec<PlaceAttachedConditionalOrderData> =>
    combineCodec(
      getPlaceAttachedConditionalOrderParamsEncoder(),
      getPlaceAttachedConditionalOrderParamsDecoder()
    );

export const getPlacePositionConditionalOrderParamsEncoder =
  (): Encoder<PlacePositionConditionalOrderData> =>
    getStructEncoder([
      ["assetId", getU32Encoder()],
      [
        "greaterTriggerOrder",
        getOptionToNullEncoder(getTriggerOrderParamsEncoder()),
      ],
      [
        "lessTriggerOrder",
        getOptionToNullEncoder(getTriggerOrderParamsEncoder()),
      ],
      ["sizeBaseLots", getOptionToNullEncoder(getBaseLotsEncoder())],
      ["sizePercent", getOptionToNullEncoder(getU8Encoder())],
    ]);

export const getPlacePositionConditionalOrderParamsDecoder =
  (): Decoder<PlacePositionConditionalOrderData> =>
    getStructDecoder([
      ["assetId", getU32Decoder()],
      [
        "greaterTriggerOrder",
        getOptionToNullDecoder(getTriggerOrderParamsDecoder()),
      ],
      [
        "lessTriggerOrder",
        getOptionToNullDecoder(getTriggerOrderParamsDecoder()),
      ],
      ["sizeBaseLots", getOptionToNullDecoder(getBaseLotsDecoder())],
      ["sizePercent", getOptionToNullDecoder(getU8Decoder())],
    ]) as unknown as Decoder<PlacePositionConditionalOrderData>;

export const getPlacePositionConditionalOrderParamsCodec =
  (): Codec<PlacePositionConditionalOrderData> =>
    combineCodec(
      getPlacePositionConditionalOrderParamsEncoder(),
      getPlacePositionConditionalOrderParamsDecoder()
    );

export const getPlaceLimitOrderWithConditionalsParamsEncoder =
  (): Encoder<PlaceLimitOrderWithConditionalsData> =>
    getStructEncoder([
      ["orderPacket", getConditionalOrderPacketEncoder()],
      ["slot", getU64Encoder()],
      [
        "greaterTriggerOrder",
        getOptionToNullEncoder(getTriggerOrderParamsEncoder()),
      ],
      [
        "lessTriggerOrder",
        getOptionToNullEncoder(getTriggerOrderParamsEncoder()),
      ],
    ]);

export const getPlaceLimitOrderWithConditionalsParamsDecoder =
  (): Decoder<PlaceLimitOrderWithConditionalsData> =>
    getStructDecoder([
      ["orderPacket", getConditionalOrderPacketDecoder()],
      ["slot", getU64Decoder()],
      [
        "greaterTriggerOrder",
        getOptionToNullDecoder(getTriggerOrderParamsDecoder()),
      ],
      [
        "lessTriggerOrder",
        getOptionToNullDecoder(getTriggerOrderParamsDecoder()),
      ],
    ]) as unknown as Decoder<PlaceLimitOrderWithConditionalsData>;

export const getPlaceLimitOrderWithConditionalsParamsCodec =
  (): Codec<PlaceLimitOrderWithConditionalsData> =>
    combineCodec(
      getPlaceLimitOrderWithConditionalsParamsEncoder(),
      getPlaceLimitOrderWithConditionalsParamsDecoder()
    );
