import {
  generateReadonlyAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { getFlightInstructionAddresses } from "@/flight/core/constants";
import {
  getFlightBuilderStateAddress,
  getFlightGlobalStateAddress,
} from "@/flight/pdas";
import { getPhoenixTraderSubaccountAddress } from "@/pdas";
import { getRegisterBuilderInstructionEncoder } from "./codec";
import type {
  RegisterBuilderAccounts,
  RegisterBuilderIx,
  RegisterBuilderParams,
} from "./types";

export const buildRegisterBuilderIx = async (
  params: RegisterBuilderParams
): Promise<RegisterBuilderIx> => {
  validate(params);

  const { programAddress, phoenixProgramAddress, systemProgramAddress } =
    getFlightInstructionAddresses(params);
  const traderPdaIndex = params.traderPdaIndex ?? 0;
  const traderSubaccountIndex = params.traderSubaccountIndex ?? 0;

  const [globalStateAccount, builderStateAccount, traderAccount] =
    await Promise.all([
      getFlightGlobalStateAddress(phoenixProgramAddress),
      getFlightBuilderStateAddress(
        params.traderAuthority,
        phoenixProgramAddress
      ),
      getPhoenixTraderSubaccountAddress({
        authority: params.traderAuthority,
        traderPdaIndex,
        subaccountIndex: traderSubaccountIndex,
        phoenixProgramAddress,
      }),
    ]);

  const accounts: RegisterBuilderAccounts = [
    generateReadonlyAccount(globalStateAccount),
    generateReadonlyAccount(phoenixProgramAddress),
    generateWritableSignerAccount(params.traderAuthority),
    generateWritableAccount(traderAccount),
    generateWritableAccount(builderStateAccount),
    generateReadonlyAccount(systemProgramAddress),
  ] as const;

  const data = getRegisterBuilderInstructionEncoder().encode({
    traderPdaIndex,
    traderSubaccountIndex,
    feeBps: params.feeBps,
  });

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: RegisterBuilderParams) => {
  if (!params.traderAuthority) {
    throw new Error("Trader authority is required");
  }
  if (params.feeBps < 0n) {
    throw new Error("Fee BPS cannot be negative");
  }
  if (
    params.traderPdaIndex !== undefined &&
    (params.traderPdaIndex < 0 || params.traderPdaIndex > 255)
  ) {
    throw new Error("Trader PDA index must be between 0 and 255");
  }
  if (
    params.traderSubaccountIndex !== undefined &&
    (params.traderSubaccountIndex < 0 || params.traderSubaccountIndex > 255)
  ) {
    throw new Error("Trader subaccount index must be between 0 and 255");
  }
};
