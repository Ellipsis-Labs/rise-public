import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PlaceAttachedConditionalOrderData,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface PlaceAttachedConditionalOrderParams
  extends
    PhoenixInstructionAddressOverrides,
    PlaceAttachedConditionalOrderData {
  traderAccount: TraderAddress;
  traderWallet: Authority;
  orderbook: MarketAddress;
  traderConditionalOrders: Address;
  payer: Authority;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

export type PlaceAttachedConditionalOrderIx = InstructionsWithAccountsAndData;

export type PlaceAttachedConditionalOrderAccounts = readonly AccountMeta[];
