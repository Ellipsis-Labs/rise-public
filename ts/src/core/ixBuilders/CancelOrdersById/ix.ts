import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getCancelOrdersByIdEncoder } from "./codec";
import type {
  CancelOrdersByIdAccounts,
  CancelOrdersByIdIx,
  CancelOrdersByIdParams,
} from "./types";

export const buildCancelOrdersByIdIx = (
  params: CancelOrdersByIdParams
): CancelOrdersByIdIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getCancelOrdersByIdEncoder().encode({
    orderIds: params.orderIds,
  });

  const accounts: CancelOrdersByIdAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // CancelOrdersByIdInstructionGroupAccounts
    generateWritableAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.traderWallet),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.splineCollection),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: CancelOrdersByIdParams) => {
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (!params.globalTraderIndex) {
    throw new Error("Global trader index is required");
  }
  if (!params.activeTraderBuffer) {
    throw new Error("Active trader buffer is required");
  }
  if (!params.orderbook) {
    throw new Error("Orderbook is required");
  }
  if (!params.splineCollection) {
    throw new Error("Spline collection is required");
  }
  if (!params.orderIds || params.orderIds.length === 0) {
    throw new Error("At least one order ID is required");
  }
  if (params.orderIds.length > 100) {
    throw new Error("Too many order IDs (maximum 100)");
  }
};
