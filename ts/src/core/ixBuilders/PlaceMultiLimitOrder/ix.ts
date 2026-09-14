import {
  getPhoenixInstructionAddresses,
  type PhoenixInstructionAddresses,
} from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import {
  CondensedOrderFlags,
  type CondensedOrderV2,
} from "@/primitives/OrderPacket";
import {
  getPlaceMultiLimitOrderEncoder,
  getPlaceMultiLimitOrderV2Encoder,
} from "./codec";
import type {
  PlaceMultiLimitOrderAccounts,
  PlaceMultiLimitOrderIx,
  PlaceMultiLimitOrderParams,
  PlaceMultiLimitOrderV2Params,
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

  const accounts = buildAccounts(params, {
    programAddress,
    logAuthorityAddress,
    globalConfigurationAddress,
  });

  return {
    programAddress,
    accounts,
    data,
  };
};

export const buildPlaceMultiLimitOrderV2Ix = (
  params: PlaceMultiLimitOrderV2Params
): PlaceMultiLimitOrderIx => {
  validate(params);
  validateV2Packet(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getPlaceMultiLimitOrderV2Encoder().encode(
    params.multipleOrderPacket
  );

  const accounts = buildAccounts(params, {
    programAddress,
    logAuthorityAddress,
    globalConfigurationAddress,
  });

  return {
    programAddress,
    accounts,
    data,
  };
};

const buildAccounts = (
  params: PlaceMultiLimitOrderParams | PlaceMultiLimitOrderV2Params,
  addresses: PhoenixInstructionAddresses
): PlaceMultiLimitOrderAccounts =>
  [
    generateReadonlyAccount(addresses.programAddress),
    generateReadonlyAccount(addresses.logAuthorityAddress),
    generateWritableAccount(addresses.globalConfigurationAddress),
    generateReadonlySignerAccount(params.trader),
    generateWritableAccount(params.traderAccount),
    generateWritableAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
    generateWritableAccount(params.orderbook),
    generateWritableAccount(params.splineCollection),
  ] as const;

const validate = (
  params: PlaceMultiLimitOrderParams | PlaceMultiLimitOrderV2Params
) => {
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

const DEFINED_CONDENSED_ORDER_FLAGS_MASK =
  CondensedOrderFlags.Slide | CondensedOrderFlags.ReduceOnly;

const validateCondensedOrderV2Flags = (order: CondensedOrderV2) => {
  if ((order.flags & ~DEFINED_CONDENSED_ORDER_FLAGS_MASK) !== 0) {
    throw new Error(
      `Unknown CondensedOrderV2 flag bits: ${order.flags.toString(16)}`
    );
  }
};

const validateV2Packet = (params: PlaceMultiLimitOrderV2Params) => {
  const { scaleSetId, bids, asks } = params.multipleOrderPacket;
  if (!Number.isInteger(scaleSetId) || scaleSetId < 0 || scaleSetId > 255) {
    throw new Error(
      `scaleSetId must be an integer in 0..=255; got ${scaleSetId}`
    );
  }
  for (const order of bids) {
    validateCondensedOrderV2Flags(order);
  }
  for (const order of asks) {
    validateCondensedOrderV2Flags(order);
  }
};
