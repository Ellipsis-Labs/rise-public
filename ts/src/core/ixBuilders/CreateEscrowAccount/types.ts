import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { Authority } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface CreateEscrowAccountParams extends PhoenixInstructionAddressOverrides {
  payer: Authority;
  traderWallet: Authority;
  escrowPda: Address;
  capacity: bigint;
}

export type CreateEscrowAccountIx = InstructionsWithAccountsAndData;

export type CreateEscrowAccountAccounts = readonly AccountMeta[];
