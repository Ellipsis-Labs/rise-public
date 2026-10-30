import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { Authority, TraderAddress } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

/**
 * A trader preference, named after the capability it allows. Each value is
 * the index of the trader preference bit backing it.
 */
export enum TraderPreferenceToggleTarget {
  CollateralSweep = 0,
  PositionAuthoritySwap = 1,
}

/** `set: true` enables the capability and clears its preference bit. */
export interface TraderPreferenceToggle {
  target: TraderPreferenceToggleTarget;
  set: boolean;
}

export interface UpdateTraderPreferenceBitsData {
  /** Preferences absent from `toggles` are left unchanged. */
  toggles: TraderPreferenceToggle[];
}

export interface UpdateTraderPreferenceBitsParams
  extends PhoenixInstructionAddressOverrides, UpdateTraderPreferenceBitsData {
  traderWallet: Authority;
  traderAccount: TraderAddress;
}

export type UpdateTraderPreferenceBitsIx = InstructionsWithAccountsAndData;

export type UpdateTraderPreferenceBitsAccounts = readonly AccountMeta[];
