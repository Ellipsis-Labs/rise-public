import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getCancelConditionalOrderEncoder } from "./codec";
import type {
  CancelConditionalOrderAccounts,
  CancelConditionalOrderIx,
  CancelConditionalOrderParams,
} from "./types";

export const buildCancelConditionalOrderIx = (
  params: CancelConditionalOrderParams
): CancelConditionalOrderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: CancelConditionalOrderAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableAccount(params.traderAccount),
    generateReadonlySignerAccount(params.traderWallet),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.traderConditionalOrders),
  ] as const;

  return {
    programAddress,
    accounts,
    data: getCancelConditionalOrderEncoder().encode({
      conditionalOrderIndex: params.conditionalOrderIndex,
      disableFirst: params.disableFirst,
      disableSecond: params.disableSecond,
    }),
  };
};

const validate = (params: CancelConditionalOrderParams) => {
  if (!params.traderAccount) throw new Error("Trader account is required");
  if (!params.traderWallet) throw new Error("Trader wallet is required");
  if (!params.orderbook) throw new Error("Orderbook is required");
  if (!params.traderConditionalOrders) {
    throw new Error("Trader conditional orders account is required");
  }
  if (
    !Number.isInteger(params.conditionalOrderIndex) ||
    params.conditionalOrderIndex <= 0 ||
    params.conditionalOrderIndex > 191
  ) {
    throw new Error("Conditional order index must be an integer in 1..=191");
  }
  if (!params.disableFirst && !params.disableSecond) {
    throw new Error("At least one disable flag must be set");
  }
};
