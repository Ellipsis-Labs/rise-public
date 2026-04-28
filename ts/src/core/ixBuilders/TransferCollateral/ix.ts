import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getTransferCollateralEncoder } from "./codec";
import type {
  TransferCollateralAccounts,
  TransferCollateralIx,
  TransferCollateralParams,
} from "./types";

export const buildTransferCollateralIx = (
  params: TransferCollateralParams
): TransferCollateralIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getTransferCollateralEncoder().encode(params.amount);

  const accounts: TransferCollateralAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.trader),
    generateWritableAccount(params.srcTraderAccount),
    generateWritableAccount(params.dstTraderAccount),
    generateReadonlyAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: TransferCollateralParams) => {
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.srcTraderAccount) {
    throw new Error("Source trader account is required");
  }
  if (!params.dstTraderAccount) {
    throw new Error("Destination trader account is required");
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
  if (params.amount === undefined || params.amount === null) {
    throw new Error("Amount is required");
  }
  if (params.amount <= 0n) {
    throw new Error("Amount must be greater than 0");
  }
};
