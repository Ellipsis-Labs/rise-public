export type { SpotCollateralMetadata } from "./types";

export {
  SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP,
  SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET,
  SPOT_COLLATERAL_FLAG_IS_ACTIVE,
  getSpotCollateralAssetId,
} from "./types";

export { getSpotCollateralMetadataDecoder } from "./codec";
