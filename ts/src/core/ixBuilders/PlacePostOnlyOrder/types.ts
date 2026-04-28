import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  PostOnlyOrderPacket,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface PlacePostOnlyOrderParams extends PhoenixInstructionAddressOverrides {
  trader: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  orderPacket: PostOnlyOrderPacket;
}

export type PlacePostOnlyOrderIx = InstructionsWithAccountsAndData;

export type PlacePostOnlyOrderAccounts = readonly AccountMeta[];
