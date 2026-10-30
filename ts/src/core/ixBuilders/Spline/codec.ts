import { DISCRIMINANTS } from "@/core/discriminants";
import { getOptionToNullDecoder, getOptionToNullEncoder } from "@/core/utils";
import {
  combineCodec,
  createEncoder,
  createDecoder,
  fixDecoderSize,
  fixEncoderSize,
  getArrayDecoder,
  getArrayEncoder,
  getBooleanDecoder,
  getBooleanEncoder,
  getBytesDecoder,
  getBytesEncoder,
  getConstantDecoder,
  getConstantEncoder,
  getDiscriminatedUnionDecoder,
  getDiscriminatedUnionEncoder,
  getHiddenPrefixDecoder,
  getHiddenPrefixEncoder,
  getStructDecoder,
  getStructEncoder,
  getU32Decoder,
  getU32Encoder,
  getU64Decoder,
  getU64Encoder,
  transformEncoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type {
  PositionSizeLimit,
  TickRegionParams,
  UpdateSplineParametersParamsWithOrdering,
  UpdateSplinePositionLimitsConfigParams,
  UpdateSplinePriceParams,
  UpdateSplinePriceParamsWithOrdering,
  UpdateSplinePriceWithMetadataParams,
} from "./types";

const normalizeClientOrderId = (
  clientOrderId: Uint8Array | undefined
): Uint8Array => {
  if (clientOrderId === undefined) {
    return new Uint8Array(16);
  }
  if (clientOrderId.length !== 16) {
    throw new Error("Client order id must be 16 bytes");
  }
  return clientOrderId;
};

export const getTickRegionParamsEncoder = (): Encoder<TickRegionParams> =>
  getStructEncoder([
    ["startOffset", getU64Encoder()],
    ["endOffset", getU64Encoder()],
    ["density", getU32Encoder()],
    ["topLevelHiddenTakeSize", getU32Encoder()],
    ["lifespan", getU64Encoder()],
  ]);

export const getTickRegionParamsDecoder = (): Decoder<TickRegionParams> =>
  getStructDecoder([
    ["startOffset", getU64Decoder()],
    ["endOffset", getU64Decoder()],
    ["density", getU32Decoder()],
    ["topLevelHiddenTakeSize", getU32Decoder()],
    ["lifespan", getU64Decoder()],
  ]) as unknown as Decoder<TickRegionParams>;

export const getTickRegionParamsCodec = (): Codec<TickRegionParams> =>
  combineCodec(getTickRegionParamsEncoder(), getTickRegionParamsDecoder());

export const getUpdateSplinePriceParamsEncoder =
  (): Encoder<UpdateSplinePriceParams> =>
    getStructEncoder([
      ["newMidPrice", getU64Encoder()],
      ["userUpdateSlot", getOptionToNullEncoder(getU64Encoder())],
      ["refreshRegions", getBooleanEncoder()],
    ]);

export const getUpdateSplinePriceParamsDecoder =
  (): Decoder<UpdateSplinePriceParams> =>
    getStructDecoder([
      ["newMidPrice", getU64Decoder()],
      ["userUpdateSlot", getOptionToNullDecoder(getU64Decoder())],
      ["refreshRegions", getBooleanDecoder()],
    ]) as unknown as Decoder<UpdateSplinePriceParams>;

export const getUpdateSplinePriceParamsCodec =
  (): Codec<UpdateSplinePriceParams> =>
    combineCodec(
      getUpdateSplinePriceParamsEncoder(),
      getUpdateSplinePriceParamsDecoder()
    );

export const getUpdateSplinePriceEncoder =
  (): Encoder<UpdateSplinePriceParams> =>
    getHiddenPrefixEncoder(getUpdateSplinePriceParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.UPDATE_SPLINE_PRICE),
    ]);

export const getUpdateSplinePriceDecoder =
  (): Decoder<UpdateSplinePriceParams> =>
    getHiddenPrefixDecoder(getUpdateSplinePriceParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.UPDATE_SPLINE_PRICE),
    ]);

export const getUpdateSplinePriceCodec = (): Codec<UpdateSplinePriceParams> =>
  combineCodec(getUpdateSplinePriceEncoder(), getUpdateSplinePriceDecoder());

export const getUpdateSplinePriceParamsWithOrderingEncoder =
  (): Encoder<UpdateSplinePriceParamsWithOrdering> => {
    const encoder = getStructEncoder([
      ["newMidPrice", getU64Encoder()],
      ["userUpdateSlot", getOptionToNullEncoder(getU64Encoder())],
      ["refreshRegions", getBooleanEncoder()],
      ["userSequenceNumber", getU64Encoder()],
      ["clientOrderId", fixEncoderSize(getBytesEncoder(), 16)],
      ["overrideSequenceNumber", getBooleanEncoder()],
    ]);

    return transformEncoder(encoder, (value) => ({
      ...value,
      clientOrderId: normalizeClientOrderId(value.clientOrderId),
      overrideSequenceNumber: value.overrideSequenceNumber ?? false,
    }));
  };

export const getUpdateSplinePriceParamsWithOrderingDecoder =
  (): Decoder<UpdateSplinePriceParamsWithOrdering> =>
    getStructDecoder([
      ["newMidPrice", getU64Decoder()],
      ["userUpdateSlot", getOptionToNullDecoder(getU64Decoder())],
      ["refreshRegions", getBooleanDecoder()],
      ["userSequenceNumber", getU64Decoder()],
      ["clientOrderId", fixDecoderSize(getBytesDecoder(), 16)],
      ["overrideSequenceNumber", getBooleanDecoder()],
    ]) as unknown as Decoder<UpdateSplinePriceParamsWithOrdering>;

export const getUpdateSplinePriceParamsWithOrderingCodec =
  (): Codec<UpdateSplinePriceParamsWithOrdering> =>
    combineCodec(
      getUpdateSplinePriceParamsWithOrderingEncoder(),
      getUpdateSplinePriceParamsWithOrderingDecoder()
    );

export const getUpdateSplinePriceWithOrderingEncoder =
  (): Encoder<UpdateSplinePriceParamsWithOrdering> =>
    getHiddenPrefixEncoder(getUpdateSplinePriceParamsWithOrderingEncoder(), [
      getConstantEncoder(DISCRIMINANTS.UPDATE_SPLINE_PRICE),
    ]);

export const getUpdateSplinePriceWithOrderingDecoder =
  (): Decoder<UpdateSplinePriceParamsWithOrdering> =>
    getHiddenPrefixDecoder(getUpdateSplinePriceParamsWithOrderingDecoder(), [
      getConstantDecoder(DISCRIMINANTS.UPDATE_SPLINE_PRICE),
    ]);

export const getUpdateSplinePriceWithOrderingCodec =
  (): Codec<UpdateSplinePriceParamsWithOrdering> =>
    combineCodec(
      getUpdateSplinePriceWithOrderingEncoder(),
      getUpdateSplinePriceWithOrderingDecoder()
    );

export const getUpdateSplinePriceWithMetadataParamsEncoder =
  (): Encoder<UpdateSplinePriceWithMetadataParams> => {
    const u64 = getU64Encoder();
    const u32 = getU32Encoder();
    // Variable-size to pair with the decoder, which also accepts a truncated
    // client order id.
    return createEncoder({
      getSizeFromValue: () => 48,
      write: (value, bytes, offset) => {
        const clientOrderId = normalizeClientOrderId(value.clientOrderId);
        const bidWidth = value.bidWidth ?? 0n;
        const askWidth = value.askWidth ?? 0n;
        for (const ticks of [value.newMidPrice, bidWidth, askWidth]) {
          if (ticks < 0n || ticks > 0xffff_ffffn) {
            throw new Error("Spline price or width exceeds the tick range");
          }
        }
        const flags =
          Number(value.refreshRegions) |
          (Number(value.overrideSequenceNumber ?? false) << 1) |
          (Number(value.userUpdateSlot != null) << 2) |
          (Number(value.bidWidth != null) << 3) |
          (Number(value.askWidth != null) << 4);
        offset = u32.write(Number(value.newMidPrice), bytes, offset);
        offset = u32.write(flags, bytes, offset);
        offset = u64.write(value.userSequenceNumber, bytes, offset);
        offset = u64.write(value.userUpdateSlot ?? 0n, bytes, offset);
        offset = u32.write(Number(bidWidth), bytes, offset);
        offset = u32.write(Number(askWidth), bytes, offset);
        bytes.set(clientOrderId, offset);
        return offset + 16;
      },
    });
  };

export const getUpdateSplinePriceWithMetadataParamsDecoder =
  (): Decoder<UpdateSplinePriceWithMetadataParams> => {
    const u64 = getU64Decoder();
    const u32 = getU32Decoder();
    return createDecoder({
      read: (bytes, offset) => {
        if (bytes.length - offset < 32) {
          throw new Error("Invalid spline metadata payload length");
        }
        const [newMidPrice, afterPrice] = u32.read(bytes, offset);
        const [flags, afterFlags] = u32.read(bytes, afterPrice);
        // Bits above the five defined flags are reserved and must be zero.
        if (flags >>> 5 !== 0) {
          throw new Error("Invalid spline metadata flags");
        }
        const [userSequenceNumber, afterSequence] = u64.read(bytes, afterFlags);
        const [updateSlot, afterSlot] = u64.read(bytes, afterSequence);
        const [bidWidth, afterBid] = u32.read(bytes, afterSlot);
        const [askWidth, afterAsk] = u32.read(bytes, afterBid);
        const clientOrderId =
          bytes.length - afterAsk >= 16
            ? Uint8Array.from(bytes.slice(afterAsk, afterAsk + 16))
            : new Uint8Array(16);
        return [
          {
            newMidPrice: BigInt(newMidPrice),
            refreshRegions: (flags & 1) !== 0,
            userSequenceNumber,
            overrideSequenceNumber: (flags & 2) !== 0,
            userUpdateSlot: (flags & 4) !== 0 ? updateSlot : null,
            bidWidth: (flags & 8) !== 0 ? BigInt(bidWidth) : null,
            askWidth: (flags & 16) !== 0 ? BigInt(askWidth) : null,
            clientOrderId,
          },
          bytes.length,
        ];
      },
    });
  };

export const getUpdateSplinePriceWithMetadataParamsCodec =
  (): Codec<UpdateSplinePriceWithMetadataParams> =>
    combineCodec(
      getUpdateSplinePriceWithMetadataParamsEncoder(),
      getUpdateSplinePriceWithMetadataParamsDecoder()
    );

export const getUpdateSplinePriceWithMetadataEncoder =
  (): Encoder<UpdateSplinePriceWithMetadataParams> =>
    getHiddenPrefixEncoder(getUpdateSplinePriceWithMetadataParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.UPDATE_SPLINE_PRICE_WITH_METADATA),
    ]);

export const getUpdateSplinePriceWithMetadataDecoder =
  (): Decoder<UpdateSplinePriceWithMetadataParams> =>
    getHiddenPrefixDecoder(getUpdateSplinePriceWithMetadataParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.UPDATE_SPLINE_PRICE_WITH_METADATA),
    ]);

export const getUpdateSplinePriceWithMetadataCodec =
  (): Codec<UpdateSplinePriceWithMetadataParams> =>
    combineCodec(
      getUpdateSplinePriceWithMetadataEncoder(),
      getUpdateSplinePriceWithMetadataDecoder()
    );

export const getUpdateSplineParametersParamsWithOrderingEncoder =
  (): Encoder<UpdateSplineParametersParamsWithOrdering> => {
    const encoder = getStructEncoder([
      ["bidRegions", getArrayEncoder(getTickRegionParamsEncoder())],
      ["askRegions", getArrayEncoder(getTickRegionParamsEncoder())],
      ["refreshRegions", getBooleanEncoder()],
      ["userSequenceNumber", getU64Encoder()],
      ["clientOrderId", fixEncoderSize(getBytesEncoder(), 16)],
      ["overrideSequenceNumber", getBooleanEncoder()],
    ]);

    return transformEncoder(encoder, (value) => ({
      ...value,
      clientOrderId: normalizeClientOrderId(value.clientOrderId),
      overrideSequenceNumber: value.overrideSequenceNumber ?? false,
    }));
  };

export const getUpdateSplineParametersParamsWithOrderingDecoder =
  (): Decoder<UpdateSplineParametersParamsWithOrdering> =>
    getStructDecoder([
      ["bidRegions", getArrayDecoder(getTickRegionParamsDecoder())],
      ["askRegions", getArrayDecoder(getTickRegionParamsDecoder())],
      ["refreshRegions", getBooleanDecoder()],
      ["userSequenceNumber", getU64Decoder()],
      ["clientOrderId", fixDecoderSize(getBytesDecoder(), 16)],
      ["overrideSequenceNumber", getBooleanDecoder()],
    ]) as unknown as Decoder<UpdateSplineParametersParamsWithOrdering>;

export const getUpdateSplineParametersParamsWithOrderingCodec =
  (): Codec<UpdateSplineParametersParamsWithOrdering> =>
    combineCodec(
      getUpdateSplineParametersParamsWithOrderingEncoder(),
      getUpdateSplineParametersParamsWithOrderingDecoder()
    );

export const getUpdateSplineParametersWithOrderingEncoder =
  (): Encoder<UpdateSplineParametersParamsWithOrdering> =>
    getHiddenPrefixEncoder(
      getUpdateSplineParametersParamsWithOrderingEncoder(),
      [getConstantEncoder(DISCRIMINANTS.UPDATE_SPLINE_PARAMETERS)]
    );

export const getUpdateSplineParametersWithOrderingDecoder =
  (): Decoder<UpdateSplineParametersParamsWithOrdering> =>
    getHiddenPrefixDecoder(
      getUpdateSplineParametersParamsWithOrderingDecoder(),
      [getConstantDecoder(DISCRIMINANTS.UPDATE_SPLINE_PARAMETERS)]
    );

export const getUpdateSplineParametersWithOrderingCodec =
  (): Codec<UpdateSplineParametersParamsWithOrdering> =>
    combineCodec(
      getUpdateSplineParametersWithOrderingEncoder(),
      getUpdateSplineParametersWithOrderingDecoder()
    );

const getPositionSizeLimitEncoder = (): Encoder<PositionSizeLimit> =>
  getDiscriminatedUnionEncoder([
    ["Disabled", getStructEncoder([])],
    [
      "Limit",
      getStructEncoder([
        [
          "value",
          getStructEncoder([
            ["long", getU32Encoder()],
            ["short", getU32Encoder()],
          ]),
        ],
      ]),
    ],
  ]) as unknown as Encoder<PositionSizeLimit>;

const getPositionSizeLimitDecoder = (): Decoder<PositionSizeLimit> =>
  getDiscriminatedUnionDecoder([
    ["Disabled", getStructDecoder([])],
    [
      "Limit",
      getStructDecoder([
        [
          "value",
          getStructDecoder([
            ["long", getU32Decoder()],
            ["short", getU32Decoder()],
          ]),
        ],
      ]),
    ],
  ]) as unknown as Decoder<PositionSizeLimit>;

export const getUpdateSplinePositionLimitsConfigParamsEncoder =
  (): Encoder<UpdateSplinePositionLimitsConfigParams> =>
    getStructEncoder([
      [
        "maxPositionSize",
        getOptionToNullEncoder(getPositionSizeLimitEncoder()),
      ],
      ["leverageDecreaseInBps", getOptionToNullEncoder(getU32Encoder())],
    ]);

export const getUpdateSplinePositionLimitsConfigParamsDecoder =
  (): Decoder<UpdateSplinePositionLimitsConfigParams> =>
    getStructDecoder([
      [
        "maxPositionSize",
        getOptionToNullDecoder(getPositionSizeLimitDecoder()),
      ],
      ["leverageDecreaseInBps", getOptionToNullDecoder(getU32Decoder())],
    ]) as unknown as Decoder<UpdateSplinePositionLimitsConfigParams>;

export const getUpdateSplinePositionLimitsConfigParamsCodec =
  (): Codec<UpdateSplinePositionLimitsConfigParams> =>
    combineCodec(
      getUpdateSplinePositionLimitsConfigParamsEncoder(),
      getUpdateSplinePositionLimitsConfigParamsDecoder()
    );

export const getUpdateSplinePositionLimitsConfigEncoder =
  (): Encoder<UpdateSplinePositionLimitsConfigParams> =>
    getHiddenPrefixEncoder(getUpdateSplinePositionLimitsConfigParamsEncoder(), [
      getConstantEncoder(DISCRIMINANTS.UPDATE_SPLINE_POSITION_LIMITS_CONFIG),
    ]);

export const getUpdateSplinePositionLimitsConfigDecoder =
  (): Decoder<UpdateSplinePositionLimitsConfigParams> =>
    getHiddenPrefixDecoder(getUpdateSplinePositionLimitsConfigParamsDecoder(), [
      getConstantDecoder(DISCRIMINANTS.UPDATE_SPLINE_POSITION_LIMITS_CONFIG),
    ]);

export const getUpdateSplinePositionLimitsConfigCodec =
  (): Codec<UpdateSplinePositionLimitsConfigParams> =>
    combineCodec(
      getUpdateSplinePositionLimitsConfigEncoder(),
      getUpdateSplinePositionLimitsConfigDecoder()
    );
