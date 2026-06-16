import {
  buildCreateAssociatedTokenAccountIdempotentSync,
  buildSplTokenApprove,
} from "@/core/helpers";
import {
  buildDepositFundsIx,
  type DepositFundsIx,
} from "@/core/ixBuilders/DepositFunds";
import {
  buildEmberDepositIx,
  type EmberDepositIx,
} from "@/core/ixBuilders/EmberDeposit";
import {
  buildEmberWithdrawIx,
  type EmberWithdrawIx,
} from "@/core/ixBuilders/EmberWithdraw";
import {
  buildTransferCollateralChildToParentIx,
  type TransferCollateralChildToParentIx,
} from "@/core/ixBuilders/TransferCollateralChildToParent";
import {
  buildTransferCollateralIx,
  type TransferCollateralIx,
} from "@/core/ixBuilders/TransferCollateral";
import {
  buildWithdrawFundsIx,
  type WithdrawFundsIx,
} from "@/core/ixBuilders/WithdrawFunds";
import { phoenixInstructionAddresses } from "@/core/constants";
import type {
  BuildDepositIxsResolvedInput,
  BuildWithdrawIxsResolvedInput,
  DepositIxsResult,
  ResolvedDepositFundsIxInput,
  ResolvedEmberDepositIxInput,
  ResolvedEmberWithdrawIxInput,
  ResolvedTransferCollateralChildToParentIxInput,
  ResolvedTransferCollateralIxInput,
  ResolvedWithdrawFundsIxInput,
  WithdrawIxsResult,
} from "./types";

export const buildDepositFundsIxResolved = (
  params: ResolvedDepositFundsIxInput
): DepositFundsIx =>
  buildDepositFundsIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    trader: params.trader.authority,
    mint: params.exchange.canonicalMint,
    traderAccount: params.trader.traderAccount,
    traderTokenAccount: params.trader.traderTokenAccount,
    globalVault: params.exchange.globalVault,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    permissionAccount: params.trader.permissionAddress,
    amount: params.amount,
  });

export const buildWithdrawFundsIxResolved = (
  params: ResolvedWithdrawFundsIxInput
): WithdrawFundsIx =>
  buildWithdrawFundsIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    trader: params.trader.authority,
    traderAccount: params.trader.traderAccount,
    mint: params.exchange.canonicalMint,
    perpAssetMap: params.exchange.perpAssetMap,
    destinationTokenAccount: params.trader.destinationTokenAccount,
    globalVault: params.exchange.globalVault,
    withdrawQueue: params.exchange.withdrawQueue,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    amount: params.amount,
  });

export const buildTransferCollateralIxResolved = (
  params: ResolvedTransferCollateralIxInput
): TransferCollateralIx =>
  buildTransferCollateralIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    trader: params.trader.positionAuthority ?? params.trader.authority,
    srcTraderAccount: params.trader.srcTraderAccount,
    dstTraderAccount: params.trader.dstTraderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    permissionAccount: params.trader.permissionAddress,
    amount: params.amount,
  });

export const buildTransferCollateralChildToParentIxResolved = (
  params: ResolvedTransferCollateralChildToParentIxInput
): TransferCollateralChildToParentIx =>
  buildTransferCollateralChildToParentIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    trader: params.trader.positionAuthority ?? params.trader.authority,
    childTraderAccount: params.trader.childTraderAccount,
    parentTraderAccount: params.trader.parentTraderAccount,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
  });

export const buildEmberDepositIxResolved = (
  params: ResolvedEmberDepositIxInput
): EmberDepositIx =>
  buildEmberDepositIx({
    owner: params.trader.authority,
    inputMint: params.exchange.usdcMint,
    outputMint: params.exchange.canonicalMint,
    inputTokenAccount: params.trader.usdcTokenAccount,
    outputTokenAccount: params.trader.phoenixTokenAccount,
    emberState: params.exchange.emberState,
    emberVault: params.exchange.emberVault,
    amount: params.amount,
  });

export const buildEmberWithdrawIxResolved = (
  params: ResolvedEmberWithdrawIxInput
): EmberWithdrawIx =>
  buildEmberWithdrawIx({
    owner: params.trader.authority,
    inputMint: params.exchange.usdcMint,
    outputMint: params.exchange.canonicalMint,
    inputTokenAccount: params.trader.usdcTokenAccount,
    outputTokenAccount: params.trader.phoenixTokenAccount,
    emberState: params.exchange.emberState,
    emberVault: params.exchange.emberVault,
    amount: params.amount,
  });

export const buildDepositIxsResolved = (
  params: BuildDepositIxsResolvedInput
): DepositIxsResult => {
  const feePayer = params.feePayer ?? params.trader.authority;
  const createPhoenixAta = buildCreateAssociatedTokenAccountIdempotentSync({
    payer: feePayer,
    ataAddress: params.trader.phoenixTokenAccount,
    owner: params.trader.authority,
    mint: params.exchange.canonicalMint,
  });
  const emberDeposit = buildEmberDepositIxResolved({
    exchange: {
      usdcMint: params.exchange.usdcMint,
      canonicalMint: params.exchange.canonicalMint,
      emberState: params.exchange.emberState,
      emberVault: params.exchange.emberVault,
    },
    trader: {
      authority: params.trader.authority,
      usdcTokenAccount: params.trader.usdcTokenAccount,
      phoenixTokenAccount: params.trader.phoenixTokenAccount,
    },
    amount: params.amount,
  });
  const depositFunds = buildDepositFundsIxResolved({
    exchange: {
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
      canonicalMint: params.exchange.canonicalMint,
      globalVault: params.exchange.globalVault,
      globalTraderIndex: params.exchange.globalTraderIndex,
      activeTraderBuffer: params.exchange.activeTraderBuffer,
    },
    trader: {
      authority: params.trader.authority,
      traderAccount: params.trader.traderAccount,
      traderTokenAccount: params.trader.phoenixTokenAccount,
    },
    amount: params.amount,
  });

  return {
    instructions: [createPhoenixAta, emberDeposit, depositFunds],
    named: {
      createPhoenixAta,
      emberDeposit,
      depositFunds,
    },
  };
};

export const buildWithdrawIxsResolved = (
  params: BuildWithdrawIxsResolvedInput
): WithdrawIxsResult => {
  const feePayer = params.feePayer ?? params.trader.authority;
  const createPhoenixAta = buildCreateAssociatedTokenAccountIdempotentSync({
    payer: feePayer,
    ataAddress: params.trader.phoenixTokenAccount,
    owner: params.trader.authority,
    mint: params.exchange.canonicalMint,
  });
  const approveToken = buildSplTokenApprove({
    owner: params.trader.authority,
    tokenAccount: params.trader.phoenixTokenAccount,
    delegate: params.exchange.emberState,
    amount: params.amount,
  });
  const createUsdcAta = buildCreateAssociatedTokenAccountIdempotentSync({
    payer: feePayer,
    ataAddress: params.trader.usdcTokenAccount,
    owner: params.trader.authority,
    mint: params.exchange.usdcMint,
  });
  const withdrawFunds = buildWithdrawFundsIxResolved({
    exchange: {
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
      canonicalMint: params.exchange.canonicalMint,
      perpAssetMap: params.exchange.perpAssetMap,
      globalVault: params.exchange.globalVault,
      withdrawQueue: params.exchange.withdrawQueue,
      globalTraderIndex: params.exchange.globalTraderIndex,
      activeTraderBuffer: params.exchange.activeTraderBuffer,
    },
    trader: {
      authority: params.trader.authority,
      traderAccount: params.trader.traderAccount,
      destinationTokenAccount: params.trader.phoenixTokenAccount,
    },
    amount: params.amount,
  });
  const emberWithdraw = buildEmberWithdrawIxResolved({
    exchange: {
      usdcMint: params.exchange.usdcMint,
      canonicalMint: params.exchange.canonicalMint,
      emberState: params.exchange.emberState,
      emberVault: params.exchange.emberVault,
    },
    trader: {
      authority: params.trader.authority,
      usdcTokenAccount: params.trader.usdcTokenAccount,
      phoenixTokenAccount: params.trader.phoenixTokenAccount,
    },
    amount: params.amount,
  });

  return {
    instructions: [
      createPhoenixAta,
      approveToken,
      createUsdcAta,
      withdrawFunds,
      emberWithdraw,
    ],
    named: {
      createPhoenixAta,
      approveToken,
      createUsdcAta,
      withdrawFunds,
      emberWithdraw,
    },
  };
};
