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
import type { CancelId } from "@/primitives/CancelId";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface CancelOrdersById {
  orderIds: CancelId[];
}

export interface CancelOrdersByIdParams
  extends CancelOrdersById, PhoenixInstructionAddressOverrides {
  traderWallet: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
}

export type CancelOrdersByIdIx = InstructionsWithAccountsAndData;

export type CancelOrdersByIdAccounts = readonly AccountMeta[];
