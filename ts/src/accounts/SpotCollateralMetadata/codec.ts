import type { Decoder } from "@solana/kit";
import {
  getI8Decoder,
  getStructDecoder,
  getU32Decoder,
  getU64Decoder,
  getU8Decoder,
  transformDecoder,
} from "@solana/kit";
import { getQuoteLotsDecoder } from "@/primitives/_numberTypes";
import { getSymbolDecoder } from "@/primitives/Symbol";
import { getFixedArrayDecoder, getMintAddressDecoder } from "../internal";
import type {
  SpotAssetOraclePrice,
  SpotCollateralAsset,
  SpotCollateralMetadata,
} from "./types";
import {
  SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP,
  SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET,
  SPOT_COLLATERAL_FLAG_IS_ACTIVE,
} from "./types";

const getSpotAssetOraclePriceDecoder = (): Decoder<SpotAssetOraclePrice> =>
  transformDecoder(
    getStructDecoder([
      ["sequenceNumber", getU64Decoder()],
      ["lastUpdateSlot", getU64Decoder()],
      ["ticks", getU64Decoder()],
      ["priceExpo", getU8Decoder()],
      ["baseLotDecimals", getI8Decoder()],
      ["_writerConfig", getFixedArrayDecoder(getU8Decoder, 38)],
      ["staleThresholdSlots", getU64Decoder()],
      ["tickSize", getU64Decoder()],
    ]),
    ({ _writerConfig, ...price }): SpotAssetOraclePrice => price
  );

/**
 * Decodes the 320-byte spot collateral metadata embedded in the global
 * configuration account. All-zero bytes decode as an inactive asset, which is
 * what exchanges that never configured spot collateral carry.
 */
export const getSpotCollateralMetadataDecoder =
  (): Decoder<SpotCollateralMetadata> =>
    transformDecoder(
      getStructDecoder([
        ["mintAddress", getMintAddressDecoder()],
        ["decimals", getU32Decoder()],
        ["perpAssetIndex", getU32Decoder()],
        ["maxPerTraderBalance", getU64Decoder()],
        ["maxGlobalBalance", getU64Decoder()],
        ["currGlobalBalance", getU64Decoder()],
        ["minMarginDiscountBps", getU32Decoder()],
        ["maxMarginDiscountBps", getU32Decoder()],
        ["maxLiquidationDiscountBps", getU32Decoder()],
        ["minLiquidationSlippageBps", getU32Decoder()],
        ["maxLiquidationSize", getU64Decoder()],
        ["postLiquidationBuffer", getQuoteLotsDecoder()],
        ["quoteLotCollateralShortfallBuffer", getQuoteLotsDecoder()],
        ["flags", getU8Decoder()],
        ["_paddingFlags", getFixedArrayDecoder(getU8Decoder, 7)],
        ["oraclePrice", getSpotAssetOraclePriceDecoder()],
        ["_padding", getFixedArrayDecoder(getU64Decoder, 16)],
      ]),
      ({
        _paddingFlags,
        _padding,
        oraclePrice,
        ...metadata
      }): SpotCollateralMetadata => {
        const hasPerpAsset =
          (metadata.flags & SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET) !== 0;
        return {
          ...metadata,
          oraclePrice: hasPerpAsset ? null : oraclePrice,
          isActive: (metadata.flags & SPOT_COLLATERAL_FLAG_IS_ACTIVE) !== 0,
          hasPerpAsset,
          positionAuthoritySwapDisabled:
            (metadata.flags &
              SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP) !==
            0,
        };
      }
    );

/**
 * Decodes the value of a perp asset map slot tagged as a spot collateral
 * collection into its initialized assets (zero asset index = uninitialized).
 */
export const getSpotCollateralCollectionDecoder = (): Decoder<
  SpotCollateralAsset[]
> =>
  transformDecoder(
    getStructDecoder([
      ["_shortMapMetadata", getFixedArrayDecoder(getU8Decoder, 8)],
      [
        "entries",
        getFixedArrayDecoder(
          () =>
            getStructDecoder([
              ["symbol", getSymbolDecoder()],
              ["assetIndex", getU32Decoder()],
              ["_padding", getFixedArrayDecoder(getU8Decoder, 4)],
              ["metadata", getSpotCollateralMetadataDecoder()],
            ]),
          4
        ),
      ],
      ["_reserved", getFixedArrayDecoder(getU64Decoder, 22)],
      ["_padding", getFixedArrayDecoder(getU8Decoder, 7)],
      ["_assetMapVariant", getU8Decoder()],
    ]),
    ({ entries }) =>
      entries
        .filter((entry) => entry.assetIndex !== 0)
        .map(({ _padding, ...asset }): SpotCollateralAsset => asset)
  );
