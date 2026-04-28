import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getPlacePostOnlyOrderEncoder } from "./codec";
import type {
  PlacePostOnlyOrderAccounts,
  PlacePostOnlyOrderIx,
  PlacePostOnlyOrderParams,
} from "./types";

export const buildPlacePostOnlyOrderIx = (
  params: PlacePostOnlyOrderParams
): PlacePostOnlyOrderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getPlacePostOnlyOrderEncoder().encode(params.orderPacket);

  const accounts: PlacePostOnlyOrderAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateWritableAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.trader),
    generateWritableAccount(params.traderAccount),
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

const validate = (params: PlacePostOnlyOrderParams) => {
  if (!params.trader) {
    throw new Error("Trader wallet is required");
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
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error(
      "Active trader buffer array is required and must not be empty"
    );
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error(
      "Global trader index array is required and must not be empty"
    );
  }
  if (!params.orderPacket) {
    throw new Error("Order packet is required");
  }
};
