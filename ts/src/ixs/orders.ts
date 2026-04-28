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
  buildPlaceLimitOrderIx,
  type PlaceLimitOrderIx,
} from "@/core/ixBuilders/PlaceLimitOrder";
import {
  buildPlaceMarketOrderIx,
  type PlaceMarketOrderIx,
} from "@/core/ixBuilders/PlaceMarketOrder";
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
  BuildPlaceLimitOrderIxResolvedInput,
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

export const buildPlaceStopLossIxResolved = (
  params: BuildPlaceStopLossIxResolvedInput
): PlaceStopLossIx =>
  buildPlaceStopLossIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    funder: params.trader.authority,
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
