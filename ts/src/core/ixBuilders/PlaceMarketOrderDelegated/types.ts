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
import type { ImmediateOrCancelOrderPacket } from "@/primitives/OrderPacket";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { AccountMeta, Address } from "@solana/kit";

export interface PlaceMarketOrderDelegatedParams extends PhoenixInstructionAddressOverrides {
  traderWallet: Authority; // Signing wallet.
  // Permission account, or traderWallet when signing as trader/primary position authority.
  permissionAccount: Address;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  orderPacket: ImmediateOrCancelOrderPacket;
}

export type PlaceMarketOrderDelegatedIx = InstructionsWithAccountsAndData;

export type PlaceMarketOrderDelegatedAccounts = readonly AccountMeta[];
