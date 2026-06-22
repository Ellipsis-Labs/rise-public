import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getUncrossCrankEncoder } from "./codec";
import type {
  UncrossCrankAccounts,
  UncrossCrankIx,
  UncrossCrankParams,
} from "./types";

export const buildUncrossCrankIx = (
  params: UncrossCrankParams
): UncrossCrankIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getUncrossCrankEncoder().encode({
    matchLimit: params.matchLimit,
  });

  const accounts: UncrossCrankAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.splineCollection),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: UncrossCrankParams) => {
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
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
  if (!params.orderbook) {
    throw new Error("Orderbook is required");
  }
  if (!params.splineCollection) {
    throw new Error("Spline collection is required");
  }
  if (params.matchLimit === undefined || params.matchLimit === null) {
    throw new Error("Match limit is required");
  }
  if (params.matchLimit < 0n) {
    throw new Error("Match limit must be non-negative");
  }
};
