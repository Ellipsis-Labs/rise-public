import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getFlightInstructionAddresses } from "@/flight/core/constants";
import {
  getFlightBuilderStateAddress,
  getFlightGlobalStateAddress,
} from "@/flight/pdas";
import { getUpdateFeeInstructionEncoder } from "./codec";
import type { UpdateFeeAccounts, UpdateFeeIx, UpdateFeeParams } from "./types";

export const buildUpdateFeeIx = async (
  params: UpdateFeeParams
): Promise<UpdateFeeIx> => {
  validate(params);

  const { programAddress, phoenixProgramAddress } =
    getFlightInstructionAddresses(params);

  const [globalStateAccount, builderStateAccount] = await Promise.all([
    getFlightGlobalStateAddress(phoenixProgramAddress),
    getFlightBuilderStateAddress(params.traderAuthority, phoenixProgramAddress),
  ]);

  const accounts: UpdateFeeAccounts = [
    generateReadonlyAccount(globalStateAccount),
    generateReadonlyAccount(phoenixProgramAddress),
    generateReadonlySignerAccount(params.traderAuthority),
    generateWritableAccount(builderStateAccount),
  ] as const;

  const data = getUpdateFeeInstructionEncoder().encode(params.feeBps);

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: UpdateFeeParams) => {
  if (!params.traderAuthority) {
    throw new Error("Trader authority is required");
  }
  if (params.feeBps < 0n) {
    throw new Error("Fee BPS cannot be negative");
  }
};
