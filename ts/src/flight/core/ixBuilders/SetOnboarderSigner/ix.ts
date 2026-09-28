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
import { getSetOnboarderSignerInstructionEncoder } from "./codec";
import type {
  SetOnboarderSignerAccounts,
  SetOnboarderSignerIx,
  SetOnboarderSignerParams,
} from "./types";

export const buildSetOnboarderSignerIx = async (
  params: SetOnboarderSignerParams
): Promise<SetOnboarderSignerIx> => {
  validate(params);

  const { programAddress, phoenixProgramAddress } =
    getFlightInstructionAddresses(params);

  const [globalStateAccount, builderStateAccount] = await Promise.all([
    getFlightGlobalStateAddress(phoenixProgramAddress),
    getFlightBuilderStateAddress(
      params.builderAuthority,
      phoenixProgramAddress
    ),
  ]);

  const accounts: SetOnboarderSignerAccounts = [
    generateReadonlyAccount(globalStateAccount),
    generateReadonlyAccount(phoenixProgramAddress),
    generateReadonlySignerAccount(params.builderAuthority),
    generateWritableAccount(builderStateAccount),
  ] as const;

  const data = getSetOnboarderSignerInstructionEncoder().encode(params.signer);

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: SetOnboarderSignerParams) => {
  if (!params.builderAuthority) {
    throw new Error("Builder authority is required");
  }
  if (!params.signer) {
    throw new Error("Signer is required");
  }
};
