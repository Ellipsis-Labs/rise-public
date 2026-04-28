import {
  SYSTEM_PROGRAM_ADDRESS,
  getPhoenixInstructionAddresses,
} from "@/core/constants";
import {
  generateReadonlyAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { getCreateConditionalOrdersAccountEncoder } from "./codec";
import type {
  CreateConditionalOrdersAccountAccounts,
  CreateConditionalOrdersAccountIx,
  CreateConditionalOrdersAccountParams,
} from "./types";

export const buildCreateConditionalOrdersAccountIx = (
  params: CreateConditionalOrdersAccountParams
): CreateConditionalOrdersAccountIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: CreateConditionalOrdersAccountAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableSignerAccount(params.payer),
    generateReadonlyAccount(params.traderWallet),
    generateReadonlyAccount(params.traderAccount),
    generateWritableAccount(params.traderConditionalOrders),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data: getCreateConditionalOrdersAccountEncoder().encode(params.capacity),
  };
};

const validate = (params: CreateConditionalOrdersAccountParams) => {
  if (!params.payer) throw new Error("Payer is required");
  if (!params.traderWallet) throw new Error("Trader wallet is required");
  if (!params.traderAccount) throw new Error("Trader account is required");
  if (!params.traderConditionalOrders) {
    throw new Error("Trader conditional orders account is required");
  }
  if (!Number.isInteger(params.capacity) || params.capacity <= 0) {
    throw new Error("Capacity must be a positive integer");
  }
};
