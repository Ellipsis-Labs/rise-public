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
import type { PerpAssetMap } from "./types";

export const getPerpAssetMapDecoder = (): Decoder<PerpAssetMap> =>
  transformDecoder(
    getHiddenPrefixDecoder(
      getStructDecoder([
        ["sequenceNumber", getSequenceNumberDecoder()],
        ["numAssets", getU16Decoder()],
        ["_padding", getFixedArrayDecoder(getU8Decoder, 6)],
        ["metadata", getPerpAssetMapMetadataEntriesDecoder()],
      ]),
      [getConstantDecoder(ACCOUNT_DISCRIMINANTS.PERP_ASSET_MAP)]
    ),
    ({ _padding, ...map }): PerpAssetMap => map
  );

export const decodePerpAssetMap = (
  bytes: Uint8Array | Readonly<Uint8Array>
): PerpAssetMap => getPerpAssetMapDecoder().decode(bytes);
