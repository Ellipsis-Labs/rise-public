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
import { getPlaceLimitOrderWithConditionalsEncoder } from "./codec";
import type {
  PlaceLimitOrderWithConditionalsAccounts,
  PlaceLimitOrderWithConditionalsIx,
  PlaceLimitOrderWithConditionalsParams,
} from "./types";

export const buildPlaceLimitOrderWithConditionalsIx = (
  params: PlaceLimitOrderWithConditionalsParams
): PlaceLimitOrderWithConditionalsIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const accounts: PlaceLimitOrderWithConditionalsAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.traderWallet),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.splineCollection),
    generateWritableSignerAccount(params.payer),
    generateWritableAccount(params.traderConditionalOrders),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data: getPlaceLimitOrderWithConditionalsEncoder().encode({
      orderPacket: params.orderPacket,
      slot: params.slot ?? 0n,
      greaterTriggerOrder: params.greaterTriggerOrder,
      lessTriggerOrder: params.lessTriggerOrder,
    }),
  };
};

const validate = (params: PlaceLimitOrderWithConditionalsParams) => {
  if (!params.traderWallet) throw new Error("Trader wallet is required");
  if (!params.traderAccount) throw new Error("Trader account is required");
  if (!params.perpAssetMap) throw new Error("Perp asset map is required");
  if (!params.orderbook) throw new Error("Orderbook is required");
  if (!params.splineCollection) {
    throw new Error("Spline collection is required");
  }
  if (!params.payer) throw new Error("Payer is required");
  if (!params.traderConditionalOrders) {
    throw new Error("Trader conditional orders account is required");
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error("Global trader index array is required");
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error("Active trader buffer array is required");
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
