import type { Authority } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface TransferSolParams {
  /** The funding wallet. Signs and is debited. */
  source: Authority;
  /** The recipient. Any account may receive lamports. */
  destination: Address;
  lamports: bigint;
}

export type TransferSolIx = InstructionsWithAccountsAndData;
export type TransferSolAccounts = readonly AccountMeta[];
