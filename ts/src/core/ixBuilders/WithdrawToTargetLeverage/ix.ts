import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  buildTransferCollateralAccounts,
  validateTransferCollateralAccounts,
} from "../TransferCollateral/ix";
import { getWithdrawToTargetLeverageEncoder } from "./codec";
import type {
  WithdrawToTargetLeverageIx,
  WithdrawToTargetLeverageParams,
} from "./types";

export const buildWithdrawToTargetLeverageIx = (
  params: WithdrawToTargetLeverageParams
): WithdrawToTargetLeverageIx => {
  validateTransferCollateralAccounts(params);
  if (
    params.targetLeverageBps === undefined ||
    params.targetLeverageBps === null
  ) {
    throw new Error("Target leverage is required");
  }
  if (params.targetLeverageBps <= 0n) {
    throw new Error("Target leverage must be greater than 0");
  }

  const addresses = getPhoenixInstructionAddresses(params);

  return {
    programAddress: addresses.programAddress,
    accounts: buildTransferCollateralAccounts(params, addresses),
    data: getWithdrawToTargetLeverageEncoder().encode(params.targetLeverageBps),
  };
};
