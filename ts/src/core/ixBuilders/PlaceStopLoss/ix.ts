import {
  SYSTEM_PROGRAM_ADDRESS,
  getPhoenixInstructionAddresses,
} from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { getPlaceStopLossEncoder } from "./codec";
import type {
  PlaceStopLossAccounts,
  PlaceStopLossIx,
  PlaceStopLossParams,
} from "./types";

export const buildPlaceStopLossIx = (
  params: PlaceStopLossParams
): PlaceStopLossIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getPlaceStopLossEncoder().encode({
    triggerPrice: params.triggerPrice,
    executionPrice: params.executionPrice,
    tradeSize: 0n,
    tradeSide: params.tradeSide,
    executionDirection: params.executionDirection,
    orderKind: params.orderKind,
  });

  const accounts: PlaceStopLossAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateWritableSignerAccount(params.funder),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.splineCollection),
    generateReadonlySignerAccount(params.positionAuthority),
    generateWritableAccount(params.stopLossAccount),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: PlaceStopLossParams) => {
  if (!params.funder) {
    throw new Error("Funder is required");
  }
  if (!params.traderAccount) {
    throw new Error("Trader account is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (!params.orderbook) {
    throw new Error("Orderbook is required");
  }
  if (!params.splineCollection) {
    throw new Error("Spline collection is required");
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
  if (!params.positionAuthority) {
    throw new Error("Position authority is required");
  }
  if (!params.stopLossAccount) {
    throw new Error("Stop loss account is required");
  }
  if (params.triggerPrice === undefined || params.triggerPrice === null) {
    throw new Error("Trigger price is required");
  }
  if (params.executionPrice === undefined || params.executionPrice === null) {
    throw new Error("Execution price is required");
  }
  if (params.executionDirection === undefined) {
    throw new Error("Execution direction is required");
  }
  if (params.orderKind === undefined) {
    throw new Error("Order kind is required");
  }
};
