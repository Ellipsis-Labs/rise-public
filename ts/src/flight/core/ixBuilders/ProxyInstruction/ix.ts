import {
  generateReadonlyAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getFlightInstructionAddresses } from "@/flight/core/constants";
import {
  getFlightBuilderStateAddress,
  getFlightGlobalStateAddress,
} from "@/flight/pdas";
import { encodeProxyInstructionData } from "./codec";
import type {
  ProxyInstructionAccounts,
  ProxyInstructionIx,
  ProxyInstructionParams,
} from "./types";
import type { PhoenixProgramAddress } from "@/primitives/index.js";

export const buildProxyInstructionIx = async (
  params: ProxyInstructionParams
): Promise<ProxyInstructionIx> => {
  const resolvedAddresses = getFlightInstructionAddresses(params);

  const { programAddress, phoenixProgramAddress } = resolvedAddresses;

  validate(params, phoenixProgramAddress);

  const [globalStateAccount, builderStateAccount] = await Promise.all([
    getFlightGlobalStateAddress(phoenixProgramAddress),
    getFlightBuilderStateAddress(
      params.builderAuthority,
      phoenixProgramAddress
    ),
  ]);

  const accounts: ProxyInstructionAccounts = [
    generateReadonlyAccount(globalStateAccount),
    generateReadonlyAccount(phoenixProgramAddress),
    generateReadonlyAccount(params.builderAuthority),
    generateWritableAccount(params.builderTraderAccount),
    generateReadonlyAccount(builderStateAccount),
    generateReadonlyAccount(params.traderWallet),
    ...(params.innerInstruction.accounts ?? []),
  ] as const;

  const data = encodeProxyInstructionData(
    params.innerInstruction.data ?? new Uint8Array(0)
  );

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (
  params: ProxyInstructionParams,
  phoenixProgramAddress: PhoenixProgramAddress
) => {
  if (!params.builderAuthority) {
    throw new Error("Builder authority is required");
  }
  if (!params.builderTraderAccount) {
    throw new Error("Builder trader account is required");
  }
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.innerInstruction?.programAddress) {
    throw new Error("Inner instruction program address is required");
  }

  if (params.innerInstruction.programAddress !== phoenixProgramAddress) {
    throw new Error(
      "Inner instruction program address must match phoenix program address"
    );
  }
};
