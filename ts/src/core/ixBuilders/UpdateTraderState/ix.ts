import { getPhoenixInstructionAddresses } from "@/core/constants";
import { DISCRIMINANTS } from "@/core/discriminants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import type {
  UpdateTraderStateAccounts,
  UpdateTraderStateIx,
  UpdateTraderStateParams,
} from "./types";

export const buildUpdateTraderStateIx = (
  params: UpdateTraderStateParams
): UpdateTraderStateIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: UpdateTraderStateAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableAccount(params.traderAccount),
    generateReadonlyAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data: DISCRIMINANTS.UPDATE_TRADER_STATE,
  };
};

const validate = (params: UpdateTraderStateParams) => {
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error("Global trader index accounts are required");
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error("Active trader buffer accounts are required");
  }
};
