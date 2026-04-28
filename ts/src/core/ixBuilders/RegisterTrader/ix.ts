import {
  getPhoenixInstructionAddresses,
  SYSTEM_PROGRAM_ADDRESS,
} from "@/core/constants";
import {
  generateReadonlyAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { getRegisterTraderInstructionEncoder } from "./codec";
import type {
  RegisterTraderAccounts,
  RegisterTraderIx,
  RegisterTraderParams,
} from "./types";

export const buildRegisterTraderIx = (
  params: RegisterTraderParams
): RegisterTraderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getRegisterTraderInstructionEncoder().encode({
    maxPositions: params.maxPositions,
    traderPdaIndex: params.traderPdaIndex,
    subaccountIndex: params.traderSubaccountIndex,
  });

  const accounts: RegisterTraderAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // RegisterTraderInstructionGroupAccounts
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableSignerAccount(params.payer),
    generateReadonlyAccount(params.trader),
    generateWritableAccount(params.traderAccount),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS), // System Program
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: RegisterTraderParams) => {
  if (!params.payer) {
    throw new Error("Payer wallet is required");
  }
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (params.maxPositions <= 0n) {
    throw new Error("Max positions must be greater than 0");
  }
  if (params.maxPositions > 128n) {
    throw new Error("Max positions cannot exceed 128");
  }
  if (params.traderPdaIndex < 0 || params.traderPdaIndex > 255) {
    throw new Error("Trader PDA index must be between 0 and 255");
  }
};
