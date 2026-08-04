import type { Decoder } from "@solana/kit";
import {
  getStructDecoder,
  getU32Decoder,
  getU64Decoder,
  getU8Decoder,
  transformDecoder,
} from "@solana/kit";
import { getQuoteLotsDecoder } from "@/primitives/_numberTypes";
import { getFixedArrayDecoder, getMintAddressDecoder } from "../internal";
import type { SpotCollateralMetadata } from "./types";
import {
  SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP,
  SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET,
  SPOT_COLLATERAL_FLAG_IS_ACTIVE,
} from "./types";

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
        ["_padding", getFixedArrayDecoder(getU64Decoder, 26)],
      ]),
      ({ _paddingFlags, _padding, ...metadata }): SpotCollateralMetadata => ({
        ...metadata,
        isActive: (metadata.flags & SPOT_COLLATERAL_FLAG_IS_ACTIVE) !== 0,
        hasPerpAsset:
          (metadata.flags & SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET) !== 0,
        positionAuthoritySwapDisabled:
          (metadata.flags &
            SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP) !==
          0,
      })
    );
