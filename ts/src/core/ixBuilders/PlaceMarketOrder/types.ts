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
import type { AccountMeta } from "@solana/kit";

export interface PlaceMarketOrderParams extends PhoenixInstructionAddressOverrides {
  trader: Authority; // Trader wallet (signer)
  traderAccount: TraderAddress; // Trader account
  perpAssetMap: PerpAssetMapAddress;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  activeTraderBuffer: ActiveTraderBufferAddressArray; // Array of active trader buffer addresses (header, then N arenas)
  globalTraderIndex: GlobalTraderIndexAddressArray; // Array of global trader index addresses (header, then N arenas)
  orderPacket: ImmediateOrCancelOrderPacket;
}

export type PlaceMarketOrderIx = InstructionsWithAccountsAndData;

export type PlaceMarketOrderAccounts = readonly AccountMeta[];
