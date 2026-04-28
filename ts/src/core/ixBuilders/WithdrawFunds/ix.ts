import {
  getPhoenixInstructionAddresses,
  SPL_TOKEN_PROGRAM_ADDRESS,
} from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getWithdrawFundsEncoder } from "./codec";
import type {
  WithdrawFundsAccounts,
  WithdrawFundsIx,
  WithdrawFundsParams,
} from "./types";

export const buildWithdrawFundsIx = (
  params: WithdrawFundsParams
): WithdrawFundsIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getWithdrawFundsEncoder().encode(params.amount);

  const accounts: WithdrawFundsAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // WithdrawFundsInstructionGroupAccounts
    generateWritableAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.trader),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.perpAssetMap),
    generateWritableAccount(params.globalVault),
    generateWritableAccount(params.destinationTokenAccount),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS), // SPL Token Program
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.withdrawQueue),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: WithdrawFundsParams) => {
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.mint) {
    throw new Error("Mint address is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (!params.destinationTokenAccount) {
    throw new Error("Destination token account is required");
  }
  if (!params.globalVault) {
    throw new Error("Global vault address is required");
  }
  if (!params.withdrawQueue) {
    throw new Error("Withdraw queue is required");
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error(
      "Active trader buffer array is required and must not be empty"
    );
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error(
      "Global trader index array is required and must not be empty"
    );
  }
  if (params.amount === undefined || params.amount === null) {
    throw new Error("Amount is required");
  }
  if (params.amount <= 0n) {
    throw new Error("Amount must be greater than 0");
  }
};
