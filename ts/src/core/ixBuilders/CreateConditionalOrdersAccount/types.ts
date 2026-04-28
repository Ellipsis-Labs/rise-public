import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { Authority, TraderAddress } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface CreateConditionalOrdersAccountParams extends PhoenixInstructionAddressOverrides {
  payer: Authority;
  traderWallet: Authority;
  traderAccount: TraderAddress;
  traderConditionalOrders: Address;
  capacity: number;
}

export type CreateConditionalOrdersAccountIx = InstructionsWithAccountsAndData;

export type CreateConditionalOrdersAccountAccounts = readonly AccountMeta[];
