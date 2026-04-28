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
import { Direction } from "@/primitives";
import { getPlacePositionConditionalOrderEncoder } from "./codec";
import type {
  PlacePositionConditionalOrderAccounts,
  PlacePositionConditionalOrderIx,
  PlacePositionConditionalOrderParams,
} from "./types";

export const buildPlacePositionConditionalOrderIx = (
  params: PlacePositionConditionalOrderParams
): PlacePositionConditionalOrderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: PlacePositionConditionalOrderAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableSignerAccount(params.payer),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.splineCollection),
    generateReadonlySignerAccount(params.traderWallet),
    generateWritableAccount(params.traderConditionalOrders),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data: getPlacePositionConditionalOrderEncoder().encode({
      assetId: params.assetId,
      greaterTriggerOrder: params.greaterTriggerOrder,
      lessTriggerOrder: params.lessTriggerOrder,
      sizeBaseLots: params.sizeBaseLots,
      sizePercent: params.sizePercent,
    }),
  };
};

const validate = (params: PlacePositionConditionalOrderParams) => {
  if (!params.payer) throw new Error("Payer is required");
  if (!params.traderAccount) throw new Error("Trader account is required");
  if (!params.perpAssetMap) throw new Error("Perp asset map is required");
  if (!params.orderbook) throw new Error("Orderbook is required");
  if (!params.splineCollection) {
    throw new Error("Spline collection is required");
  }
  if (!params.traderWallet) throw new Error("Trader wallet is required");
  if (!params.traderConditionalOrders) {
    throw new Error("Trader conditional orders account is required");
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error("Global trader index array is required");
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error("Active trader buffer array is required");
  }
  if (
    !Number.isInteger(params.assetId) ||
    params.assetId < 0 ||
    params.assetId > 0xffffffff
  ) {
    throw new Error("Asset id must be a u32");
  }
  if (!params.greaterTriggerOrder && !params.lessTriggerOrder) {
    throw new Error("At least one trigger direction is required");
  }
  if (
    params.greaterTriggerOrder &&
    params.greaterTriggerOrder.triggerDirection !== Direction.GreaterThan
  ) {
    throw new Error("Greater trigger must use Direction.GreaterThan");
  }
  if (
    params.lessTriggerOrder &&
    params.lessTriggerOrder.triggerDirection !== Direction.LessThan
  ) {
    throw new Error("Less trigger must use Direction.LessThan");
  }
  if (
    (params.sizeBaseLots === null || params.sizeBaseLots === undefined) &&
    (params.sizePercent === null || params.sizePercent === undefined)
  ) {
    throw new Error("Either sizeBaseLots or sizePercent is required");
  }
  if (
    params.sizeBaseLots !== null &&
    params.sizeBaseLots !== undefined &&
    params.sizePercent !== null &&
    params.sizePercent !== undefined
  ) {
    throw new Error("sizeBaseLots and sizePercent cannot both be set");
  }
  if (
    params.sizePercent !== null &&
    params.sizePercent !== undefined &&
    (!Number.isInteger(params.sizePercent) ||
      params.sizePercent <= 0 ||
      params.sizePercent > 100)
  ) {
    throw new Error("sizePercent must be an integer in 1..=100");
  }
};
