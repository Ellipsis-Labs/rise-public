import type { ExchangeInstructionContext } from "@/exchange-cache/types";
import { fetchBuilderState } from "@/flight/accounts";
import type { PhoenixFlightClientConfig } from "@/flight/client";
import { wrapInstructionWithFlight } from "@/flight/client";
import { debugRise, summarizeDebugError } from "@/internal/_debug";
import type { PhoenixOrderPacketBuilders } from "@/orderPackets";
import type { PhoenixPdaClient } from "@/pdaClient";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalConfigurationAddress,
  GlobalVaultAddress,
  GlobalTraderIndexAddressArray,
  MintAddress,
  PerpAssetMapAddress,
  Symbol,
  TraderAddress,
  WithdrawQueueAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { Address } from "@solana/kit";
import type { PhoenixExchangeMetadata } from "../exchange-cache";
import type { PhoenixRpcClient } from "../rpc";
import { buildResolvedPlaceOrderContext } from "./context";
import {
  createPhoenixIxOperations,
  type PhoenixIxResolvedMarketContext,
} from "./operations";
import type { PhoenixIxClient, ResolvedPlaceOrderContext } from "./types";

type IxMethodName = Exclude<keyof PhoenixIxClient, "orderPackets">;

const IX_METHOD_NAMES = [
  "buildCancelAll",
  "buildCancelOrdersById",
  "buildCancelStopLoss",
  "buildCreateConditionalOrdersAccount",
  "buildCreateEscrowRequest",
  "buildDelegateTrader",
  "buildOnboardTraderDelegated",
  "buildPlaceAttachedConditionalOrder",
  "buildPlaceLimitOrder",
  "buildPlaceLimitOrderWithConditionals",
  "buildPlaceMarketOrder",
  "buildPlacePositionConditionalOrder",
  "buildPlacePostOnlyOrder",
  "placeLimitOrder",
  "placeMarketOrder",
  "placePositionConditionalOrder",
  "placePostOnlyOrder",
  "buildDepositFunds",
  "buildWithdrawFunds",
  "buildTransferCollateral",
  "buildTransferCollateralChildToParent",
  "buildEmberDeposit",
  "buildEmberWithdraw",
  "buildPlaceStopLoss",
  "buildRegisterTrader",
  "buildSyncParentToChild",
  "buildDepositIxs",
  "buildWithdrawIxs",
] as const satisfies readonly IxMethodName[];

const summarizeIxParams = (params: unknown): Record<string, unknown> => {
  if (!params || typeof params !== "object") {
    return {};
  }

  const input = params as Record<string, unknown>;
  const summary: Record<string, unknown> = {};
  for (const key of [
    "symbol",
    "authority",
    "traderAuthority",
    "positionAuthority",
    "permissionAuthority",
    "delegatedKey",
    "traderPdaIndex",
    "traderSubaccountIndex",
    "subaccountIndex",
    "amount",
    "feePayer",
  ]) {
    if (key in input) {
      summary[key] = input[key];
    }
  }

  if ("orderPacket" in input) {
    const packet = input.orderPacket;
    if (packet && typeof packet === "object") {
      const orderPacket = packet as Record<string, unknown>;
      summary.orderPacketKeys = Object.keys(orderPacket).sort();
      if ("side" in orderPacket) {
        summary.side = orderPacket.side;
      }
    }
  }

  return summary;
};

const withIxDebugLogging = (client: PhoenixIxClient): PhoenixIxClient => {
  const wrapped = { ...client } as PhoenixIxClient;
  const wrappedRecord = wrapped as unknown as Record<string, unknown>;

  for (const method of IX_METHOD_NAMES) {
    const original = client[method] as (params: unknown) => Promise<unknown>;
    wrappedRecord[method] = async (params: unknown) => {
      const startedAt = Date.now();
      debugRise("ix", `${String(method)}:start`, summarizeIxParams(params));
      try {
        const result = await original(params);
        debugRise("ix", `${String(method)}:done`, {
          durationMs: Date.now() - startedAt,
          ...summarizeIxParams(params),
        });
        return result;
      } catch (error) {
        debugRise("ix", `${String(method)}:error`, {
          durationMs: Date.now() - startedAt,
          ...summarizeIxParams(params),
          ...summarizeDebugError(error),
        });
        throw error;
      }
    };
  }

  return wrapped;
};

const getInstructionContextOrThrow = async (
  exchange: PhoenixExchangeMetadata,
  symbol: Symbol
): Promise<ExchangeInstructionContext> => {
  debugRise("ix", "exchange.ready:instructionContext", { symbol });
  await exchange.ready();
  const context = exchange.instructionContext(symbol);
  if (context) {
    return context;
  }
  const availableSymbols = exchange
    .snapshot()
    .markets.map((market) => market.symbol)
    .join(", ");
  throw new Error(
    `Unknown market symbol '${symbol}'. Available symbols: ${availableSymbols}`
  );
};

const getExchangeSnapshot = async (exchange: PhoenixExchangeMetadata) => {
  debugRise("ix", "exchange.ready:exchangeSnapshot");
  await exchange.ready();
  return exchange.snapshot().exchange;
};

export const createPhoenixIxClient = (config: {
  exchange: PhoenixExchangeMetadata;
  pda: PhoenixPdaClient;
  orderPackets: PhoenixOrderPacketBuilders;
  rpc?: PhoenixRpcClient;
  flight?: PhoenixFlightClientConfig;
}): PhoenixIxClient => {
  const phoenixProgramAddress = config.pda.getProgramAddress();
  const logAuthorityAddressPromise = config.pda.getLogAuthorityAddress({
    phoenixProgramAddress,
  });
  const emberStatePromise = config.pda.getEmberStateAddress({
    phoenixProgramAddress,
  });
  const emberVaultPromise = config.pda.getEmberVaultAddress({
    phoenixProgramAddress,
  });

  const resolveContext = async (params: {
    authority: Authority;
    positionAuthority?: Authority;
    symbol: Symbol;
    traderPdaIndex?: number;
    traderSubaccountIndex?: number;
  }): Promise<ResolvedPlaceOrderContext> => {
    const [instructionContext, logAuthorityAddress, traderAccount] =
      await Promise.all([
        getInstructionContextOrThrow(config.exchange, params.symbol),
        logAuthorityAddressPromise,
        config.pda.getTraderAddress({
          authority: params.authority,
          traderPdaIndex: params.traderPdaIndex ?? 0,
          subaccountIndex: params.traderSubaccountIndex ?? 0,
          phoenixProgramAddress,
        }),
      ]);

    return buildResolvedPlaceOrderContext({
      instructionContext,
      phoenixProgramAddress,
      logAuthorityAddress,
      authority: params.authority,
      positionAuthority: params.positionAuthority,
      traderAccount,
    });
  };

  const resolveMarketContext = async (
    symbol: Symbol
  ): Promise<PhoenixIxResolvedMarketContext> => {
    const instructionContext = await getInstructionContextOrThrow(
      config.exchange,
      symbol
    );

    return {
      assetId: instructionContext.market.assetId,
      marketAddress: instructionContext.market.marketPubkey as never,
      splineCollection: instructionContext.market.splinePubkey as never,
      tickSize: instructionContext.market.tickSize,
      baseLotsDecimals: instructionContext.market.baseLotsDecimals,
    };
  };

  const resolveExchangeInstructionAccounts = async () => {
    const [exchangeSnapshot, logAuthorityAddress] = await Promise.all([
      getExchangeSnapshot(config.exchange),
      logAuthorityAddressPromise,
    ]);

    return {
      phoenixProgramAddress,
      logAuthorityAddress,
      globalConfigurationAddress:
        exchangeSnapshot.globalConfig as GlobalConfigurationAddress,
      canonicalMint: exchangeSnapshot.canonicalMint as MintAddress,
      usdcMint: exchangeSnapshot.usdcMint as MintAddress,
      perpAssetMap: exchangeSnapshot.perpAssetMap as PerpAssetMapAddress,
      globalVault: exchangeSnapshot.globalVault as GlobalVaultAddress,
      withdrawQueue: exchangeSnapshot.withdrawQueue as WithdrawQueueAddress,
      globalTraderIndex:
        exchangeSnapshot.globalTraderIndex as GlobalTraderIndexAddressArray,
      activeTraderBuffer:
        exchangeSnapshot.activeTraderBuffer as ActiveTraderBufferAddressArray,
    };
  };

  const resolveEmberAccounts = async () => {
    const [emberState, emberVault] = await Promise.all([
      emberStatePromise,
      emberVaultPromise,
    ]);

    return {
      emberState,
      emberVault,
    };
  };

  const resolveTraderAccount = async (params: {
    authority: Authority;
    traderPdaIndex?: number;
    traderSubaccountIndex?: number;
  }): Promise<TraderAddress> =>
    config.pda.getTraderAddress({
      authority: params.authority,
      traderPdaIndex: params.traderPdaIndex ?? 0,
      subaccountIndex: params.traderSubaccountIndex ?? 0,
      phoenixProgramAddress,
    });

  const resolveFlightFeeCollectorTraderAddress = async (
    builderAuthority: Authority,
    traderPdaIndex: number,
    subaccountIndex: number
  ) => {
    if (!config.flight) {
      throw new Error("Flight routing is not configured for this client");
    }

    const rpc = config.rpc;
    if (rpc?.available) {
      try {
        const builderState = await fetchBuilderState({
          client: {
            fetchAccount: (address) => rpc.accounts.fetchAccount(address),
            _cacheEnabled: false,
          },
          authority: builderAuthority,
          phoenixProgramAddress,
          skipCache: true,
        });
        if (builderState.isActive) {
          return builderState.traderKey;
        }
      } catch (error) {
        debugRise("ix", "flight.resolveFeeCollector:builderStateMiss", {
          builderAuthority,
          traderPdaIndex,
          subaccountIndex,
          ...summarizeDebugError(error),
        });
      }
    }

    return config.pda.getTraderAddress({
      authority: builderAuthority,
      traderPdaIndex,
      subaccountIndex,
      phoenixProgramAddress,
    });
  };

  const maybeWrapOrderIx = async <TIx extends InstructionsWithAccountsAndData>(
    instruction: TIx,
    authority: Authority
  ): Promise<TIx> => {
    if (!config.flight) {
      return instruction;
    }

    debugRise("ix", "flight.wrap:start", {
      authority,
    });
    return (await wrapInstructionWithFlight({
      phoenixInstruction: instruction,
      authority,
      phoenixProgramAddress,
      flight: config.flight,
      resolveFeeCollectorTraderAddress: (traderPdaIndex, subaccountIndex) =>
        resolveFlightFeeCollectorTraderAddress(
          config.flight!.builderAuthority,
          traderPdaIndex,
          subaccountIndex
        ),
    })) as TIx;
  };

  const resolveTraderTokenAccounts = async (
    authority: Authority,
    params: {
      canonicalMint: MintAddress;
      usdcMint: MintAddress;
    }
  ) => {
    const [phoenixTokenAccount, usdcTokenAccount] = await Promise.all([
      config.pda.getTraderTokenAccountAddress({
        authority,
        mint: params.canonicalMint,
      }),
      config.pda.getTraderTokenAccountAddress({
        authority,
        mint: params.usdcMint,
      }),
    ]);

    return {
      phoenixTokenAccount,
      usdcTokenAccount,
    };
  };

  const accountExists = async (address: Address): Promise<boolean> => {
    if (!config.rpc?.available) {
      return false;
    }

    try {
      await config.rpc.accounts.fetchAccount(address);
      return true;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (
        message.includes(`Account ${address} was not returned from RPC`) ||
        message.includes("RPC getAccountInfo failed")
      ) {
        return false;
      }
      throw error;
    }
  };

  const operations = createPhoenixIxOperations({
    orderPackets: config.orderPackets,
    phoenixProgramAddress,
    resolvePlaceOrderContext: resolveContext,
    resolveMarketContext,
    resolveExchangeInstructionAccounts,
    resolveTraderAccount,
    resolveTraderTokenAccounts,
    resolveEmberAccounts,
    resolveLogAuthorityAddress: () => logAuthorityAddressPromise,
    resolveGlobalConfigurationAddress: () =>
      config.pda.getGlobalConfigurationAddress({
        phoenixProgramAddress,
      }),
    resolveEscrowAddress: (authority) =>
      config.pda.getEscrowAddress({
        authority,
        phoenixProgramAddress,
      }),
    resolvePermissionAddress: ({ permissionAuthority, delegatedKey }) =>
      config.pda.getPermissionAddress({
        permissionAuthority,
        delegatedKey,
        phoenixProgramAddress,
      }),
    resolveStopLossAddress: ({ traderAccount, assetId }) =>
      config.pda.getStopLossAddress({
        traderAccount,
        assetId,
        phoenixProgramAddress,
      }),
    maybeWrapOrderIx,
    accountExists,
  });

  return withIxDebugLogging(operations);
};
