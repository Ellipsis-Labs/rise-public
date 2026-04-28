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
import { getPlaceAttachedConditionalOrderEncoder } from "./codec";
import type {
  PlaceAttachedConditionalOrderAccounts,
  PlaceAttachedConditionalOrderIx,
  PlaceAttachedConditionalOrderParams,
} from "./types";

export const buildPlaceAttachedConditionalOrderIx = (
  params: PlaceAttachedConditionalOrderParams
): PlaceAttachedConditionalOrderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: PlaceAttachedConditionalOrderAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableAccount(params.traderAccount),
    generateReadonlySignerAccount(params.traderWallet),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.traderConditionalOrders),
    generateWritableSignerAccount(params.payer),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data: getPlaceAttachedConditionalOrderEncoder().encode({
      orderId: params.orderId,
      assetId: params.assetId,
      greaterTriggerOrder: params.greaterTriggerOrder,
      lessTriggerOrder: params.lessTriggerOrder,
    }),
  };
};

const validate = (params: PlaceAttachedConditionalOrderParams) => {
  if (!params.traderAccount) throw new Error("Trader account is required");
  if (!params.traderWallet) throw new Error("Trader wallet is required");
  if (!params.orderbook) throw new Error("Orderbook is required");
  if (!params.traderConditionalOrders) {
    throw new Error("Trader conditional orders account is required");
  }
  if (!params.payer) throw new Error("Payer is required");
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
};
