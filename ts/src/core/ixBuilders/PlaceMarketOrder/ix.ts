import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getPlaceMarketOrderEncoder } from "./codec";
import type {
  PlaceMarketOrderAccounts,
  PlaceMarketOrderIx,
  PlaceMarketOrderParams,
} from "./types";

export const buildPlaceMarketOrderIx = (
  params: PlaceMarketOrderParams
): PlaceMarketOrderIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getPlaceMarketOrderEncoder().encode(params.orderPacket);

  const accounts: PlaceMarketOrderAccounts = [
    // LogAccountGroupAccounts (2 accounts)
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    // MarketActionInstructionGroupAccounts
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

const validate = (params: PlaceMarketOrderParams) => {
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
