import {
  SYSTEM_PROGRAM_ADDRESS,
  getPhoenixInstructionAddresses,
} from "@/core/constants";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { getCancelStopLossEncoder } from "./codec";
import type {
  CancelStopLossAccounts,
  CancelStopLossIx,
  CancelStopLossParams,
} from "./types";

export const buildCancelStopLossIx = (
  params: CancelStopLossParams
): CancelStopLossIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getCancelStopLossEncoder().encode({
    executionDirection: params.executionDirection,
  });

  const accounts: CancelStopLossAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableSignerAccount(params.funder),
    generateReadonlyAccount(params.traderAccount),
    generateReadonlySignerAccount(params.traderWallet),
    generateWritableAccount(params.stopLossAccount),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: CancelStopLossParams) => {
  if (!params.funder) {
    throw new Error("Funder is required");
  }
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.stopLossAccount) {
    throw new Error("Stop loss account is required");
  }
  if (params.executionDirection === undefined) {
    throw new Error("Execution direction is required");
  }
};
