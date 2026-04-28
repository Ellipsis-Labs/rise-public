import type {
  Authority,
  EmberStateAddress,
  EmberVaultAddress,
  MintAddress,
  TokenAccountAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface EmberWithdrawParams {
  owner: Authority; // Owner wallet (signer)
  inputMint: MintAddress; // Input token mint (Phoenix)
  outputMint: MintAddress; // Output token mint (USDC)
  inputTokenAccount: TokenAccountAddress; // Owner's input token account (Phoenix ATA)
  outputTokenAccount: TokenAccountAddress; // Owner's output token account (USDC ATA)
  emberState: EmberStateAddress; // Ember state PDA
  emberVault: EmberVaultAddress; // Ember vault PDA
  amount: bigint | null; // Amount to withdraw in lamports (null for full withdrawal)
}

export type EmberWithdrawIx = InstructionsWithAccountsAndData;

export type EmberWithdrawAccounts = readonly AccountMeta[];
