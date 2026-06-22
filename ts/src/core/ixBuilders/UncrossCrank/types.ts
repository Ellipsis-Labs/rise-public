import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  SplineCollectionAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface UncrossCrankParams extends PhoenixInstructionAddressOverrides {
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  matchLimit: bigint;
}

export interface UncrossCrankInstruction {
  matchLimit: bigint;
}

export type UncrossCrankIx = InstructionsWithAccountsAndData;

export type UncrossCrankAccounts = readonly AccountMeta[];
