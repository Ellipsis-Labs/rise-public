import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export const MAX_SPLINE_REGIONS = 10;

export interface RegisterSplineParams extends PhoenixInstructionAddressOverrides {
  payer: Authority;
  authority: Authority;
  permissionAccount?: Address;
  marketAccount: MarketAddress;
  splineCollection: SplineCollectionAddress;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

export interface DeactivateSplineParams extends PhoenixInstructionAddressOverrides {
  authority: Authority;
  permissionAccount?: Address;
  marketAccount: MarketAddress;
  splineCollection: SplineCollectionAddress;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

export interface SplineUpdateAccounts extends PhoenixInstructionAddressOverrides {
  signer: Authority;
  traderAccount: TraderAddress;
  splineCollection: SplineCollectionAddress;
}

export interface TickRegionParams {
  startOffset: bigint;
  endOffset: bigint;
  density: number;
  topLevelHiddenTakeSize: number;
  lifespan: bigint;
}

export interface UpdateSplinePriceParams {
  newMidPrice: bigint;
  userUpdateSlot: bigint | null;
  refreshRegions: boolean;
}

export interface UpdateSplinePriceParamsWithOrdering extends UpdateSplinePriceParams {
  userSequenceNumber: bigint;
  clientOrderId?: Uint8Array;
  overrideSequenceNumber?: boolean;
}

export interface UpdateSplineParametersParamsWithOrdering {
  bidRegions: TickRegionParams[];
  askRegions: TickRegionParams[];
  refreshRegions: boolean;
  userSequenceNumber: bigint;
  clientOrderId?: Uint8Array;
  overrideSequenceNumber?: boolean;
}

export interface PositionSizeLimits {
  long: number;
  short: number;
}

export type PositionSizeLimit =
  | { __kind: "Disabled" }
  | { __kind: "Limit"; value: PositionSizeLimits };

export interface UpdateSplinePositionLimitsConfigParams {
  maxPositionSize: PositionSizeLimit | null;
  leverageDecreaseInBps: number | null;
}

export type RegisterSplineIx = InstructionsWithAccountsAndData;
export type DeactivateSplineIx = InstructionsWithAccountsAndData;
export type UpdateSplinePriceIx = InstructionsWithAccountsAndData;
export type UpdateSplinePriceWithOrderingIx = InstructionsWithAccountsAndData;
export type UpdateSplineParametersWithOrderingIx =
  InstructionsWithAccountsAndData;
export type UpdateSplinePositionLimitsConfigIx =
  InstructionsWithAccountsAndData;

export type RegisterSplineAccounts = readonly AccountMeta[];
export type DeactivateSplineAccounts = readonly AccountMeta[];
export type SplineUpdateAccountMetas = readonly AccountMeta[];
