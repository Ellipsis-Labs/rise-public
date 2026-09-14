import type { ResolveFlightInstructionAddressesInput } from "@/flight/core/constants";
import type { Authority, TraderAddress } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface ProxyInstructionParams extends ResolveFlightInstructionAddressesInput {
  builderAuthority: Authority;
  builderTraderAccount: TraderAddress;
  traderWallet: Authority;
  feeBpsOverride?: bigint | null;
  /**
   * Phoenix root authority used to derive the collateral-transfer permission
   * account; set iff the signer is a position authority — presence appends
   * the collateral-transfer tail accounts. This is the single source of
   * truth for the tail (never inferred from the inner instruction): leave it
   * unset for owner-signed orders — including owner-signed
   * `PlaceMarketOrderDelegated`, which Flight detects on-chain and settles
   * via the plain transfer — because the tail write-locks a global
   * permission account.
   */
  rootAuthority?: Authority;
  innerInstruction: InstructionsWithAccountsAndData;
}

export type ProxyInstructionAccounts = readonly AccountMeta[];
export type ProxyInstructionIx = InstructionsWithAccountsAndData;
