import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getOnboardTraderDelegatedEncoder } from "./codec";
import type {
  OnboardTraderDelegatedAccounts,
  OnboardTraderDelegatedIx,
  OnboardTraderDelegatedParams,
} from "./types";

export const buildOnboardTraderDelegatedIx = (
  params: OnboardTraderDelegatedParams
): OnboardTraderDelegatedIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: OnboardTraderDelegatedAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.authority),
    generateWritableAccount(params.permissionAccount),
    generateWritableAccount(params.traderAccount),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data: getOnboardTraderDelegatedEncoder().encode(undefined),
  };
};

const validate = (params: OnboardTraderDelegatedParams) => {
  if (!params.authority) {
    throw new Error("Authority is required");
  }
  if (!params.permissionAccount) {
    throw new Error("Permission account is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error(
      "Global trader index array is required and must not be empty"
    );
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error(
      "Active trader buffer array is required and must not be empty"
    );
  }
};
