import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  Side,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface CancelUpToParams extends PhoenixInstructionAddressOverrides {
  trader: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  side: Side;
  numOrdersToCancel: bigint | null;
  tickLimit: bigint | null;
}

export interface CancelUpToInstruction {
  side: Side;
  numOrdersToCancel: bigint | null;
  tickLimit: bigint | null;
}

export type CancelUpToIx = InstructionsWithAccountsAndData;

export type CancelUpToAccounts = readonly AccountMeta[];
