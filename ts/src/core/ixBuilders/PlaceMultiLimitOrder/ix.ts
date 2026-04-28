import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getPlaceMultiLimitOrderEncoder } from "./codec";
import type {
  PlaceMultiLimitOrderAccounts,
  PlaceMultiLimitOrderIx,
  PlaceMultiLimitOrderParams,
} from "./types";

export const buildPlaceMultiLimitOrderIx = (
  params: PlaceMultiLimitOrderParams
): PlaceMultiLimitOrderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getPlaceMultiLimitOrderEncoder().encode(
    params.multipleOrderPacket
  );

  const accounts: PlaceMultiLimitOrderAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
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

const validate = (params: PlaceMultiLimitOrderParams) => {
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
  if (!params.multipleOrderPacket) {
    throw new Error("Multiple order packet is required");
  }
};
