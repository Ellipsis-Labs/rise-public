import type {
  GlobalTraderIndexAddressArray,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { AccountMeta, Address } from "@solana/kit";

export interface SyncParentToChildParams extends PhoenixInstructionAddressOverrides {
  traderWallet: Address; // Trader wallet (authority for both traders, NOT a signer - permissionless)
  parentTraderAccount: TraderAddress; // Parent (cross) trader account (subaccount_index = 0)
  childTraderAccount: TraderAddress; // Child (isolated) trader subaccount
  globalTraderIndex: GlobalTraderIndexAddressArray; // Array of global trader index addresses (header, then N arenas)
}

export type SyncParentToChildIx = InstructionsWithAccountsAndData;

export type SyncParentToChildAccounts = readonly AccountMeta[];
