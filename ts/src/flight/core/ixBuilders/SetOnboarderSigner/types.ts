import type { ResolveFlightInstructionAddressesInput } from "@/flight/core/constants";
import type { Authority } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface SetOnboarderSignerParams extends ResolveFlightInstructionAddressesInput {
  builderAuthority: Authority;
  signer: Address;
}

export type SetOnboarderSignerAccounts = readonly AccountMeta[];
export type SetOnboarderSignerIx = InstructionsWithAccountsAndData;
