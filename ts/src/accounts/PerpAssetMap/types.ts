import type { Symbol } from "@/primitives/Symbol";
import type { SpotCollateralAsset } from "../SpotCollateralMetadata";
import type {
  PerpAssetMetadata,
  SequenceNumber,
  ShortEntries,
} from "../internal";

export interface PerpAssetMap {
  sequenceNumber: SequenceNumber;
  numAssets: number;
  metadata: ShortEntries<Symbol, PerpAssetMetadata>;
  /** Registered SPL spot collateral assets, active or not. */
  spotCollaterals: SpotCollateralAsset[];
}
