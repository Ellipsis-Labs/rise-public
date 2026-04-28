import type { ResolveFlightInstructionAddressesInput } from "@/flight/core/constants";
import type { Authority } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface UpdateFeeParams extends ResolveFlightInstructionAddressesInput {
  traderAuthority: Authority;
  feeBps: bigint;
}

export type UpdateFeeAccounts = readonly AccountMeta[];
export type UpdateFeeIx = InstructionsWithAccountsAndData;
