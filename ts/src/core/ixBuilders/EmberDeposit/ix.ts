import {
  EMBER_PROGRAM_ADDRESS,
  SPL_TOKEN_PROGRAM_ADDRESS,
} from "@/core/constants";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getEmberDepositEncoder } from "./codec";
import type {
  EmberDepositAccounts,
  EmberDepositIx,
  EmberDepositParams,
} from "./types";

/**
 * Build Ember Deposit instruction
 * This creates the instruction that deposits USDC to the Ember program
 * and receives wrapped Phoenix tokens in return
 */
export const buildEmberDepositIx = (
  params: EmberDepositParams
): EmberDepositIx => {
  validate(params);

  const data = getEmberDepositEncoder().encode(params.amount);

  // Account order matches the Rust implementation:
  // 1. owner (signer, readonly)
  // 2. state (readonly)
  // 3. input_mint (readonly)
  // 4. output_mint (writable)
  // 5. input_token_account (writable)
  // 6. output_token_account (writable)
  // 7. vault (writable)
  // 8. spl_token (readonly)
  const accounts: EmberDepositAccounts = [
    generateReadonlySignerAccount(params.owner),
    generateReadonlyAccount(params.emberState),
    generateReadonlyAccount(params.inputMint),
    generateWritableAccount(params.outputMint),
    generateWritableAccount(params.inputTokenAccount),
    generateWritableAccount(params.outputTokenAccount),
    generateWritableAccount(params.emberVault),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress: EMBER_PROGRAM_ADDRESS,
    accounts,
    data,
  };
};

const validate = (params: EmberDepositParams) => {
  if (!params.owner) {
    throw new Error("Owner wallet is required");
  }
  if (!params.inputMint) {
    throw new Error("Input mint address is required");
  }
  if (!params.outputMint) {
    throw new Error("Output mint address is required");
  }
  if (!params.inputTokenAccount) {
    throw new Error("Input token account is required");
  }
  if (!params.outputTokenAccount) {
    throw new Error("Output token account is required");
  }
  if (!params.emberState) {
    throw new Error("Ember state address is required");
  }
  if (!params.emberVault) {
    throw new Error("Ember vault address is required");
  }
  if (params.amount === undefined || params.amount === null) {
    throw new Error("Amount is required");
  }
  if (params.amount <= 0n) {
    throw new Error("Amount must be greater than 0");
  }
};
