import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  PlaceLimitOrderWithConditionalsData,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface PlaceLimitOrderWithConditionalsParams
  extends
    PhoenixInstructionAddressOverrides,
    Omit<PlaceLimitOrderWithConditionalsData, "slot"> {
  traderWallet: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  orderbook: MarketAddress;
  splineCollection: SplineCollectionAddress;
  payer: Authority;
  traderConditionalOrders: Address;
  slot?: bigint;
}

export type PlaceLimitOrderWithConditionalsIx = InstructionsWithAccountsAndData;

export type PlaceLimitOrderWithConditionalsAccounts = readonly AccountMeta[];
