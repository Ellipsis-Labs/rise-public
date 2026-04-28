import { getPhoenixInstructionAddresses } from "@/core/constants";
import { DISCRIMINANTS } from "@/core/discriminants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import type {
  TransferCollateralChildToParentAccounts,
  TransferCollateralChildToParentIx,
  TransferCollateralChildToParentParams,
} from "./types";

export const buildTransferCollateralChildToParentIx = (
  params: TransferCollateralChildToParentParams
): TransferCollateralChildToParentIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: TransferCollateralChildToParentAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // TransferCollateralChildToParentInstructionGroupAccounts
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlyAccount(params.trader), // trader_wallet
    generateWritableAccount(params.childTraderAccount),
    generateWritableAccount(params.parentTraderAccount),
    generateReadonlyAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data: DISCRIMINANTS.TRANSFER_COLLATERAL_CHILD_TO_PARENT,
  };
};

const validate = (params: TransferCollateralChildToParentParams) => {
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.childTraderAccount) {
    throw new Error("Child trader account is required");
  }
  if (!params.parentTraderAccount) {
    throw new Error("Parent trader account is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
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
  if (params.childTraderAccount === params.parentTraderAccount) {
    throw new Error("Child and parent trader accounts must be different");
  }
};
