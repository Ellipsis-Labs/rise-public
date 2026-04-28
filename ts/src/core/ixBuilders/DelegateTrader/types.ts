import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { Authority, TraderAddress } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface DelegateTraderParams extends PhoenixInstructionAddressOverrides {
  traderWallet: Authority;
  traderAccount: TraderAddress;
  newPositionAuthority: Address;
}

export type DelegateTraderIx = InstructionsWithAccountsAndData;

export type DelegateTraderAccounts = readonly AccountMeta[];
