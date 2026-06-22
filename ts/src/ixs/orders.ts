import { phoenixInstructionAddresses } from "@/core/constants";
import {
  buildCancelAllIx,
  type CancelAllIx,
} from "@/core/ixBuilders/CancelAll";
import {
  buildCancelOrdersByIdIx,
  type CancelOrdersByIdIx,
} from "@/core/ixBuilders/CancelOrdersById";
import {
  buildCancelUpToIx,
  type CancelUpToIx,
} from "@/core/ixBuilders/CancelUpTo";
import {
  buildUncrossCrankIx,
  type UncrossCrankIx,
} from "@/core/ixBuilders/UncrossCrank";
import {
  buildPlaceLimitOrderIx,
  type PlaceLimitOrderIx,
} from "@/core/ixBuilders/PlaceLimitOrder";
import {
  buildPlaceMarketOrderIx,
  type PlaceMarketOrderIx,
} from "@/core/ixBuilders/PlaceMarketOrder";
import {
  buildPlaceMarketOrderDelegatedIx,
  type PlaceMarketOrderDelegatedIx,
} from "@/core/ixBuilders/PlaceMarketOrderDelegated";
import {
  buildPlacePostOnlyOrderIx,
  type PlacePostOnlyOrderIx,
} from "@/core/ixBuilders/PlacePostOnlyOrder";
import {
  buildPlaceStopLossIx,
  type PlaceStopLossIx,
} from "@/core/ixBuilders/PlaceStopLoss";
import type {
  BuildCancelAllIxResolvedInput,
  BuildCancelOrdersByIdIxResolvedInput,
  BuildCancelUpToIxResolvedInput,
  BuildUncrossCrankIxResolvedInput,
  BuildPlaceLimitOrderIxResolvedInput,
  BuildPlaceMarketOrderDelegatedIxResolvedInput,
  BuildPlaceMarketOrderIxResolvedInput,
  BuildPlacePostOnlyOrderIxResolvedInput,
  BuildPlaceStopLossIxResolvedInput,
} from "./types";

export const buildPlaceLimitOrderIxResolved = (
  params: BuildPlaceLimitOrderIxResolvedInput
): PlaceLimitOrderIx =>
  buildPlaceLimitOrderIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    trader: params.trader.positionAuthority ?? params.trader.authority,
    traderAccount: params.trader.traderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
    orderPacket: params.orderPacket,
  });

export const buildPlaceMarketOrderIxResolved = (
  params: BuildPlaceMarketOrderIxResolvedInput
): PlaceMarketOrderIx =>
  buildPlaceMarketOrderIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    trader: params.trader.positionAuthority ?? params.trader.authority,
    traderAccount: params.trader.traderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
    orderPacket: params.orderPacket,
  });

export const buildPlaceMarketOrderDelegatedIxResolved = (
  params: BuildPlaceMarketOrderDelegatedIxResolvedInput
): PlaceMarketOrderDelegatedIx => {
  const traderWallet =
    params.traderWallet ??
    params.trader.positionAuthority ??
    params.trader.authority;
  return buildPlaceMarketOrderDelegatedIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    traderWallet,
    permissionAccount: params.permissionAccount ?? traderWallet,
    traderAccount: params.trader.traderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
    orderPacket: params.orderPacket,
  });
};

export const buildPlacePostOnlyOrderIxResolved = (
  params: BuildPlacePostOnlyOrderIxResolvedInput
): PlacePostOnlyOrderIx =>
  buildPlacePostOnlyOrderIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    trader: params.trader.positionAuthority ?? params.trader.authority,
    traderAccount: params.trader.traderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
    orderPacket: params.orderPacket,
  });

export const buildCancelAllIxResolved = (
  params: BuildCancelAllIxResolvedInput
): CancelAllIx =>
  buildCancelAllIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    traderWallet: params.trader.positionAuthority ?? params.trader.authority,
    traderAccount: params.trader.traderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
  });

export const buildCancelOrdersByIdIxResolved = (
  params: BuildCancelOrdersByIdIxResolvedInput
): CancelOrdersByIdIx =>
  buildCancelOrdersByIdIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    traderWallet: params.trader.positionAuthority ?? params.trader.authority,
    traderAccount: params.trader.traderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
    orderIds: params.orderIds,
  });

export const buildCancelUpToIxResolved = (
  params: BuildCancelUpToIxResolvedInput
): CancelUpToIx =>
  buildCancelUpToIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    trader: params.trader.positionAuthority ?? params.trader.authority,
    traderAccount: params.trader.traderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
    side: params.side,
    numOrdersToCancel:
      params.numOrdersToCancel === undefined ||
      params.numOrdersToCancel === null
        ? null
        : BigInt(params.numOrdersToCancel),
    tickLimit: params.tickLimit ?? null,
  });

export const buildUncrossCrankIxResolved = (
  params: BuildUncrossCrankIxResolvedInput
): UncrossCrankIx =>
  buildUncrossCrankIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
    matchLimit:
      params.matchLimit === undefined ? 100n : BigInt(params.matchLimit),
  });

export const buildPlaceStopLossIxResolved = (
  params: BuildPlaceStopLossIxResolvedInput
): PlaceStopLossIx =>
  buildPlaceStopLossIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    funder: params.trader.funder ?? params.trader.authority,
    traderAccount: params.trader.traderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    orderbook: params.market.marketAddress,
    splineCollection: params.market.splineCollection,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    positionAuthority:
      params.trader.positionAuthority ?? params.trader.authority,
    stopLossAccount: params.stopLossAccount,
    triggerPrice: params.triggerPrice,
    executionPrice: params.executionPrice,
    tradeSide: params.tradeSide,
    executionDirection: params.executionDirection,
    orderKind: params.orderKind,
  });
