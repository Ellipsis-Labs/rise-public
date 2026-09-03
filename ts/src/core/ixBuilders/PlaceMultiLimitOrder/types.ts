import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  MultipleOrderPacket,
  MultipleOrderPacketV2,
  PerpAssetMapAddress,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface PlaceMultiLimitOrderParams extends PhoenixInstructionAddressOverrides {
  trader: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  multipleOrderPacket: MultipleOrderPacket;
}

export interface PlaceMultiLimitOrderV2Params extends Omit<
  PlaceMultiLimitOrderParams,
  "multipleOrderPacket"
> {
  multipleOrderPacket: MultipleOrderPacketV2;
}

export type PlaceMultiLimitOrderIx = InstructionsWithAccountsAndData;

export type PlaceMultiLimitOrderAccounts = readonly AccountMeta[];
