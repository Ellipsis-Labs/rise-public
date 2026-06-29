import {
  getPhoenixInstructionAddresses,
  SYSTEM_PROGRAM_ADDRESS,
} from "@/core/constants";
import { DISCRIMINANTS } from "@/core/discriminants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import {
  getUpdateSplineParametersWithOrderingEncoder,
  getUpdateSplinePositionLimitsConfigEncoder,
  getUpdateSplinePriceEncoder,
  getUpdateSplinePriceWithOrderingEncoder,
} from "./codec";
import type {
  DeactivateSplineAccounts,
  DeactivateSplineIx,
  DeactivateSplineParams,
  RegisterSplineAccounts,
  RegisterSplineIx,
  RegisterSplineParams,
  SplineUpdateAccountMetas,
  SplineUpdateAccounts,
  UpdateSplineParametersParamsWithOrdering,
  UpdateSplineParametersWithOrderingIx,
  UpdateSplinePositionLimitsConfigIx,
  UpdateSplinePositionLimitsConfigParams,
  UpdateSplinePriceIx,
  UpdateSplinePriceParams,
  UpdateSplinePriceParamsWithOrdering,
  UpdateSplinePriceWithOrderingIx,
} from "./types";
import { MAX_SPLINE_REGIONS } from "./types";

export const buildRegisterSplineIx = (
  params: RegisterSplineParams
): RegisterSplineIx => {
  validateRegisterSpline(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: RegisterSplineAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.authority),
    generateWritableAccount(params.permissionAccount ?? params.authority),
    generateWritableSignerAccount(params.payer),
    generateWritableAccount(params.splineCollection),
    generateReadonlyAccount(params.marketAccount),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.traderAccount),
    generateReadonlyAccount(params.perpAssetMap),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data: DISCRIMINANTS.REGISTER_SPLINE,
  };
};

export const buildDeactivateSplineIx = (
  params: DeactivateSplineParams
): DeactivateSplineIx => {
  validateDeactivateSpline(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: DeactivateSplineAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.authority),
    generateWritableAccount(params.permissionAccount ?? params.authority),
    generateWritableAccount(params.splineCollection),
    generateReadonlyAccount(params.marketAccount),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.traderAccount),
    generateReadonlyAccount(params.perpAssetMap),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data: DISCRIMINANTS.DEACTIVATE_SPLINE,
  };
};

export const buildUpdateSplinePriceIx = (
  accounts: SplineUpdateAccounts,
  params: UpdateSplinePriceParams
): UpdateSplinePriceIx => {
  validateSplineUpdateAccounts(accounts);

  return {
    programAddress: getPhoenixInstructionAddresses(accounts).programAddress,
    accounts: buildSplineUpdateAccountMetas(accounts),
    data: getUpdateSplinePriceEncoder().encode(params),
  };
};

export const buildUpdateSplinePriceWithOrderingIx = (
  accounts: SplineUpdateAccounts,
  params: UpdateSplinePriceParamsWithOrdering
): UpdateSplinePriceWithOrderingIx => {
  validateSplineUpdateAccounts(accounts);

  return {
    programAddress: getPhoenixInstructionAddresses(accounts).programAddress,
    accounts: buildSplineUpdateAccountMetas(accounts),
    data: getUpdateSplinePriceWithOrderingEncoder().encode(params),
  };
};

export const buildUpdateSplineParametersWithOrderingIx = (
  accounts: SplineUpdateAccounts,
  params: UpdateSplineParametersParamsWithOrdering
): UpdateSplineParametersWithOrderingIx => {
  validateSplineUpdateAccounts(accounts);
  validateSplineParameters(params);

  return {
    programAddress: getPhoenixInstructionAddresses(accounts).programAddress,
    accounts: buildSplineUpdateAccountMetas(accounts),
    data: getUpdateSplineParametersWithOrderingEncoder().encode(params),
  };
};

export const buildUpdateSplinePositionLimitsConfigIx = (
  accounts: SplineUpdateAccounts,
  params: UpdateSplinePositionLimitsConfigParams
): UpdateSplinePositionLimitsConfigIx => {
  validateSplineUpdateAccounts(accounts);
  validatePositionLimitsConfig(params);

  return {
    programAddress: getPhoenixInstructionAddresses(accounts).programAddress,
    accounts: buildSplineUpdateAccountMetas(accounts),
    data: getUpdateSplinePositionLimitsConfigEncoder().encode(params),
  };
};

const buildSplineUpdateAccountMetas = (
  accounts: SplineUpdateAccounts
): SplineUpdateAccountMetas => {
  const { programAddress, logAuthorityAddress } =
    getPhoenixInstructionAddresses(accounts);

  return [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlySignerAccount(accounts.signer),
    generateReadonlyAccount(accounts.traderAccount),
    generateWritableAccount(accounts.splineCollection),
  ] as const;
};

const validateRegisterSpline = (params: RegisterSplineParams) => {
  if (!params.payer) throw new Error("Payer is required");
  validateDeactivateSpline(params);
};

const validateDeactivateSpline = (params: DeactivateSplineParams) => {
  if (!params.authority) throw new Error("Authority is required");
  if (!params.marketAccount) throw new Error("Market account is required");
  if (!params.splineCollection) {
    throw new Error("Spline collection is required");
  }
  if (!params.traderAccount) throw new Error("Trader account is required");
  if (!params.perpAssetMap) throw new Error("Perp asset map is required");
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error("Global trader index accounts are required");
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error("Active trader buffer accounts are required");
  }
};

const validateSplineUpdateAccounts = (accounts: SplineUpdateAccounts) => {
  if (!accounts.signer) throw new Error("Signer is required");
  if (!accounts.traderAccount) throw new Error("Trader account is required");
  if (!accounts.splineCollection) {
    throw new Error("Spline collection is required");
  }
};

const validateSplineParameters = (
  params: UpdateSplineParametersParamsWithOrdering
) => {
  if (params.bidRegions.length === 0 && params.askRegions.length === 0) {
    throw new Error("At least one spline region is required");
  }
  if (
    params.bidRegions.length > MAX_SPLINE_REGIONS ||
    params.askRegions.length > MAX_SPLINE_REGIONS
  ) {
    throw new Error(`Spline regions cannot exceed ${MAX_SPLINE_REGIONS}`);
  }
};

const validatePositionLimitsConfig = (
  params: UpdateSplinePositionLimitsConfigParams
) => {
  if (
    params.maxPositionSize === null &&
    params.leverageDecreaseInBps === null
  ) {
    throw new Error("At least one spline position limit update is required");
  }
  if (
    params.leverageDecreaseInBps !== null &&
    params.leverageDecreaseInBps > 10_000
  ) {
    throw new Error("Leverage decrease bps cannot exceed 10000");
  }
};
