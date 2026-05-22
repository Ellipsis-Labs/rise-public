import { MAX_SUBACCOUNTS } from "@/core/helpers";
import {
  buildCreateConditionalOrdersAccountIx,
  type CreateConditionalOrdersAccountIx,
} from "@/core/ixBuilders/CreateConditionalOrdersAccount";
import {
  buildPlaceAttachedConditionalOrderIx,
  type PlaceAttachedConditionalOrderIx,
} from "@/core/ixBuilders/PlaceAttachedConditionalOrder";
import {
  buildPlaceLimitOrderWithConditionalsIx,
  type PlaceLimitOrderWithConditionalsIx,
} from "@/core/ixBuilders/PlaceLimitOrderWithConditionals";
import {
  buildPlacePositionConditionalOrderIx,
  type PlacePositionConditionalOrderIx,
} from "@/core/ixBuilders/PlacePositionConditionalOrder";
import type { PhoenixOrderPacketBuilders } from "@/orderPackets";
import { getPhoenixConditionalOrdersAddress } from "@/pdas";
import { MarginType, ticks, toMaxPositions } from "@/primitives";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  EmberStateAddress,
  EmberVaultAddress,
  GlobalConfigurationAddress,
  GlobalVaultAddress,
  GlobalTraderIndexAddressArray,
  LogAuthorityAddress,
  MarketAddress,
  MintAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  Symbol,
  TokenAccountAddress,
  TraderAddress,
  WithdrawQueueAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { Address } from "@solana/kit";
import {
  buildTriggerOrderParams,
  normalizeTriggerOrderParams,
} from "../conditionalOrders";
import {
  buildDepositIxsResolved,
  buildDepositFundsIxResolved,
  buildEmberDepositIxResolved,
  buildEmberWithdrawIxResolved,
  buildTransferCollateralChildToParentIxResolved,
  buildTransferCollateralIxResolved,
  buildWithdrawFundsIxResolved,
  buildWithdrawIxsResolved,
} from "./collateral";
import { buildCreateEscrowRequestIxResolved } from "./escrow";
import {
  buildCancelAllIxResolved,
  buildCancelOrdersByIdIxResolved,
  buildPlaceLimitOrderIxResolved,
  buildPlaceMarketOrderIxResolved,
  buildPlacePostOnlyOrderIxResolved,
  buildPlaceStopLossIxResolved,
} from "./orders";
import {
  buildCancelStopLossIxResolved,
  buildDelegateTraderIxResolved,
  buildRegisterTraderIxResolved,
  buildSyncParentToChildIxResolved,
} from "./trader";
import type {
  BuildRegisterTraderParams,
  ClientCancelAllInput,
  ClientCancelOrdersByIdInput,
  ClientCancelStopLossInput,
  ClientCreateConditionalOrdersAccountInput,
  ClientCreateEscrowRequestInput,
  ClientDepositInput,
  ClientDelegateTraderInput,
  ClientPlaceAttachedConditionalOrderInput,
  ClientPlaceLimitOrderWithConditionalsInput,
  ClientPlaceOrderInput,
  ClientPlacePositionConditionalOrderInput,
  ClientPlaceStopLossInput,
  ClientSyncParentToChildInput,
  ClientTransferCollateralChildToParentInput,
  ClientTransferCollateralInput,
  ClientWithdrawInput,
  DepositIxsResult,
  PhoenixIxClient,
  ResolvedPlaceOrderContext,
  WithdrawIxsResult,
} from "./types";
import type {
  ImmediateOrCancelOrderPacket,
  LimitOrderPacket,
  PostOnlyOrderPacket,
} from "@/primitives";

export interface PhoenixIxResolvedMarketContext {
  assetId: number;
  marketAddress: MarketAddress;
  splineCollection: SplineCollectionAddress;
  tickSize?: number;
  baseLotsDecimals?: number;
}

export interface PhoenixIxExchangeInstructionAccounts {
  phoenixProgramAddress: PhoenixProgramAddress;
  logAuthorityAddress: LogAuthorityAddress;
  globalConfigurationAddress: GlobalConfigurationAddress;
  canonicalMint: MintAddress;
  usdcMint: MintAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalVault: GlobalVaultAddress;
  withdrawQueue: WithdrawQueueAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

export interface PhoenixIxEmberAccounts {
  emberState: EmberStateAddress;
  emberVault: EmberVaultAddress;
}

export interface PhoenixIxOperationContext {
  orderPackets: PhoenixOrderPacketBuilders;
  phoenixProgramAddress: PhoenixProgramAddress;
  resolvePlaceOrderContext: (params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    traderPdaIndex?: number;
    traderSubaccountIndex?: number;
  }) => Promise<ResolvedPlaceOrderContext>;
  resolveMarketContext: (
    symbol: Symbol
  ) => Promise<PhoenixIxResolvedMarketContext>;
  resolveExchangeInstructionAccounts: () => Promise<PhoenixIxExchangeInstructionAccounts>;
  resolveTraderAccount: (params: {
    authority: Authority;
    traderPdaIndex?: number;
    traderSubaccountIndex?: number;
  }) => Promise<TraderAddress>;
  resolveTraderTokenAccounts: (
    authority: Authority,
    params: {
      canonicalMint: MintAddress;
      usdcMint: MintAddress;
    }
  ) => Promise<{
    phoenixTokenAccount: TokenAccountAddress;
    usdcTokenAccount: TokenAccountAddress;
  }>;
  resolveEmberAccounts: () => Promise<PhoenixIxEmberAccounts>;
  resolveLogAuthorityAddress: () => Promise<LogAuthorityAddress>;
  resolveGlobalConfigurationAddress: () => Promise<GlobalConfigurationAddress>;
  resolveEscrowAddress: (authority: Address) => Promise<Address>;
  resolvePermissionAddress: (params: {
    permissionAuthority: Address;
    delegatedKey: Address;
  }) => Promise<Address>;
  resolveStopLossAddress: (params: {
    traderAccount: TraderAddress;
    assetId: bigint;
  }) => Promise<Address>;
  maybeWrapOrderIx: <TIx extends InstructionsWithAccountsAndData>(
    instruction: TIx,
    authority: Authority
  ) => Promise<TIx>;
  accountExists: (address: Address) => Promise<boolean>;
}

const priceToTicks = (
  priceUsd: bigint | number,
  tickSize: number,
  baseLotsDecimals: number
): bigint => {
  const price = typeof priceUsd === "bigint" ? Number(priceUsd) : priceUsd;
  const priceTicks =
    (price * 1_000_000) / (tickSize * Math.pow(10, baseLotsDecimals));
  return BigInt(Math.floor(priceTicks));
};

export const createPhoenixIxOperations = (
  context: PhoenixIxOperationContext
): PhoenixIxClient => {
  const buildWrappedOrder = async <
    TPacket extends
      | LimitOrderPacket
      | ImmediateOrCancelOrderPacket
      | PostOnlyOrderPacket,
    TIx extends InstructionsWithAccountsAndData,
  >(
    params: ClientPlaceOrderInput<TPacket>,
    buildInstruction: (
      input: ResolvedPlaceOrderContext & { orderPacket: TPacket }
    ) => TIx
  ): Promise<TIx> =>
    context.maybeWrapOrderIx(
      buildInstruction({
        ...(await context.resolvePlaceOrderContext(params)),
        orderPacket: params.orderPacket,
      }),
      params.authority
    );

  const buildWrappedLimitOrder = (
    params: ClientPlaceOrderInput<LimitOrderPacket>
  ) => buildWrappedOrder(params, buildPlaceLimitOrderIxResolved);

  const buildWrappedMarketOrder = (
    params: ClientPlaceOrderInput<ImmediateOrCancelOrderPacket>
  ) => buildWrappedOrder(params, buildPlaceMarketOrderIxResolved);

  const buildWrappedPostOnlyOrder = (
    params: ClientPlaceOrderInput<PostOnlyOrderPacket>
  ) => buildWrappedOrder(params, buildPlacePostOnlyOrderIxResolved);

  const resolveConditionalOrdersAccount = async (
    traderAccount: TraderAddress
  ) =>
    getPhoenixConditionalOrdersAddress({
      traderAccount,
      phoenixProgramAddress: context.phoenixProgramAddress,
    });

  const buildCreateConditionalOrdersAccount = async (
    params: ClientCreateConditionalOrdersAccountInput
  ): Promise<CreateConditionalOrdersAccountIx> => {
    const [logAuthorityAddress, globalConfigurationAddress, traderAccount] =
      await Promise.all([
        context.resolveLogAuthorityAddress(),
        context.resolveGlobalConfigurationAddress(),
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
        }),
      ]);

    return buildCreateConditionalOrdersAccountIx({
      programAddress: context.phoenixProgramAddress,
      logAuthorityAddress,
      globalConfigurationAddress,
      payer: params.payer ?? params.authority,
      traderWallet: params.positionAuthority ?? params.authority,
      traderAccount,
      traderConditionalOrders:
        await resolveConditionalOrdersAccount(traderAccount),
      capacity: params.capacity ?? 32,
    });
  };

  const buildWrappedPositionConditionalOrder = async (
    params: ClientPlacePositionConditionalOrderInput
  ): Promise<PlacePositionConditionalOrderIx> => {
    const [exchangeAccounts, market, traderAccount] = await Promise.all([
      context.resolveExchangeInstructionAccounts(),
      context.resolveMarketContext(params.symbol),
      context.resolveTraderAccount({
        authority: params.authority,
        traderPdaIndex: params.traderPdaIndex,
        traderSubaccountIndex: params.traderSubaccountIndex,
      }),
    ]);

    return context.maybeWrapOrderIx(
      buildPlacePositionConditionalOrderIx({
        programAddress: exchangeAccounts.phoenixProgramAddress,
        logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
        globalConfigurationAddress: exchangeAccounts.globalConfigurationAddress,
        payer: params.payer ?? params.authority,
        traderAccount,
        perpAssetMap: exchangeAccounts.perpAssetMap,
        globalTraderIndex: exchangeAccounts.globalTraderIndex,
        activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        orderbook: market.marketAddress,
        splineCollection: market.splineCollection,
        traderWallet: params.positionAuthority ?? params.authority,
        traderConditionalOrders:
          await resolveConditionalOrdersAccount(traderAccount),
        assetId: market.assetId,
        greaterTriggerOrder: normalizeTriggerOrderParams(
          params.greaterTriggerOrder
        ),
        lessTriggerOrder: normalizeTriggerOrderParams(params.lessTriggerOrder),
        sizeBaseLots: params.sizeBaseLots ?? null,
        sizePercent: params.sizePercent ?? null,
      }),
      params.authority
    );
  };

  const buildWrappedAttachedConditionalOrder = async (
    params: ClientPlaceAttachedConditionalOrderInput
  ): Promise<PlaceAttachedConditionalOrderIx> => {
    const [exchangeAccounts, market, traderAccount] = await Promise.all([
      context.resolveExchangeInstructionAccounts(),
      context.resolveMarketContext(params.symbol),
      context.resolveTraderAccount({
        authority: params.authority,
        traderPdaIndex: params.traderPdaIndex,
        traderSubaccountIndex: params.traderSubaccountIndex,
      }),
    ]);

    return context.maybeWrapOrderIx(
      buildPlaceAttachedConditionalOrderIx({
        programAddress: exchangeAccounts.phoenixProgramAddress,
        logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
        globalConfigurationAddress: exchangeAccounts.globalConfigurationAddress,
        traderAccount,
        traderWallet: params.positionAuthority ?? params.authority,
        orderbook: market.marketAddress,
        traderConditionalOrders:
          await resolveConditionalOrdersAccount(traderAccount),
        payer: params.payer ?? params.authority,
        globalTraderIndex: exchangeAccounts.globalTraderIndex,
        activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        orderId: params.orderId,
        assetId: market.assetId,
        greaterTriggerOrder: normalizeTriggerOrderParams(
          params.greaterTriggerOrder
        ),
        lessTriggerOrder: normalizeTriggerOrderParams(params.lessTriggerOrder),
      }),
      params.authority
    );
  };

  const buildWrappedLimitOrderWithConditionals = async (
    params: ClientPlaceLimitOrderWithConditionalsInput
  ): Promise<PlaceLimitOrderWithConditionalsIx> => {
    const [exchangeAccounts, market, traderAccount] = await Promise.all([
      context.resolveExchangeInstructionAccounts(),
      context.resolveMarketContext(params.symbol),
      context.resolveTraderAccount({
        authority: params.authority,
        traderPdaIndex: params.traderPdaIndex,
        traderSubaccountIndex: params.traderSubaccountIndex,
      }),
    ]);

    return context.maybeWrapOrderIx(
      buildPlaceLimitOrderWithConditionalsIx({
        programAddress: exchangeAccounts.phoenixProgramAddress,
        logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
        globalConfigurationAddress: exchangeAccounts.globalConfigurationAddress,
        traderWallet: params.positionAuthority ?? params.authority,
        traderAccount,
        perpAssetMap: exchangeAccounts.perpAssetMap,
        globalTraderIndex: exchangeAccounts.globalTraderIndex,
        activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        orderbook: market.marketAddress,
        splineCollection: market.splineCollection,
        payer: params.payer ?? params.authority,
        traderConditionalOrders:
          await resolveConditionalOrdersAccount(traderAccount),
        orderPacket: params.orderPacket,
        slot: params.slot,
        greaterTriggerOrder: normalizeTriggerOrderParams(
          params.greaterTriggerOrder
        ),
        lessTriggerOrder: normalizeTriggerOrderParams(params.lessTriggerOrder),
      }),
      params.authority
    );
  };

  return {
    orderPackets: context.orderPackets,
    buildCancelAll: async (params: ClientCancelAllInput) =>
      buildCancelAllIxResolved(await context.resolvePlaceOrderContext(params)),
    buildCancelOrdersById: async (params: ClientCancelOrdersByIdInput) => {
      const [exchangeAccounts, market, traderAccount] = await Promise.all([
        context.resolveExchangeInstructionAccounts(),
        context.resolveMarketContext(params.symbol),
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
        }),
      ]);
      const { tickSize, baseLotsDecimals } = market;
      if (tickSize === undefined) {
        throw new Error(
          `Market metadata for ${params.symbol} is missing tick size`
        );
      }
      if (baseLotsDecimals === undefined) {
        throw new Error(
          `Market metadata for ${params.symbol} is missing base lot decimals`
        );
      }

      return buildCancelOrdersByIdIxResolved({
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          perpAssetMap: exchangeAccounts.perpAssetMap,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
          activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        },
        market: {
          marketAddress: market.marketAddress,
          splineCollection: market.splineCollection,
        },
        trader: {
          authority: params.authority,
          positionAuthority: params.positionAuthority,
          traderAccount,
        },
        orderIds: params.orders.map((order) => ({
          nodePointer: null,
          orderId: {
            priceInTicks: ticks(
              priceToTicks(order.price, tickSize, baseLotsDecimals)
            ),
            orderSequenceNumber: BigInt(order.orderSequenceNumber),
          },
        })),
      });
    },
    buildCancelStopLoss: async (params: ClientCancelStopLossInput) => {
      const [exchangeAccounts, market, traderAccount] = await Promise.all([
        context.resolveExchangeInstructionAccounts(),
        context.resolveMarketContext(params.symbol),
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
        }),
      ]);
      const stopLossAccount = await context.resolveStopLossAddress({
        traderAccount,
        assetId: BigInt(market.assetId),
      });

      return buildCancelStopLossIxResolved({
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
        },
        trader: {
          authority: params.authority,
          funder: params.funder,
          traderAccount,
          stopLossAccount,
        },
        executionDirection: params.executionDirection,
      });
    },
    buildCreateConditionalOrdersAccount,
    buildCreateEscrowRequest: async (
      params: ClientCreateEscrowRequestInput
    ) => {
      const exchangeAccounts =
        await context.resolveExchangeInstructionAccounts();
      const senderPdaIndex = params.senderPdaIndex ?? 0;
      const senderSubaccountIndex = params.senderSubaccountIndex ?? 0;
      const receiverPdaIndex = params.receiverPdaIndex ?? 0;
      const receiverSubaccountIndex = params.receiverSubaccountIndex ?? 0;

      const [
        senderTraderAccount,
        receiverTraderAccount,
        receiverEscrow,
        permissionAccount,
      ] = await Promise.all([
        context.resolveTraderAccount({
          authority: params.senderAuthority,
          traderPdaIndex: senderPdaIndex,
          traderSubaccountIndex: senderSubaccountIndex,
        }),
        context.resolveTraderAccount({
          authority: params.receiverAuthority,
          traderPdaIndex: receiverPdaIndex,
          traderSubaccountIndex: receiverSubaccountIndex,
        }),
        context.resolveEscrowAddress(params.receiverAuthority),
        context.resolvePermissionAddress({
          permissionAuthority: params.receiverAuthority,
          delegatedKey: params.senderAuthority,
        }),
      ]);

      return buildCreateEscrowRequestIxResolved({
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          perpAssetMap: exchangeAccounts.perpAssetMap,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
          activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        },
        sender: {
          authority: params.senderAuthority,
          traderAccount: senderTraderAccount,
          traderPdaIndex: senderPdaIndex,
          traderSubaccountIndex: senderSubaccountIndex,
        },
        receiver: {
          authority: params.receiverAuthority,
          traderAccount: receiverTraderAccount,
          escrowAddress: receiverEscrow,
          permissionAddress: permissionAccount,
          traderPdaIndex: receiverPdaIndex,
          traderSubaccountIndex: receiverSubaccountIndex,
        },
        actions: params.actions,
        lastValidSlot: params.lastValidSlot,
      });
    },
    buildDelegateTrader: async (params: ClientDelegateTraderInput) =>
      buildDelegateTraderIxResolved({
        exchange: {
          phoenixProgramAddress: context.phoenixProgramAddress,
          logAuthorityAddress: await context.resolveLogAuthorityAddress(),
          globalConfigurationAddress:
            await context.resolveGlobalConfigurationAddress(),
        },
        trader: {
          authority: params.traderWallet,
          traderAccount: await context.resolveTraderAccount({
            authority: params.traderWallet,
            traderPdaIndex: params.traderPdaIndex,
            traderSubaccountIndex: params.traderSubaccountIndex,
          }),
        },
        newPositionAuthority: params.newPositionAuthority,
      }),
    buildPlaceLimitOrder: buildWrappedLimitOrder,
    buildPlaceMarketOrder: buildWrappedMarketOrder,
    buildPlacePositionConditionalOrder: buildWrappedPositionConditionalOrder,
    buildPlaceAttachedConditionalOrder: buildWrappedAttachedConditionalOrder,
    buildPlaceLimitOrderWithConditionals:
      buildWrappedLimitOrderWithConditionals,
    buildPlacePostOnlyOrder: buildWrappedPostOnlyOrder,
    placeLimitOrder: buildWrappedLimitOrder,
    placeMarketOrder: buildWrappedMarketOrder,
    placePositionConditionalOrder: buildWrappedPositionConditionalOrder,
    placePostOnlyOrder: buildWrappedPostOnlyOrder,
    buildDepositFunds: async (params: ClientDepositInput) => {
      const exchangeAccounts =
        await context.resolveExchangeInstructionAccounts();
      const [traderAccount, { phoenixTokenAccount }] = await Promise.all([
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
        }),
        context.resolveTraderTokenAccounts(params.authority, {
          canonicalMint: exchangeAccounts.canonicalMint,
          usdcMint: exchangeAccounts.usdcMint,
        }),
      ]);

      return buildDepositFundsIxResolved({
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          canonicalMint: exchangeAccounts.canonicalMint,
          globalVault: exchangeAccounts.globalVault,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
          activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        },
        trader: {
          authority: params.authority,
          traderAccount,
          traderTokenAccount: phoenixTokenAccount,
          permissionAddress: params.permissionAddress,
        },
        amount: params.amount,
      });
    },
    buildWithdrawFunds: async (params: ClientWithdrawInput) => {
      const exchangeAccounts =
        await context.resolveExchangeInstructionAccounts();
      const [traderAccount, { phoenixTokenAccount }] = await Promise.all([
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
        }),
        context.resolveTraderTokenAccounts(params.authority, {
          canonicalMint: exchangeAccounts.canonicalMint,
          usdcMint: exchangeAccounts.usdcMint,
        }),
      ]);

      return buildWithdrawFundsIxResolved({
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          canonicalMint: exchangeAccounts.canonicalMint,
          perpAssetMap: exchangeAccounts.perpAssetMap,
          globalVault: exchangeAccounts.globalVault,
          withdrawQueue: exchangeAccounts.withdrawQueue,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
          activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        },
        trader: {
          authority: params.authority,
          traderAccount,
          destinationTokenAccount: phoenixTokenAccount,
        },
        amount: params.amount,
      });
    },
    buildTransferCollateral: async (params: ClientTransferCollateralInput) => {
      const exchangeAccounts =
        await context.resolveExchangeInstructionAccounts();
      const [srcTraderAccount, dstTraderAccount] = await Promise.all([
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.srcSubaccountIndex,
        }),
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.dstSubaccountIndex,
        }),
      ]);

      return buildTransferCollateralIxResolved({
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          perpAssetMap: exchangeAccounts.perpAssetMap,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
          activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        },
        trader: {
          authority: params.authority,
          positionAuthority: params.positionAuthority,
          srcTraderAccount,
          dstTraderAccount,
        },
        amount: params.amount,
      });
    },
    buildTransferCollateralChildToParent: async (
      params: ClientTransferCollateralChildToParentInput
    ) => {
      const exchangeAccounts =
        await context.resolveExchangeInstructionAccounts();
      const [childTraderAccount, parentTraderAccount] = await Promise.all([
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.childSubaccountIndex,
        }),
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: 0,
        }),
      ]);

      return buildTransferCollateralChildToParentIxResolved({
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          perpAssetMap: exchangeAccounts.perpAssetMap,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
          activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
        },
        trader: {
          authority: params.authority,
          positionAuthority: params.positionAuthority,
          childTraderAccount,
          parentTraderAccount,
        },
      });
    },
    buildEmberDeposit: async (params: ClientDepositInput) => {
      const [exchangeAccounts, emberAccounts] = await Promise.all([
        context.resolveExchangeInstructionAccounts(),
        context.resolveEmberAccounts(),
      ]);
      const { phoenixTokenAccount, usdcTokenAccount } =
        await context.resolveTraderTokenAccounts(params.authority, {
          canonicalMint: exchangeAccounts.canonicalMint,
          usdcMint: exchangeAccounts.usdcMint,
        });

      return buildEmberDepositIxResolved({
        exchange: {
          usdcMint: exchangeAccounts.usdcMint,
          canonicalMint: exchangeAccounts.canonicalMint,
          emberState: emberAccounts.emberState,
          emberVault: emberAccounts.emberVault,
        },
        trader: {
          authority: params.authority,
          usdcTokenAccount,
          phoenixTokenAccount,
        },
        amount: params.amount,
      });
    },
    buildEmberWithdraw: async (params: ClientWithdrawInput) => {
      const [exchangeAccounts, emberAccounts] = await Promise.all([
        context.resolveExchangeInstructionAccounts(),
        context.resolveEmberAccounts(),
      ]);
      const { phoenixTokenAccount, usdcTokenAccount } =
        await context.resolveTraderTokenAccounts(params.authority, {
          canonicalMint: exchangeAccounts.canonicalMint,
          usdcMint: exchangeAccounts.usdcMint,
        });

      return buildEmberWithdrawIxResolved({
        exchange: {
          usdcMint: exchangeAccounts.usdcMint,
          canonicalMint: exchangeAccounts.canonicalMint,
          emberState: emberAccounts.emberState,
          emberVault: emberAccounts.emberVault,
        },
        trader: {
          authority: params.authority,
          usdcTokenAccount,
          phoenixTokenAccount,
        },
        amount: params.amount,
      });
    },
    buildPlaceStopLoss: async (params: ClientPlaceStopLossInput) => {
      const [exchangeAccounts, market, traderAccount] = await Promise.all([
        context.resolveExchangeInstructionAccounts(),
        context.resolveMarketContext(params.symbol),
        context.resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
        }),
      ]);
      const stopLossAccount = await context.resolveStopLossAddress({
        traderAccount,
        assetId: BigInt(market.assetId),
      });
      const triggerOrder = buildTriggerOrderParams({
        triggerDirection: params.executionDirection,
        tradeSide: params.tradeSide,
        orderKind: params.orderKind,
        triggerPrice: ticks(params.triggerPrice),
        executionPrice:
          params.executionPrice == null
            ? params.executionPrice
            : ticks(params.executionPrice),
        slippageBps: params.slippageBps,
      });

      return context.maybeWrapOrderIx(
        buildPlaceStopLossIxResolved({
          exchange: {
            phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
            logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
            globalConfigurationAddress:
              exchangeAccounts.globalConfigurationAddress,
            perpAssetMap: exchangeAccounts.perpAssetMap,
            globalTraderIndex: exchangeAccounts.globalTraderIndex,
            activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
          },
          market: {
            marketAddress: market.marketAddress,
            splineCollection: market.splineCollection,
          },
          trader: {
            authority: params.authority,
            funder: params.payer,
            positionAuthority: params.positionAuthority,
            traderAccount,
          },
          stopLossAccount,
          triggerPrice: triggerOrder.triggerPrice,
          executionPrice: triggerOrder.executionPrice,
          tradeSide: triggerOrder.tradeSide,
          executionDirection: triggerOrder.triggerDirection,
          orderKind: triggerOrder.orderKind,
        }),
        params.authority
      );
    },
    buildRegisterTrader: async (params: BuildRegisterTraderParams) => {
      const traderPdaIndex = params.traderPdaIndex ?? 0;
      if (traderPdaIndex !== 0) {
        throw new Error("Non-zero traderPdaIndex is not yet supported.");
      }

      const traderSubaccountIndex =
        params.marginType === MarginType.Isolated
          ? params.traderSubaccountIndex
          : 0;

      if (params.marginType === MarginType.Isolated) {
        if (traderSubaccountIndex === undefined) {
          throw new Error(
            "Trader subaccount index is required for isolated margin"
          );
        }
        if (traderSubaccountIndex === 0) {
          throw new Error(
            "Trader subaccount index 0 is reserved for cross margin"
          );
        }
      }

      if ((traderSubaccountIndex ?? 0) > MAX_SUBACCOUNTS) {
        throw new Error("Trader subaccount index is out of bounds");
      }

      const traderAccount = await context.resolveTraderAccount({
        authority: params.authority,
        traderPdaIndex,
        traderSubaccountIndex: traderSubaccountIndex ?? 0,
      });

      if (await context.accountExists(traderAccount)) {
        throw new Error(
          params.marginType === MarginType.Isolated
            ? "Isolated margin trader account already exists"
            : "Cross-margin trader account already exists"
        );
      }

      return buildRegisterTraderIxResolved({
        exchange: {
          phoenixProgramAddress: context.phoenixProgramAddress,
          logAuthorityAddress: await context.resolveLogAuthorityAddress(),
          globalConfigurationAddress:
            await context.resolveGlobalConfigurationAddress(),
        },
        trader: {
          payer: params.feePayer ?? params.authority,
          authority: params.authority,
          traderAccount,
        },
        maxPositions: toMaxPositions(params.marginType),
        traderPdaIndex,
        traderSubaccountIndex: traderSubaccountIndex ?? 0,
      });
    },
    buildSyncParentToChild: async (params: ClientSyncParentToChildInput) => {
      const exchangeAccounts =
        await context.resolveExchangeInstructionAccounts();
      const [parentTraderAccount, childTraderAccount] = await Promise.all([
        context.resolveTraderAccount({
          authority: params.traderWallet,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: 0,
        }),
        context.resolveTraderAccount({
          authority: params.traderWallet,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
        }),
      ]);

      return buildSyncParentToChildIxResolved({
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
        },
        trader: {
          authority: params.traderWallet,
          parentTraderAccount,
          childTraderAccount,
        },
      });
    },
    buildDepositIxs: async (
      params: ClientDepositInput
    ): Promise<DepositIxsResult> => {
      const [exchangeAccounts, emberAccounts, traderAccount] =
        await Promise.all([
          context.resolveExchangeInstructionAccounts(),
          context.resolveEmberAccounts(),
          context.resolveTraderAccount({
            authority: params.authority,
            traderPdaIndex: params.traderPdaIndex,
            traderSubaccountIndex: params.traderSubaccountIndex,
          }),
        ]);
      const { phoenixTokenAccount, usdcTokenAccount } =
        await context.resolveTraderTokenAccounts(params.authority, {
          canonicalMint: exchangeAccounts.canonicalMint,
          usdcMint: exchangeAccounts.usdcMint,
        });

      return buildDepositIxsResolved({
        feePayer: params.feePayer,
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          canonicalMint: exchangeAccounts.canonicalMint,
          usdcMint: exchangeAccounts.usdcMint,
          globalVault: exchangeAccounts.globalVault,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
          activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
          emberState: emberAccounts.emberState,
          emberVault: emberAccounts.emberVault,
        },
        trader: {
          authority: params.authority,
          traderAccount,
          usdcTokenAccount,
          phoenixTokenAccount,
        },
        amount: params.amount,
      });
    },
    buildWithdrawIxs: async (
      params: ClientWithdrawInput
    ): Promise<WithdrawIxsResult> => {
      const [exchangeAccounts, emberAccounts, traderAccount] =
        await Promise.all([
          context.resolveExchangeInstructionAccounts(),
          context.resolveEmberAccounts(),
          context.resolveTraderAccount({
            authority: params.authority,
            traderPdaIndex: params.traderPdaIndex,
            traderSubaccountIndex: params.traderSubaccountIndex,
          }),
        ]);
      const { phoenixTokenAccount, usdcTokenAccount } =
        await context.resolveTraderTokenAccounts(params.authority, {
          canonicalMint: exchangeAccounts.canonicalMint,
          usdcMint: exchangeAccounts.usdcMint,
        });

      return buildWithdrawIxsResolved({
        feePayer: params.feePayer,
        exchange: {
          phoenixProgramAddress: exchangeAccounts.phoenixProgramAddress,
          logAuthorityAddress: exchangeAccounts.logAuthorityAddress,
          globalConfigurationAddress:
            exchangeAccounts.globalConfigurationAddress,
          canonicalMint: exchangeAccounts.canonicalMint,
          usdcMint: exchangeAccounts.usdcMint,
          perpAssetMap: exchangeAccounts.perpAssetMap,
          globalVault: exchangeAccounts.globalVault,
          withdrawQueue: exchangeAccounts.withdrawQueue,
          globalTraderIndex: exchangeAccounts.globalTraderIndex,
          activeTraderBuffer: exchangeAccounts.activeTraderBuffer,
          emberState: emberAccounts.emberState,
          emberVault: emberAccounts.emberVault,
        },
        trader: {
          authority: params.authority,
          traderAccount,
          usdcTokenAccount,
          phoenixTokenAccount,
        },
        amount: params.amount,
      });
    },
  };
};
