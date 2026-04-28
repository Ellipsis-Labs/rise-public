import { getPhoenixInstructionAddresses } from "@/core/constants";
import { DISCRIMINANTS } from "@/core/discriminants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import type {
  SyncParentToChildAccounts,
  SyncParentToChildIx,
  SyncParentToChildParams,
} from "./types";

export const buildSyncParentToChildIx = (
  params: SyncParentToChildParams
): SyncParentToChildIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: SyncParentToChildAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // SyncParentToChildInstructionGroupAccounts
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlyAccount(params.traderWallet), // trader_wallet (not a signer - permissionless)
    generateReadonlyAccount(params.parentTraderAccount),
    generateWritableAccount(params.childTraderAccount),
    ...generateArenaAccounts(params.globalTraderIndex),
  ] as const;

  return {
    programAddress,
    accounts,
    data: DISCRIMINANTS.SYNC_PARENT_TO_CHILD,
  };
};

const validate = (params: SyncParentToChildParams) => {
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.parentTraderAccount) {
    throw new Error("Parent trader account is required");
  }
  if (!params.childTraderAccount) {
    throw new Error("Child trader account is required");
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error(
      "Global trader index array is required and must not be empty"
    );
  }
  if (params.childTraderAccount === params.parentTraderAccount) {
    throw new Error("Child and parent trader accounts must be different");
  }
};
