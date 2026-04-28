import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  PlacePositionConditionalOrderData,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface PlacePositionConditionalOrderParams
  extends
    PhoenixInstructionAddressOverrides,
    PlacePositionConditionalOrderData {
  payer: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  traderWallet: Authority;
  traderConditionalOrders: Address;
}

export type PlacePositionConditionalOrderIx = InstructionsWithAccountsAndData;

export type PlacePositionConditionalOrderAccounts = readonly AccountMeta[];
