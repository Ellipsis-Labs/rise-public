import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getUpdateTraderPreferenceBitsEncoder } from "./codec";
import type {
  UpdateTraderPreferenceBitsAccounts,
  UpdateTraderPreferenceBitsIx,
  UpdateTraderPreferenceBitsParams,
} from "./types";

/** Builds the trader-wallet-signed form of `UpdateTraderPreferenceBits`. */
export const buildUpdateTraderPreferenceBitsIx = (
  params: UpdateTraderPreferenceBitsParams
): UpdateTraderPreferenceBitsIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getUpdateTraderPreferenceBitsEncoder().encode({
    toggles: params.toggles,
  });

  // The permission slot takes the signer itself when no permission account
  // applies.
  const accounts: UpdateTraderPreferenceBitsAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.traderWallet),
    generateReadonlyAccount(params.traderWallet),
    generateWritableAccount(params.traderAccount),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: UpdateTraderPreferenceBitsParams) => {
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (params.toggles.length === 0) {
    throw new Error("At least one preference toggle is required");
  }
};
