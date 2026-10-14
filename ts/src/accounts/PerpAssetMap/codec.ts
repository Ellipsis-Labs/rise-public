import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import type { Decoder } from "@solana/kit";
import {
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getStructDecoder,
  getU8Decoder,
  getU16Decoder,
  transformDecoder,
} from "@solana/kit";
import {
  getFixedArrayDecoder,
  getPerpAssetMapMetadataEntriesDecoder,
  getSequenceNumberDecoder,
} from "../internal";
import { getSpotCollateralCollectionDecoder } from "../SpotCollateralMetadata";
import type { PerpAssetMap } from "./types";

export const getPerpAssetMapDecoder = (): Decoder<PerpAssetMap> =>
  transformDecoder(
    getHiddenPrefixDecoder(
      getStructDecoder([
        ["sequenceNumber", getSequenceNumberDecoder()],
        ["numAssets", getU16Decoder()],
        ["_padding", getFixedArrayDecoder(getU8Decoder, 6)],
        ["slots", getPerpAssetMapMetadataEntriesDecoder()],
      ]),
      [getConstantDecoder(ACCOUNT_DISCRIMINANTS.PERP_ASSET_MAP)]
    ),
    ({ _padding, slots, ...map }): PerpAssetMap => {
      const spotDecoder = getSpotCollateralCollectionDecoder();
      return {
        ...map,
        metadata: slots.metadata,
        spotCollaterals: slots.spotCollectionSlots.flatMap((slot) =>
          spotDecoder.decode(slot)
        ),
      };
    }
  );

export const decodePerpAssetMap = (
  bytes: Uint8Array | Readonly<Uint8Array>
): PerpAssetMap => getPerpAssetMapDecoder().decode(bytes);
