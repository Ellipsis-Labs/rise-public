import type {
  Authority,
  EmberStateAddress,
  EmberVaultAddress,
  MintAddress,
  TokenAccountAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface EmberDepositParams {
  owner: Authority; // Owner wallet (signer)
  inputMint: MintAddress; // Input token mint (USDC)
  outputMint: MintAddress; // Output token mint (Phoenix)
  inputTokenAccount: TokenAccountAddress; // Owner's input token account
  outputTokenAccount: TokenAccountAddress; // Owner's output token account
  emberState: EmberStateAddress; // Ember state PDA
  emberVault: EmberVaultAddress; // Ember vault PDA
  amount: bigint; // Amount to deposit in lamports
}

export type EmberDepositIx = InstructionsWithAccountsAndData;

export type EmberDepositAccounts = readonly AccountMeta[];
