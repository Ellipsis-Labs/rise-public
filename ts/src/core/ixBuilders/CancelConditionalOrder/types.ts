import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  Authority,
  ConditionalOrderIndex,
  MarketAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface CancelConditionalOrderParams extends PhoenixInstructionAddressOverrides {
  traderAccount: TraderAddress;
  traderWallet: Authority;
  orderbook: MarketAddress;
  traderConditionalOrders: Address;
  conditionalOrderIndex: ConditionalOrderIndex;
  disableFirst: boolean;
  disableSecond: boolean;
}

export interface CancelConditionalOrderData {
  conditionalOrderIndex: ConditionalOrderIndex;
  disableFirst: boolean;
  disableSecond: boolean;
}

export type CancelConditionalOrderIx = InstructionsWithAccountsAndData;

export type CancelConditionalOrderAccounts = readonly AccountMeta[];
