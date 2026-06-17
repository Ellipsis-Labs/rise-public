import type { ResolveFlightInstructionAddressesInput } from "@/flight/core/constants";
import type { Authority, TraderAddress } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface ProxyInstructionParams extends ResolveFlightInstructionAddressesInput {
  builderAuthority: Authority;
  builderTraderAccount: TraderAddress;
  traderWallet: Authority;
  feeBpsOverride?: bigint | null;
  innerInstruction: InstructionsWithAccountsAndData;
}

export type ProxyInstructionAccounts = readonly AccountMeta[];
export type ProxyInstructionIx = InstructionsWithAccountsAndData;
