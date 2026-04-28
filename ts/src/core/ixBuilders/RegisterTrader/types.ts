import type { Authority, TraderAddress } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { AccountMeta } from "@solana/kit";

export interface RegisterTraderParams extends PhoenixInstructionAddressOverrides {
  payer: Authority; // Payer wallet (signer)
  trader: Authority; // Trader wallet (signer)
  traderAccount: TraderAddress; // Trader account (derived from trader wallet)
  maxPositions: bigint; // Maximum number of positions (u64)
  traderPdaIndex: number; // Trader PDA index (u8)
  traderSubaccountIndex: number; // Trader subaccount index (u8)
}

export type RegisterTraderAccounts = readonly AccountMeta[];
export type RegisterTraderIx = InstructionsWithAccountsAndData;
