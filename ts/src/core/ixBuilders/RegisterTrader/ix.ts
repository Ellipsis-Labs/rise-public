import {
  getPhoenixInstructionAddresses,
  SYSTEM_PROGRAM_ADDRESS,
} from "@/core/constants";
import {
  ALL_TRADER_PREFERENCE_KINDS,
  encodeTraderPreferences,
  TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP,
} from "@/accounts/Trader/preferences";
import {
  generateReadonlyAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { getRegisterTraderInstructionEncoder } from "./codec";
import type {
  RegisterTraderAccounts,
  RegisterTraderIx,
  RegisterTraderParams,
} from "./types";

export const buildRegisterTraderIx = (
  params: RegisterTraderParams
): RegisterTraderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);
  const traderPreferenceBits = resolveTraderPreferenceBits(params);

  const data = getRegisterTraderInstructionEncoder().encode({
    maxPositions: params.maxPositions,
    traderPreferenceBits,
    traderPdaIndex: params.traderPdaIndex,
    subaccountIndex: params.traderSubaccountIndex,
  });

  const accounts: RegisterTraderAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // RegisterTraderInstructionGroupAccounts
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableSignerAccount(params.payer),
    generateReadonlyAccount(params.trader),
    generateWritableAccount(params.traderAccount),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS), // System Program
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: RegisterTraderParams) => {
  if (!params.payer) {
    throw new Error("Payer wallet is required");
  }
  if (!params.trader) {
    throw new Error("Trader wallet is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (params.maxPositions <= 0n) {
    throw new Error("Max positions must be greater than 0");
  }
  if (params.maxPositions > 128n) {
    throw new Error("Max positions cannot exceed 128");
  }
  assertIntegerInRange(params.traderPdaIndex, "Trader PDA index", 0, 255);
  assertIntegerInRange(
    params.traderSubaccountIndex,
    "Trader subaccount index",
    0,
    100
  );
  if (params.traderPreferenceBits !== undefined) {
    if (!Number.isInteger(params.traderPreferenceBits)) {
      throw new Error("Trader preference bits must be an integer");
    }
    if (
      params.traderPreferenceBits < 0 ||
      params.traderPreferenceBits > 0xffffffff
    ) {
      throw new Error("Trader preference bits must fit in a u32");
    }
  }
  if (params.traderPreferences !== undefined) {
    const validPreferences = new Set(ALL_TRADER_PREFERENCE_KINDS);
    for (const preference of params.traderPreferences) {
      if (!validPreferences.has(preference)) {
        throw new Error(`Unknown trader preference: ${preference}`);
      }
    }
  }
};

const resolveTraderPreferenceBits = (params: RegisterTraderParams): number => {
  let traderPreferenceBits = params.traderPreferenceBits ?? 0;
  if (params.traderPreferences !== undefined) {
    traderPreferenceBits |= encodeTraderPreferences(params.traderPreferences);
  }
  if (params.disableCollateralSweep === true) {
    traderPreferenceBits |= TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP;
  } else if (params.disableCollateralSweep === false) {
    traderPreferenceBits &= ~TRADER_PREFERENCE_DISABLE_COLLATERAL_SWEEP;
  }
  return traderPreferenceBits;
};

const assertIntegerInRange = (
  value: number,
  label: string,
  min: number,
  max: number
) => {
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new Error(`${label} must be an integer between ${min} and ${max}`);
  }
};
