import { phoenixInstructionAddresses } from "@/core/constants";
import {
  buildCancelStopLossIx,
  type CancelStopLossIx,
} from "@/core/ixBuilders/CancelStopLoss";
import {
  buildDelegateTraderIx,
  type DelegateTraderIx,
} from "@/core/ixBuilders/DelegateTrader";
import {
  buildRegisterTraderIx,
  type RegisterTraderIx,
} from "@/core/ixBuilders/RegisterTrader";
import {
  buildOnboardTraderDelegatedIx,
  type OnboardTraderDelegatedIx,
} from "@/core/ixBuilders/OnboardTraderDelegated";
import {
  buildSyncParentToChildIx,
  type SyncParentToChildIx,
} from "@/core/ixBuilders/SyncParentToChild";
import type {
  ResolvedCancelStopLossIxInput,
  ResolvedDelegateTraderIxInput,
  ResolvedRegisterTraderIxInput,
  ResolvedOnboardTraderDelegatedIxInput,
  ResolvedSyncParentToChildIxInput,
} from "./types";

export const buildCancelStopLossIxResolved = (
  params: ResolvedCancelStopLossIxInput
): CancelStopLossIx =>
  buildCancelStopLossIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    funder: params.trader.funder ?? params.trader.authority,
    traderWallet: params.trader.authority,
    traderAccount: params.trader.traderAccount,
    stopLossAccount: params.trader.stopLossAccount,
    executionDirection: params.executionDirection,
  });

export const buildDelegateTraderIxResolved = (
  params: ResolvedDelegateTraderIxInput
): DelegateTraderIx =>
  buildDelegateTraderIx({
    ...(params.exchange.logAuthorityAddress &&
    params.exchange.globalConfigurationAddress
      ? phoenixInstructionAddresses({
          phoenixProgramAddress: params.exchange.phoenixProgramAddress,
          logAuthorityAddress: params.exchange.logAuthorityAddress,
          globalConfigurationAddress:
            params.exchange.globalConfigurationAddress,
        })
      : { programAddress: params.exchange.phoenixProgramAddress }),
    traderWallet: params.trader.authority,
    traderAccount: params.trader.traderAccount,
    newPositionAuthority: params.newPositionAuthority,
  });

export const buildRegisterTraderIxResolved = (
  params: ResolvedRegisterTraderIxInput
): RegisterTraderIx =>
  buildRegisterTraderIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    payer: params.trader.payer,
    trader: params.trader.authority,
    traderAccount: params.trader.traderAccount,
    maxPositions: params.maxPositions,
    traderPreferenceBits: params.traderPreferenceBits,
    traderPreferences: params.traderPreferences,
    disableCollateralSweep: params.disableCollateralSweep,
    traderPdaIndex: params.traderPdaIndex,
    traderSubaccountIndex: params.traderSubaccountIndex,
  });

export const buildOnboardTraderDelegatedIxResolved = (
  params: ResolvedOnboardTraderDelegatedIxInput
): OnboardTraderDelegatedIx =>
  buildOnboardTraderDelegatedIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    authority: params.trader.authority,
    permissionAccount: params.trader.permissionAccount,
    traderAccount: params.trader.traderAccount,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
  });

export const buildSyncParentToChildIxResolved = (
  params: ResolvedSyncParentToChildIxInput
): SyncParentToChildIx =>
  buildSyncParentToChildIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    traderWallet: params.trader.authority,
    parentTraderAccount: params.trader.parentTraderAccount,
    childTraderAccount: params.trader.childTraderAccount,
    globalTraderIndex: params.exchange.globalTraderIndex,
  });
