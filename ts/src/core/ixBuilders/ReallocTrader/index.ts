import {
  getPhoenixInstructionAddresses,
  SYSTEM_PROGRAM_ADDRESS,
} from "@/core/constants";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import { DISCRIMINANTS } from "@/core/discriminants";
import {
  generateReadonlyAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import type {
  Authority,
  TraderAddress,
  InstructionsWithAccountsAndData,
} from "@/primitives";

export interface ReallocTraderParams extends PhoenixInstructionAddressOverrides {
  /** Signs and pays any additional rent. */
  payer: Authority;
  trader: Authority;
  traderAccount: TraderAddress;
}

/**
 * Reserve one map entry for collateral or other account extensions without
 * changing the trader's position limit. Sufficiently sized accounts are a
 * no-op. Invoke before transferring deposit lamports so rent is funded by the
 * payer separately from the deposit.
 */
export const buildReallocTraderIx = (
  params: ReallocTraderParams
): InstructionsWithAccountsAndData => {
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  return {
    programAddress,
    accounts: [
      generateReadonlyAccount(programAddress),
      generateReadonlyAccount(logAuthorityAddress),
      generateReadonlyAccount(globalConfigurationAddress),
      generateWritableSignerAccount(params.payer),
      generateReadonlyAccount(params.trader),
      generateWritableAccount(params.traderAccount),
      generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
    ],
    data: DISCRIMINANTS.REALLOC_TRADER,
  };
};
