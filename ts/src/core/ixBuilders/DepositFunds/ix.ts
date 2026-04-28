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
import { getDepositFundsEncoder } from "./codec";
import type {
  DepositFundsAccounts,
  DepositFundsIx,
  DepositFundsParams,
} from "./types";

export const buildDepositFundsIx = (
  params: DepositFundsParams
): DepositFundsIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getDepositFundsEncoder().encode(params.amount);

  const accounts: DepositFundsAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // DepositFundsInstructionGroupAccounts
    generateWritableAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.trader),
    generateWritableAccount(params.traderTokenAccount),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.globalVault),
    generateReadonlyAccount(SPL_TOKEN_PROGRAM_ADDRESS), // SPL Token Program
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    ...(params.permissionAccount
      ? [generateWritableAccount(params.permissionAccount)]
      : []),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: DepositFundsParams) => {
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.mint) {
    throw new Error("Mint address is required");
  }
  if (!params.traderTokenAccount) {
    throw new Error("Trader token account is required");
  }
  if (!params.globalVault) {
    throw new Error("Global vault address is required");
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
