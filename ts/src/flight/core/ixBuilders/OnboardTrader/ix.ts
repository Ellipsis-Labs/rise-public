import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { getFlightInstructionAddresses } from "@/flight/core/constants";
import {
  getFlightBuilderStateAddress,
  getFlightGlobalStateAddress,
  getFlightTraderOnboardingAuthorityAddress,
  getFlightTraderOnboardingPermissionAddress,
} from "@/flight/pdas";
import { getPhoenixTraderSubaccountAddress } from "@/pdas";
import { getOnboardTraderInstructionEncoder } from "./codec";
import type {
  OnboardTraderAccounts,
  OnboardTraderIx,
  OnboardTraderParams,
} from "./types";

export const buildOnboardTraderIx = async (
  params: OnboardTraderParams
): Promise<OnboardTraderIx> => {
  validate(params);

  const { programAddress, phoenixProgramAddress, systemProgramAddress } =
    getFlightInstructionAddresses(params);
  const { logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses({
      programAddress: phoenixProgramAddress,
      phoenixEnv: params.phoenixEnv,
      logAuthorityAddress: params.logAuthorityAddress,
      globalConfigurationAddress: params.globalConfigurationAddress,
    });

  const [
    globalStateAccount,
    builderStateAccount,
    traderAccount,
    traderOnboardingAuthorityAddress,
    onboardingPermissionAddress,
    feeUpdatePermissionAddress,
  ] = await Promise.all([
    getFlightGlobalStateAddress(phoenixProgramAddress),
    getFlightBuilderStateAddress(
      params.builderAuthority,
      phoenixProgramAddress
    ),
    getPhoenixTraderSubaccountAddress({
      authority: params.traderWallet,
      traderPdaIndex: 0,
      subaccountIndex: 0,
      phoenixProgramAddress,
    }),
    getFlightTraderOnboardingAuthorityAddress(phoenixProgramAddress),
    getFlightTraderOnboardingPermissionAddress(
      params.riskAuthority,
      phoenixProgramAddress
    ),
    getFlightTraderOnboardingPermissionAddress(
      params.marketAuthority,
      phoenixProgramAddress
    ),
  ]);

  const accounts: OnboardTraderAccounts = [
    generateReadonlyAccount(globalStateAccount),
    generateReadonlyAccount(phoenixProgramAddress),
    generateReadonlyAccount(params.builderAuthority),
    generateWritableAccount(builderStateAccount),
    generateReadonlySignerAccount(params.onboarderSigner),
    generateReadonlyAccount(traderOnboardingAuthorityAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableSignerAccount(params.payer),
    generateReadonlyAccount(params.traderWallet),
    generateWritableAccount(traderAccount),
    generateReadonlyAccount(systemProgramAddress),
    generateWritableAccount(onboardingPermissionAddress),
    generateWritableAccount(feeUpdatePermissionAddress),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  const data = getOnboardTraderInstructionEncoder().encode({
    maxPositions: params.maxPositions,
    traderPreferenceBits: params.traderPreferenceBits,
  });

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: OnboardTraderParams) => {
  if (!params.builderAuthority) {
    throw new Error("Builder authority is required");
  }
  if (!params.onboarderSigner) {
    throw new Error("Onboarder signer is required");
  }
  if (!params.payer) {
    throw new Error("Payer is required");
  }
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.riskAuthority) {
    throw new Error("Risk authority is required");
  }
  if (!params.marketAuthority) {
    throw new Error("Market authority is required");
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error(
      "Global trader index array is required and must not be empty"
    );
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error(
      "Active trader buffer array is required and must not be empty"
    );
  }
  if (
    !Number.isInteger(params.maxPositions) ||
    params.maxPositions < 0 ||
    params.maxPositions > 0xffffffff
  ) {
    throw new Error("Max positions must be an integer between 0 and 2^32-1");
  }
  if (
    !Number.isInteger(params.traderPreferenceBits) ||
    params.traderPreferenceBits < 0 ||
    params.traderPreferenceBits > 0xffffffff
  ) {
    throw new Error(
      "Trader preference bits must be an integer between 0 and 2^32-1"
    );
  }
};
