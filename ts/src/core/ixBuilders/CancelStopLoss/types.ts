import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { Authority, Direction, TraderAddress } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface CancelStopLossParams extends PhoenixInstructionAddressOverrides {
  funder: Authority;
  traderWallet: Authority;
  traderAccount: TraderAddress;
  stopLossAccount: Address;
  executionDirection: Direction;
}

export interface CancelStopLossData {
  executionDirection: Direction;
}

export type CancelStopLossIx = InstructionsWithAccountsAndData;

export type CancelStopLossAccounts = readonly AccountMeta[];
