import { getPhoenixInstructionAddresses } from "@/core/constants";
import { DISCRIMINANTS } from "@/core/discriminants";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import type {
  DelegateTraderAccounts,
  DelegateTraderIx,
  DelegateTraderParams,
} from "./types";

export const buildDelegateTraderIx = (
  params: DelegateTraderParams
): DelegateTraderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: DelegateTraderAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.traderWallet),
    generateWritableAccount(params.traderAccount),
    generateReadonlyAccount(params.newPositionAuthority),
  ] as const;

  return {
    programAddress,
    accounts,
    data: DISCRIMINANTS.DELEGATE_TRADER,
  };
};

const validate = (params: DelegateTraderParams) => {
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.newPositionAuthority) {
    throw new Error("New position authority is required");
  }
};
