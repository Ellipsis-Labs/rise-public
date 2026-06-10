import {
  type PhoenixAccountExistenceClient,
  type PhoenixInstructionClient,
  type PhoenixTransactionClient,
  type SendInstructionOptions,
} from "@/core/clientTypes";
import {
  clientPhoenixInstructionAddresses,
  getPhoenixProgramAddress,
} from "@/core/constants";
import {
  fetchRequiredAccounts,
  getClientTraderAddresses,
  getMarketAddressForSymbol,
  getMarketMetadataForSymbol,
} from "@/core/helpers";
import type { PhoenixOrderPacketBuilders } from "@/orderPackets";
import {
  type Authority,
  type BaseLots,
  type ConditionalOrderPacket,
  type Direction,
  type FIFOOrderId,
  type MintAddress,
  type Side,
  type StopLossOrderKind,
  type Symbol,
  type TokenAccountAddress,
} from "@/primitives";
import type {
  ImmediateOrCancelOrderPacket,
  LimitOrderPacket,
  PostOnlyOrderPacket,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import {
  getEmberVaultAddress,
  getPhoenixConditionalOrdersAddress,
  getPhoenixEscrowAddress,
  getPhoenixGlobalVaultAddress,
  getPhoenixPermissionAddress,
  getPhoenixSplineCollectionAddress,
  getPhoenixStopLossAddress,
  getPhoenixTraderSubaccountAddress,
  getPhoenixTraderTokenAccountAddress,
} from "@/pdas";
import type { Address, Signature } from "@solana/kit";
import { type CancelAllIx } from "./core/ixBuilders/CancelAll";
import { type CancelOrdersByIdIx } from "./core/ixBuilders/CancelOrdersById";
import { type CancelStopLossIx } from "./core/ixBuilders/CancelStopLoss";
import {
  buildCancelConditionalOrderIx,
  type CancelConditionalOrderIx,
} from "./core/ixBuilders/CancelConditionalOrder";
import {
  buildCreateConditionalOrdersAccountIx,
  type CreateConditionalOrdersAccountIx,
} from "./core/ixBuilders/CreateConditionalOrdersAccount";
import {
  type CreateEscrowRequestIx,
  type EscrowAction,
} from "./core/ixBuilders/CreateEscrowRequest";
import { type DelegateTraderIx } from "./core/ixBuilders/DelegateTrader";
import { type DepositFundsIx } from "./core/ixBuilders/DepositFunds";
import { type EmberDepositIx } from "./core/ixBuilders/EmberDeposit";
import { type EmberWithdrawIx } from "./core/ixBuilders/EmberWithdraw";
import { type PlaceLimitOrderIx } from "./core/ixBuilders/PlaceLimitOrder";
import { type PlaceAttachedConditionalOrderIx } from "./core/ixBuilders/PlaceAttachedConditionalOrder";
import { type PlaceLimitOrderWithConditionalsIx } from "./core/ixBuilders/PlaceLimitOrderWithConditionals";
import { type PlaceMarketOrderIx } from "./core/ixBuilders/PlaceMarketOrder";
import { type PlacePostOnlyOrderIx } from "./core/ixBuilders/PlacePostOnlyOrder";
import { type PlacePositionConditionalOrderIx } from "./core/ixBuilders/PlacePositionConditionalOrder";
import { type PlaceStopLossIx } from "./core/ixBuilders/PlaceStopLoss";
import { type RegisterTraderIx } from "./core/ixBuilders/RegisterTrader";
import type { OnboardTraderDelegatedIx } from "./core/ixBuilders/OnboardTraderDelegated";
import { type SyncParentToChildIx } from "./core/ixBuilders/SyncParentToChild";
import { type TransferCollateralChildToParentIx } from "./core/ixBuilders/TransferCollateralChildToParent";
import { type TransferCollateralIx } from "./core/ixBuilders/TransferCollateral";
import { type WithdrawFundsIx } from "./core/ixBuilders/WithdrawFunds";
import {
  buildDelegateTraderIxResolved,
  buildEmberWithdrawIxResolved,
  type BuildRegisterTraderParams,
} from "./ixs";
import {
  createPhoenixIxOperations,
  type PhoenixIxOperationContext,
} from "./ixs/operations";
import { isFlightClient } from "./flight/client.js";
import type { TriggerOrderParamsInput } from "./conditionalOrders";

export type { BuildRegisterTraderParams } from "./ixs";

type Commitment = SendInstructionOptions["commitment"];
type SendableInstruction = Parameters<
  PhoenixTransactionClient["sendAndConfirmFromInstruction"]
>[0];
type LegacyInstructionClient = PhoenixInstructionClient &
  Partial<PhoenixAccountExistenceClient>;

const unsupportedOrderPackets: PhoenixOrderPacketBuilders = {
  buildLimitOrderPacket: async () => {
    throw new Error(
      "orderPackets are unavailable from the standalone builder helpers"
    );
  },
  buildMarketOrderPacket: async () => {
    throw new Error(
      "orderPackets are unavailable from the standalone builder helpers"
    );
  },
};

const sendBuiltInstruction = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  instruction: Promise<SendableInstruction>,
  options?: SendInstructionOptions
): Promise<Signature> =>
  client.sendAndConfirmFromInstruction(await instruction, options);

const createLegacyPhoenixIxOperationContext = (
  client: LegacyInstructionClient
): PhoenixIxOperationContext => {
  const phoenixProgramAddress = client.addresses.phoenixProgramAddress;
  const requiredAccountsPromise = fetchRequiredAccounts(client);

  const resolveExchangeInstructionAccounts = async () => {
    const { globalConfiguration, arenaAddresses, globalTraderIndexAddresses } =
      await requiredAccountsPromise;
    const globalVault = await getPhoenixGlobalVaultAddress(
      globalConfiguration.canonicalTokenMintKey,
      phoenixProgramAddress
    );

    return {
      phoenixProgramAddress,
      logAuthorityAddress: client.addresses.logAuthorityAddress,
      globalConfigurationAddress: client.addresses.globalConfigurationAddress,
      canonicalMint: globalConfiguration.canonicalTokenMintKey,
      usdcMint: client.addresses.usdcMintAddress,
      perpAssetMap: globalConfiguration.perpAssetMapKey,
      globalVault,
      withdrawQueue: globalConfiguration.withdrawQueueKey,
      globalTraderIndex: globalTraderIndexAddresses,
      activeTraderBuffer: arenaAddresses,
    };
  };

  const resolveMarketContext = async (symbol: Symbol) => {
    const metadata = await getMarketMetadataForSymbol(symbol, client);
    if (
      metadata.tickSize === undefined ||
      metadata.baseLotDecimals === undefined
    ) {
      throw new Error(
        `Market metadata for ${symbol} is missing tick size or base lot decimals`
      );
    }

    return {
      assetId: Number(metadata.assetId),
      marketAddress: metadata.marketAddress as never,
      splineCollection: (metadata.splineAddress ??
        (await getPhoenixSplineCollectionAddress(
          metadata.marketAddress,
          phoenixProgramAddress
        ))) as never,
      tickSize: metadata.tickSize,
      baseLotsDecimals: metadata.baseLotDecimals,
    };
  };

  const resolveTraderAccount = async (params: {
    authority: Authority;
    traderPdaIndex?: number;
    traderSubaccountIndex?: number;
  }) =>
    getPhoenixTraderSubaccountAddress({
      authority: params.authority,
      traderPdaIndex: params.traderPdaIndex ?? 0,
      subaccountIndex: params.traderSubaccountIndex ?? 0,
      phoenixProgramAddress,
    });

  const resolveTraderTokenAccounts = async (
    authority: Authority,
    params: {
      canonicalMint: MintAddress;
      usdcMint: MintAddress;
    }
  ): Promise<{
    phoenixTokenAccount: TokenAccountAddress;
    usdcTokenAccount: TokenAccountAddress;
  }> => {
    const [phoenixTokenAccount, usdcTokenAccount] = await Promise.all([
      getPhoenixTraderTokenAccountAddress(authority, params.canonicalMint),
      getPhoenixTraderTokenAccountAddress(authority, params.usdcMint),
    ]);

    return {
      phoenixTokenAccount,
      usdcTokenAccount,
    };
  };

  const maybeWrapOrderIx = async <TIx extends InstructionsWithAccountsAndData>(
    instruction: TIx,
    authority: Authority
  ): Promise<TIx> => {
    if (!isFlightClient(client)) {
      return instruction;
    }

    return (await client.tryWrapFlightInstruction(
      instruction,
      authority
    )) as TIx;
  };

  return {
    orderPackets: unsupportedOrderPackets,
    phoenixProgramAddress,
    resolvePlaceOrderContext: async (params) => {
      const [exchangeAccounts, market, traderAccount] = await Promise.all([
        resolveExchangeInstructionAccounts(),
        resolveMarketContext(params.symbol),
        resolveTraderAccount({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex,
          traderSubaccountIndex: params.traderSubaccountIndex,
        }),
      ]);

      return {
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
      };
    },
    resolveMarketContext,
    resolveExchangeInstructionAccounts,
    resolveTraderAccount,
    resolveTraderTokenAccounts,
    resolveEmberAccounts: async () => ({
      emberState: client.addresses.emberStateAddress,
      emberVault: await getEmberVaultAddress(phoenixProgramAddress),
    }),
    resolveLogAuthorityAddress: async () =>
      client.addresses.logAuthorityAddress,
    resolveGlobalConfigurationAddress: async () =>
      client.addresses.globalConfigurationAddress,
    resolveEscrowAddress: async (authority) =>
      getPhoenixEscrowAddress(authority, phoenixProgramAddress),
    resolvePermissionAddress: async ({ permissionAuthority, delegatedKey }) =>
      getPhoenixPermissionAddress(
        permissionAuthority,
        delegatedKey,
        phoenixProgramAddress
      ),
    resolveStopLossAddress: async ({ traderAccount, assetId }) =>
      getPhoenixStopLossAddress({
        traderAccount,
        assetId,
        phoenixProgramAddress,
      }),
    maybeWrapOrderIx,
    accountExists: async (address) =>
      typeof client.accountExists === "function"
        ? client.accountExists(address)
        : false,
  };
};

const createLegacyPhoenixIxOperations = (client: LegacyInstructionClient) =>
  createPhoenixIxOperations(createLegacyPhoenixIxOperationContext(client));

const buildLegacyEmberWithdraw = async (
  params: {
    authority: Authority;
    amount: bigint | null;
  },
  client: PhoenixInstructionClient
): Promise<EmberWithdrawIx> => {
  const context = createLegacyPhoenixIxOperationContext(client);
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
};

export const buildCancelAll = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<CancelAllIx> =>
  createLegacyPhoenixIxOperations(client).buildCancelAll({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const cancelAllOrders = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
  },
  options: {
    traderPdaIndex?: number;
    commitment?: Commitment;
  } = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildCancelAll(params, client, options.traderPdaIndex ?? 0),
    options
  );

export const buildCancelOrdersById = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    orders: Array<{
      price: number | bigint;
      orderSequenceNumber: string | number;
    }>;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<CancelOrdersByIdIx> =>
  createLegacyPhoenixIxOperations(client).buildCancelOrdersById({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const cancelOrdersById = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    orders: Array<{
      price: bigint;
      orderSequenceNumber: string | number;
    }>;
  },
  options: {
    traderPdaIndex?: number;
    traderSubaccountIndex?: number;
    commitment?: Commitment;
  } = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildCancelOrdersById(
      params,
      client,
      options.traderPdaIndex ?? 0,
      options.traderSubaccountIndex ?? 0
    ),
    options
  );

export const buildCancelStopLoss = async (
  params: {
    authority: Authority;
    symbol: Symbol;
    executionDirection: Direction;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<CancelStopLossIx> =>
  createLegacyPhoenixIxOperations(client).buildCancelStopLoss({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const buildCreateEscrowRequest = async (
  params: {
    senderAuthority: Authority;
    receiverAuthority: Authority;
    actions: EscrowAction[];
    senderPdaIndex?: number;
    senderSubaccountIndex?: number;
    receiverPdaIndex?: number;
    receiverSubaccountIndex?: number;
    lastValidSlot?: bigint | null;
  },
  client: PhoenixInstructionClient
): Promise<CreateEscrowRequestIx> =>
  createLegacyPhoenixIxOperations(client).buildCreateEscrowRequest(params);

export const createEscrowRequest = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    senderAuthority: Authority;
    receiverAuthority: Authority;
    actions: EscrowAction[];
    senderPdaIndex?: number;
    senderSubaccountIndex?: number;
    receiverPdaIndex?: number;
    receiverSubaccountIndex?: number;
    lastValidSlot?: bigint | null;
  },
  options: SendInstructionOptions = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildCreateEscrowRequest(params, client),
    options
  );

export const buildDelegateTrader = async (params: {
  traderWallet: Authority;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  newPositionAuthority: Address;
  phoenixAddresses?: {
    phoenixProgramAddress: PhoenixInstructionClient["addresses"]["phoenixProgramAddress"];
    logAuthorityAddress: PhoenixInstructionClient["addresses"]["logAuthorityAddress"];
    globalConfigurationAddress: PhoenixInstructionClient["addresses"]["globalConfigurationAddress"];
  };
}): Promise<DelegateTraderIx> => {
  const traderAccount = await getPhoenixTraderSubaccountAddress({
    authority: params.traderWallet,
    traderPdaIndex: params.traderPdaIndex,
    subaccountIndex: params.traderSubaccountIndex,
    phoenixProgramAddress: params.phoenixAddresses?.phoenixProgramAddress,
  });

  return buildDelegateTraderIxResolved({
    exchange: {
      phoenixProgramAddress:
        params.phoenixAddresses?.phoenixProgramAddress ??
        getPhoenixProgramAddress(),
      logAuthorityAddress: params.phoenixAddresses?.logAuthorityAddress,
      globalConfigurationAddress:
        params.phoenixAddresses?.globalConfigurationAddress,
    },
    trader: {
      authority: params.traderWallet,
      traderAccount,
    },
    newPositionAuthority: params.newPositionAuthority,
  });
};

export const buildDepositFunds = async (
  params: {
    authority: Authority;
    amount: bigint;
    permissionAddress?: Address;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0
): Promise<DepositFundsIx> =>
  createLegacyPhoenixIxOperations(client).buildDepositFunds({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex: 0,
  });

export const depositFunds = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    authority: Authority;
    amount: bigint;
    permissionAddress?: Address;
  },
  options: {
    traderPdaIndex?: number;
    commitment?: Commitment;
  } = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildDepositFunds(params, client, options.traderPdaIndex ?? 0),
    options
  );

export const buildEmberDeposit = async (
  params: {
    authority: Authority;
    amount: bigint;
  },
  client: PhoenixInstructionClient
): Promise<EmberDepositIx> =>
  createLegacyPhoenixIxOperations(client).buildEmberDeposit(params);

export const buildEmberWithdraw = async (
  params: {
    authority: Authority;
    amount: bigint | null;
  },
  client: PhoenixInstructionClient
): Promise<EmberWithdrawIx> => buildLegacyEmberWithdraw(params, client);

export const buildPlaceLimitOrder = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    orderPacket: LimitOrderPacket;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<PlaceLimitOrderIx> =>
  createLegacyPhoenixIxOperations(client).buildPlaceLimitOrder({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const placeLimitOrder = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    orderPacket: LimitOrderPacket;
  },
  options: {
    traderPdaIndex?: number;
    commitment?: Commitment;
  } = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildPlaceLimitOrder(params, client, options.traderPdaIndex ?? 0),
    options
  );

export const buildPlaceMarketOrder = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    orderPacket: ImmediateOrCancelOrderPacket;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<PlaceMarketOrderIx> =>
  createLegacyPhoenixIxOperations(client).buildPlaceMarketOrder({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const placeMarketOrder = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    orderPacket: ImmediateOrCancelOrderPacket;
  },
  options: {
    traderPdaIndex?: number;
    traderSubaccountIndex?: number;
    commitment?: Commitment;
  } = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildPlaceMarketOrder(
      params,
      client,
      options.traderPdaIndex ?? 0,
      options.traderSubaccountIndex ?? 0
    ),
    options
  );

export const buildPlacePostOnlyOrder = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    orderPacket: PostOnlyOrderPacket;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<PlacePostOnlyOrderIx> =>
  createLegacyPhoenixIxOperations(client).buildPlacePostOnlyOrder({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const placePostOnlyOrder = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    orderPacket: PostOnlyOrderPacket;
  },
  options: {
    traderPdaIndex?: number;
    commitment?: Commitment;
  } = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildPlacePostOnlyOrder(params, client, options.traderPdaIndex ?? 0),
    options
  );

export const buildPlaceStopLoss = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    triggerPrice: bigint;
    executionPrice?: bigint;
    slippageBps?: number | null;
    tradeSide: Side;
    executionDirection: Direction;
    orderKind: StopLossOrderKind;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<PlaceStopLossIx> =>
  createLegacyPhoenixIxOperations(client).buildPlaceStopLoss({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const buildCreateConditionalOrdersAccount = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    payer?: Authority;
    capacity?: number;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<CreateConditionalOrdersAccountIx> => {
  const { globalConfiguration } = await fetchRequiredAccounts(client);
  const { traderAccount } = await getClientTraderAddresses(
    client,
    params.authority,
    globalConfiguration.canonicalTokenMintKey,
    traderPdaIndex,
    traderSubaccountIndex
  );
  const traderConditionalOrders = await getPhoenixConditionalOrdersAddress({
    traderAccount,
    phoenixProgramAddress: client.addresses.phoenixProgramAddress,
  });

  return buildCreateConditionalOrdersAccountIx({
    ...clientPhoenixInstructionAddresses(client),
    payer: params.payer ?? params.authority,
    traderWallet: params.positionAuthority ?? params.authority,
    traderAccount,
    traderConditionalOrders,
    capacity: params.capacity ?? 32,
  });
};

export const buildPlacePositionConditionalOrder = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    payer?: Authority;
    symbol: Symbol;
    greaterTriggerOrder?: TriggerOrderParamsInput | null;
    lessTriggerOrder?: TriggerOrderParamsInput | null;
    sizeBaseLots?: BaseLots | null;
    sizePercent?: number | null;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<PlacePositionConditionalOrderIx> =>
  createLegacyPhoenixIxOperations(client).buildPlacePositionConditionalOrder({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const buildPlaceAttachedConditionalOrder = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    payer?: Authority;
    symbol: Symbol;
    orderId: FIFOOrderId;
    greaterTriggerOrder?: TriggerOrderParamsInput | null;
    lessTriggerOrder?: TriggerOrderParamsInput | null;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<PlaceAttachedConditionalOrderIx> =>
  createLegacyPhoenixIxOperations(client).buildPlaceAttachedConditionalOrder({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const buildPlaceLimitOrderWithConditionals = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    payer?: Authority;
    symbol: Symbol;
    orderPacket: ConditionalOrderPacket;
    slot?: bigint;
    greaterTriggerOrder?: TriggerOrderParamsInput | null;
    lessTriggerOrder?: TriggerOrderParamsInput | null;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<PlaceLimitOrderWithConditionalsIx> =>
  createLegacyPhoenixIxOperations(client).buildPlaceLimitOrderWithConditionals({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const buildCancelConditionalOrder = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    conditionalOrderIndex: number;
    disableFirst: boolean;
    disableSecond: boolean;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<CancelConditionalOrderIx> => {
  const { globalConfiguration } = await fetchRequiredAccounts(client);
  const { traderAccount } = await getClientTraderAddresses(
    client,
    params.authority,
    globalConfiguration.canonicalTokenMintKey,
    traderPdaIndex,
    traderSubaccountIndex
  );
  const marketAddress = await getMarketAddressForSymbol(params.symbol, client);
  const traderConditionalOrders = await getPhoenixConditionalOrdersAddress({
    traderAccount,
    phoenixProgramAddress: client.addresses.phoenixProgramAddress,
  });

  return buildCancelConditionalOrderIx({
    ...clientPhoenixInstructionAddresses(client),
    traderAccount,
    traderWallet: params.positionAuthority ?? params.authority,
    orderbook: marketAddress,
    traderConditionalOrders,
    conditionalOrderIndex: params.conditionalOrderIndex,
    disableFirst: params.disableFirst,
    disableSecond: params.disableSecond,
  });
};

export const buildRegisterTrader = async (
  params: BuildRegisterTraderParams,
  client: PhoenixInstructionClient & PhoenixAccountExistenceClient
): Promise<RegisterTraderIx> =>
  createLegacyPhoenixIxOperations(client).buildRegisterTrader(params);

export const buildOnboardTraderDelegated = async (
  params: {
    authority: Authority;
    traderAuthority: Authority;
    permissionAccount: Address;
    traderPdaIndex?: number;
    traderSubaccountIndex?: number;
  },
  client: PhoenixInstructionClient
): Promise<OnboardTraderDelegatedIx> =>
  createLegacyPhoenixIxOperations(client).buildOnboardTraderDelegated(params);

export const registerTrader = async (
  client: PhoenixInstructionClient &
    PhoenixAccountExistenceClient &
    PhoenixTransactionClient,
  params: BuildRegisterTraderParams,
  options: { commitment?: Commitment } = {}
): Promise<Signature> =>
  sendBuiltInstruction(client, buildRegisterTrader(params, client), options);

export const buildSyncParentToChild = async (
  params: {
    traderWallet: Authority;
    traderPdaIndex: number;
    traderSubaccountIndex: number;
  },
  client: PhoenixInstructionClient
): Promise<SyncParentToChildIx> =>
  createLegacyPhoenixIxOperations(client).buildSyncParentToChild(params);

export const buildTransferCollateral = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    traderPdaIndex: number;
    srcSubaccountIndex: number;
    dstSubaccountIndex: number;
    amount: bigint;
  },
  client: PhoenixInstructionClient
): Promise<TransferCollateralIx> =>
  createLegacyPhoenixIxOperations(client).buildTransferCollateral(params);

export const transferCollateral = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    traderPdaIndex: number;
    srcSubaccountIndex: number;
    dstSubaccountIndex: number;
    amount: bigint;
  },
  options: { commitment?: Commitment } = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildTransferCollateral(params, client),
    options
  );

export const buildTransferCollateralChildToParent = async (
  params: {
    authority: Authority;
    positionAuthority?: Authority;
    traderPdaIndex: number;
    childSubaccountIndex: number;
  },
  client: PhoenixInstructionClient
): Promise<TransferCollateralChildToParentIx> =>
  createLegacyPhoenixIxOperations(client).buildTransferCollateralChildToParent(
    params
  );

export const buildWithdrawFunds = async (
  params: {
    authority: Authority;
    amount: bigint;
  },
  client: PhoenixInstructionClient,
  traderPdaIndex = 0,
  traderSubaccountIndex = 0
): Promise<WithdrawFundsIx> =>
  createLegacyPhoenixIxOperations(client).buildWithdrawFunds({
    ...params,
    traderPdaIndex,
    traderSubaccountIndex,
  });

export const withdrawFunds = async (
  client: PhoenixInstructionClient & PhoenixTransactionClient,
  params: {
    authority: Authority;
    amount: bigint;
  },
  options: {
    traderPdaIndex?: number;
    commitment?: Commitment;
  } = {}
): Promise<Signature> =>
  sendBuiltInstruction(
    client,
    buildWithdrawFunds(params, client, options.traderPdaIndex ?? 0),
    options
  );
