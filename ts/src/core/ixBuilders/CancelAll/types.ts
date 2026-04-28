import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface CancelAllParams extends PhoenixInstructionAddressOverrides {
  traderWallet: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
}

export type CancelAllIx = InstructionsWithAccountsAndData;

export type CancelAllAccounts = readonly AccountMeta[];
