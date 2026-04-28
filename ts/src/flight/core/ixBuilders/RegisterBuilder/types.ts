import type { ResolveFlightInstructionAddressesInput } from "@/flight/core/constants";
import type { Authority } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface RegisterBuilderParams extends ResolveFlightInstructionAddressesInput {
  traderAuthority: Authority;
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
  feeBps: bigint;
}

export type RegisterBuilderAccounts = readonly AccountMeta[];
export type RegisterBuilderIx = InstructionsWithAccountsAndData;
