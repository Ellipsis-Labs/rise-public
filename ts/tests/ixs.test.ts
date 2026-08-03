import {
  Direction,
  OrderFlags,
  SelfTradeBehavior,
  Side,
  StopLossOrderKind,
  TraderPreferenceKind,
  baseLots,
  buildCancelAllIxResolved,
  buildCancelOrdersByIdIxResolved,
  buildCancelStopLossIxResolved,
  buildCancelUpToIxResolved,
  buildUncrossCrankIxResolved,
  buildCreateEscrowRequestIxResolved,
  buildDelegateTraderIxResolved,
  buildDepositFundsIxResolved,
  buildDepositIxsResolved,
  buildEmberDepositIxResolved,
  buildEmberWithdrawIxResolved,
  buildPlaceLimitOrderIxResolved,
  buildPlaceMarketOrderDelegatedIxResolved,
  buildPlaceMarketOrderIxResolved,
  buildPlaceMultiLimitOrderIx,
  buildPlacePostOnlyOrderIxResolved,
  buildPlaceStopLossIxResolved,
  buildRegisterTraderIxResolved,
  buildOnboardTraderDelegatedIxResolved,
  buildSyncParentToChildIxResolved,
  buildTransferCollateralChildToParentIxResolved,
  buildTransferCollateralIxResolved,
  buildWithdrawFundsIxResolved,
  buildWithdrawIxsResolved,
  quoteLots,
  ticks,
} from "@/index";
import {
  createPhoenixIxOperations,
  type PhoenixIxOperationContext,
} from "@/ixs/operations";
import type { ResolvedPlaceOrderContext } from "@/ixs/types";
import { getCancelOrdersByIdDecoder } from "@/core/ixBuilders/CancelOrdersById";
import type { Authority } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import { AccountRole } from "@solana/kit";
import { describe, expect, it } from "vitest";
import { DISCRIMINANTS } from "@/core/discriminants";

const resolvedOrderContext = {
  exchange: {
    phoenixProgramAddress: "phoenix-program" as never,
    logAuthorityAddress: "log-authority" as never,
    globalConfigurationAddress: "global-config" as never,
    perpAssetMap: "perp-asset-map" as never,
    globalTraderIndex: ["gti-0"] as never,
    activeTraderBuffer: ["atb-0"] as never,
  },
  market: {
    marketAddress: "market-address" as never,
    splineCollection: "spline-address" as never,
  },
  trader: {
    authority: "trader-authority" as never,
    positionAuthority: "position-authority" as never,
    traderAccount: "trader-account" as never,
  },
} as const;

const resolvedDepositExchange = {
  phoenixProgramAddress: "phoenix-program" as never,
  logAuthorityAddress: "log-authority" as never,
  globalConfigurationAddress: "global-config" as never,
  canonicalMint: "phoenix-mint" as never,
  usdcMint: "usdc-mint" as never,
  perpAssetMap: "perp-asset-map" as never,
  globalVault: "global-vault" as never,
  withdrawQueue: "withdraw-queue" as never,
  globalTraderIndex: ["gti-0"] as never,
  activeTraderBuffer: ["atb-0"] as never,
  emberState: "ember-state" as never,
  emberVault: "ember-vault" as never,
} as const;

const resolvedDepositTrader = {
  authority: "trader-authority" as never,
  traderAccount: "trader-account" as never,
  phoenixTokenAccount: "phoenix-token-account" as never,
  usdcTokenAccount: "usdc-token-account" as never,
} as const;

const marketOrderPacket = {
  side: Side.Ask,
  priceInTicks: ticks(99n),
  numBaseLots: baseLots(3n),
  numQuoteLots: null,
  minBaseLotsToFill: baseLots(3n),
  minQuoteLotsToFill: quoteLots(1n),
  selfTradeBehavior: SelfTradeBehavior.Abort,
  matchLimit: null,
  clientOrderId: 0n,
  lastValidSlot: null,
  orderFlags: OrderFlags.ReduceOnly,
  cancelExisting: false,
} as const;

const unexpectedOperationResolver = async (): Promise<never> => {
  throw new Error("unexpected operation resolver call");
};

const createTestOperationContext = (
  wrapCalls: Array<{
    instruction: InstructionsWithAccountsAndData;
    signer: Authority;
    usePositionAuthority: boolean | undefined;
  }>
): PhoenixIxOperationContext => ({
  orderPackets: {} as never,
  phoenixProgramAddress: "phoenix-program" as never,
  resolvePlaceOrderContext: async () =>
    resolvedOrderContext as ResolvedPlaceOrderContext,
  resolveMarketContext: unexpectedOperationResolver,
  resolveExchangeInstructionAccounts: unexpectedOperationResolver,
  resolveTraderAccount: unexpectedOperationResolver,
  resolveTraderTokenAccounts: unexpectedOperationResolver,
  resolveEmberAccounts: unexpectedOperationResolver,
  resolveLogAuthorityAddress: unexpectedOperationResolver,
  resolveGlobalConfigurationAddress: unexpectedOperationResolver,
  resolveEscrowAddress: unexpectedOperationResolver,
  resolvePermissionAddress: unexpectedOperationResolver,
  resolveStopLossAddress: unexpectedOperationResolver,
  maybeWrapOrderIx: async <TIx extends InstructionsWithAccountsAndData>(
    instruction: TIx,
    signer: Authority,
    usePositionAuthority?: boolean
  ) => {
    wrapCalls.push({ instruction, signer, usePositionAuthority });
    return instruction;
  },
  maybeWrapConditionalOrderIx: async <
    TIx extends InstructionsWithAccountsAndData,
  >(
    instruction: TIx,
    authority: Authority
  ) => {
    wrapCalls.push({
      instruction,
      signer: authority,
      usePositionAuthority: false,
    });
    return instruction;
  },
  accountExists: async () => false,
});

type WrapCall = {
  instruction: InstructionsWithAccountsAndData;
  signer: Authority;
  usePositionAuthority: boolean | undefined;
};

describe("ix operations", () => {
  const createCancelByIdOperations = (
    market: {
      tickSize?: number;
      baseLotsDecimals?: number;
    } = {
      tickSize: 100,
      baseLotsDecimals: 3,
    }
  ) =>
    createPhoenixIxOperations({
      ...createTestOperationContext([]),
      resolveExchangeInstructionAccounts: async () => resolvedDepositExchange,
      resolveMarketContext: async () => ({
        assetId: 0,
        marketAddress: "market-address" as never,
        splineCollection: "spline-address" as never,
        ...market,
      }),
      resolveTraderAccount: async () => "trader-account" as never,
    });

  it("wraps market order entrypoints with the effective signer", async () => {
    const wrapCalls: WrapCall[] = [];
    const operations = createPhoenixIxOperations(
      createTestOperationContext(wrapCalls)
    );

    const marketIx = await operations.placeMarketOrder({
      authority: "trader-authority" as never,
      symbol: "BTC-PERP" as never,
      orderPacket: marketOrderPacket,
    });
    const delegatedIx = await operations.placeMarketOrderDelegated({
      authority: "trader-authority" as never,
      symbol: "BTC-PERP" as never,
      traderWallet: "delegated-wallet" as never,
      permissionAccount: "permission-account" as never,
      orderPacket: marketOrderPacket,
    });

    expect(wrapCalls).toHaveLength(2);
    expect(wrapCalls[0]?.instruction).toBe(marketIx);
    expect(wrapCalls[0]?.signer).toBe("trader-authority");
    expect(wrapCalls[0]?.usePositionAuthority).toBe(false);
    expect(Array.from(marketIx.data.slice(0, 8))).toEqual(
      Array.from(DISCRIMINANTS.PLACE_MARKET_ORDER)
    );
    // The delegated order names a distinct signer, so the wrap is told the
    // signer is a position authority.
    expect(wrapCalls[1]?.instruction).toBe(delegatedIx);
    expect(wrapCalls[1]?.signer).toBe("delegated-wallet");
    expect(wrapCalls[1]?.usePositionAuthority).toBe(true);
    expect(Array.from(delegatedIx.data.slice(0, 8))).toEqual(
      Array.from(DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED)
    );
    expect(delegatedIx.accounts[3]?.address).toBe("delegated-wallet");
    expect(delegatedIx.accounts[4]?.address).toBe("permission-account");
  });

  it("declares position authority only when the signer differs from the owner", async () => {
    const wrapCalls: WrapCall[] = [];
    const operations = createPhoenixIxOperations(
      createTestOperationContext(wrapCalls)
    );

    await operations.placeMarketOrder({
      authority: "trader-authority" as never,
      positionAuthority: "trader-authority" as never,
      symbol: "BTC-PERP" as never,
      orderPacket: marketOrderPacket,
    });
    await operations.placeMarketOrder({
      authority: "trader-authority" as never,
      positionAuthority: "delegate-signer" as never,
      symbol: "BTC-PERP" as never,
      orderPacket: marketOrderPacket,
    });

    expect(wrapCalls).toHaveLength(2);
    expect(wrapCalls[0]?.signer).toBe("trader-authority");
    expect(wrapCalls[0]?.usePositionAuthority).toBe(false);
    expect(wrapCalls[1]?.signer).toBe("delegate-signer");
    expect(wrapCalls[1]?.usePositionAuthority).toBe(true);
  });

  it("builds delegated primary-position-authority market orders through the shared wrapper", async () => {
    const wrapCalls: WrapCall[] = [];
    const operations = createPhoenixIxOperations(
      createTestOperationContext(wrapCalls)
    );

    const ix = await operations.placeMarketOrderDelegated({
      authority: "trader-authority" as never,
      symbol: "BTC-PERP" as never,
      orderPacket: marketOrderPacket,
    });

    expect(wrapCalls).toHaveLength(1);
    expect(wrapCalls[0]?.instruction).toBe(ix);
    expect(wrapCalls[0]?.signer).toBe("trader-authority");
    expect(wrapCalls[0]?.usePositionAuthority).toBe(false);
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("position-authority");
  });

  it("builds cancel-by-id from tick prices without converting them as USD", async () => {
    const ix = await createCancelByIdOperations().buildCancelOrdersById({
      authority: "trader-authority" as never,
      symbol: "BTC-PERP" as never,
      orders: [
        {
          priceInTicks: ticks(123_456n),
          orderSequenceNumber: 7n,
        },
        {
          priceInTicks: 234_567,
          orderSequenceNumber: "8",
        },
        {
          priceInTicks: "345678",
          orderSequenceNumber: 9,
        },
        {
          price: 86_000,
          orderSequenceNumber: 10,
        },
      ],
    });

    const decoded = getCancelOrdersByIdDecoder().decode(ix.data);
    expect(decoded.orderIds[0]?.orderId.priceInTicks).toBe(ticks(123_456n));
    expect(decoded.orderIds[0]?.orderId.orderSequenceNumber).toBe(7n);
    expect(decoded.orderIds[1]?.orderId.priceInTicks).toBe(ticks(234_567n));
    expect(decoded.orderIds[1]?.orderId.orderSequenceNumber).toBe(8n);
    expect(decoded.orderIds[2]?.orderId.priceInTicks).toBe(ticks(345_678n));
    expect(decoded.orderIds[2]?.orderId.orderSequenceNumber).toBe(9n);
    expect(decoded.orderIds[3]?.orderId.priceInTicks).toBe(ticks(860_000n));
    expect(decoded.orderIds[3]?.orderId.orderSequenceNumber).toBe(10n);
  });

  it("builds tick-native cancel-by-id without tick-size metadata", async () => {
    const ix = await createCancelByIdOperations({}).buildCancelOrdersById({
      authority: "trader-authority" as never,
      symbol: "BTC-PERP" as never,
      orders: [
        {
          priceInTicks: ticks(123_456n),
          orderSequenceNumber: 7n,
        },
      ],
    });

    const decoded = getCancelOrdersByIdDecoder().decode(ix.data);
    expect(decoded.orderIds).toHaveLength(1);
    expect(decoded.orderIds[0]?.orderId.priceInTicks).toBe(ticks(123_456n));
  });

  it("requires tick-size metadata for legacy cancel-by-id USD prices", async () => {
    await expect(
      createCancelByIdOperations({ baseLotsDecimals: 3 }).buildCancelOrdersById(
        {
          authority: "trader-authority" as never,
          symbol: "BTC-PERP" as never,
          orders: [{ price: 86_000, orderSequenceNumber: 7 }],
        }
      )
    ).rejects.toThrow("Market metadata is missing tick size");

    await expect(
      createCancelByIdOperations({ tickSize: 100 }).buildCancelOrdersById({
        authority: "trader-authority" as never,
        symbol: "BTC-PERP" as never,
        orders: [{ price: 86_000, orderSequenceNumber: 7 }],
      })
    ).rejects.toThrow("Market metadata is missing base lot decimals");
  });

  it("rejects cancel-by-id orders without a price field", async () => {
    await expect(
      createCancelByIdOperations().buildCancelOrdersById({
        authority: "trader-authority" as never,
        symbol: "BTC-PERP" as never,
        orders: [{ orderSequenceNumber: 7 }] as never,
      })
    ).rejects.toThrow("Cancel order requires priceInTicks or price");
  });
});

describe("resolved ix builders", () => {
  it("builds a resolved place limit order ix synchronously", () => {
    const ix = buildPlaceLimitOrderIxResolved({
      ...resolvedOrderContext,
      orderPacket: {
        side: Side.Bid,
        priceInTicks: ticks(100n),
        numBaseLots: baseLots(5n),
        selfTradeBehavior: SelfTradeBehavior.CancelProvide,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts.at(-2)?.address).toBe("market-address");
    expect(ix.accounts.at(-1)?.address).toBe("spline-address");
  });

  it("builds a resolved place market order ix synchronously", () => {
    const ix = buildPlaceMarketOrderIxResolved({
      ...resolvedOrderContext,
      orderPacket: {
        side: Side.Ask,
        priceInTicks: ticks(99n),
        numBaseLots: baseLots(3n),
        numQuoteLots: null,
        minBaseLotsToFill: baseLots(3n),
        minQuoteLotsToFill: quoteLots(1n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.ReduceOnly,
        cancelExisting: false,
      },
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts.at(-2)?.address).toBe("market-address");
  });

  it("builds a resolved delegated market order ix with explicit signer and permission", () => {
    const ix = buildPlaceMarketOrderDelegatedIxResolved({
      ...resolvedOrderContext,
      traderWallet: "delegated-wallet" as never,
      permissionAccount: "permission-account" as never,
      orderPacket: {
        side: Side.Ask,
        priceInTicks: ticks(99n),
        numBaseLots: baseLots(3n),
        numQuoteLots: null,
        minBaseLotsToFill: baseLots(3n),
        minQuoteLotsToFill: quoteLots(1n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.ReduceOnly,
        cancelExisting: false,
      },
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(Array.from(ix.data.slice(0, 8))).toEqual(
      Array.from(DISCRIMINANTS.PLACE_MARKET_ORDER_DELEGATED)
    );
    expect(ix.accounts[3]?.address).toBe("delegated-wallet");
    expect(ix.accounts[3]?.role).toBe(AccountRole.READONLY_SIGNER);
    expect(ix.accounts[4]?.address).toBe("permission-account");
    expect(ix.accounts[5]?.address).toBe("trader-account");
    expect(ix.accounts.at(-2)?.address).toBe("market-address");
  });

  it("uses the position authority as the delegated signer and permission account by default", () => {
    const ix = buildPlaceMarketOrderDelegatedIxResolved({
      ...resolvedOrderContext,
      orderPacket: {
        side: Side.Bid,
        priceInTicks: null,
        numBaseLots: baseLots(2n),
        numQuoteLots: null,
        minBaseLotsToFill: baseLots(1n),
        minQuoteLotsToFill: quoteLots(1n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
    });

    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("position-authority");
  });

  it("builds a resolved post-only order ix synchronously", () => {
    const ix = buildPlacePostOnlyOrderIxResolved({
      ...resolvedOrderContext,
      orderPacket: {
        side: Side.Bid,
        priceInTicks: ticks(101n),
        numBaseLots: baseLots(7n),
        clientOrderId: 0n,
        slide: false,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts.at(-1)?.address).toBe("spline-address");
  });

  it("builds a resolved cancel-all ix synchronously", () => {
    const ix = buildCancelAllIxResolved(resolvedOrderContext);

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts.at(-2)?.address).toBe("market-address");
    expect(ix.accounts.at(-1)?.address).toBe("spline-address");
  });

  it("builds a resolved cancel-orders-by-id ix synchronously", () => {
    const ix = buildCancelOrdersByIdIxResolved({
      ...resolvedOrderContext,
      orderIds: [
        {
          nodePointer: null,
          orderId: {
            priceInTicks: ticks(100n),
            orderSequenceNumber: 7n,
          },
        },
      ],
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts.at(-2)?.address).toBe("market-address");
    expect(ix.accounts.at(-1)?.address).toBe("spline-address");
  });

  it("builds a resolved cancel-up-to ix synchronously", () => {
    const ix = buildCancelUpToIxResolved({
      ...resolvedOrderContext,
      side: Side.Ask,
      numOrdersToCancel: 2,
      tickLimit: ticks(100n),
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(Array.from(ix.data.slice(0, 8))).toEqual(
      Array.from(DISCRIMINANTS.CANCEL_UP_TO)
    );
    expect(ix.accounts[2]?.address).toBe("global-config");
    expect(ix.accounts[2]?.role).toBe(AccountRole.WRITABLE);
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts.at(-2)?.address).toBe("market-address");
    expect(ix.accounts.at(-1)?.address).toBe("spline-address");
  });

  it("builds a resolved uncross-crank ix synchronously", () => {
    const ix = buildUncrossCrankIxResolved({
      exchange: resolvedOrderContext.exchange,
      market: resolvedOrderContext.market,
      matchLimit: 5n,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(Array.from(ix.data.slice(0, 8))).toEqual(
      Array.from(DISCRIMINANTS.UNCROSS_CRANK)
    );
    expect(ix.accounts[3]?.address).toBe("perp-asset-map");
    expect(ix.accounts[4]?.address).toBe("gti-0");
    expect(ix.accounts[5]?.address).toBe("atb-0");
    expect(ix.accounts.at(-2)?.address).toBe("market-address");
    expect(ix.accounts.at(-1)?.address).toBe("spline-address");
  });

  it("builds a resolved cancel-stop-loss ix synchronously", () => {
    const ix = buildCancelStopLossIxResolved({
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
      },
      trader: {
        authority: "trader-authority" as never,
        traderAccount: "trader-account" as never,
        stopLossAccount: "stop-loss-account" as never,
      },
      executionDirection: Direction.GreaterThan,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("trader-authority");
    expect(ix.accounts[3]?.role).toBe(AccountRole.WRITABLE);
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts[5]?.role).toBe(AccountRole.READONLY_SIGNER);
    expect(ix.accounts[6]?.address).toBe("stop-loss-account");
  });

  it("builds a resolved cancel-stop-loss ix with a separate funder", () => {
    const ix = buildCancelStopLossIxResolved({
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
      },
      trader: {
        authority: "trader-authority" as never,
        funder: "stop-loss-funder" as never,
        traderAccount: "trader-account" as never,
        stopLossAccount: "stop-loss-account" as never,
      },
      executionDirection: Direction.GreaterThan,
    });

    expect(ix.accounts[3]?.address).toBe("stop-loss-funder");
    expect(ix.accounts[5]?.address).toBe("trader-authority");
  });

  it("builds a resolved create-escrow-request ix synchronously", () => {
    const ix = buildCreateEscrowRequestIxResolved({
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
        perpAssetMap: "perp-asset-map" as never,
        globalTraderIndex: ["gti-0"] as never,
        activeTraderBuffer: ["atb-0"] as never,
      },
      sender: {
        authority: "sender-authority" as never,
        traderAccount: "sender-trader-account" as never,
        traderPdaIndex: 0,
        traderSubaccountIndex: 0,
      },
      receiver: {
        authority: "receiver-authority" as never,
        traderAccount: "receiver-trader-account" as never,
        escrowAddress: "receiver-escrow" as never,
        permissionAddress: "permission-account" as never,
        traderPdaIndex: 0,
        traderSubaccountIndex: 1,
      },
      actions: [{ kind: "cash", amount: 100n }],
      lastValidSlot: 10n,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("sender-authority");
    expect(ix.accounts[4]?.address).toBe("sender-trader-account");
    expect(ix.accounts[5]?.address).toBe("permission-account");
    expect(ix.accounts[8]?.address).toBe("receiver-escrow");
  });

  it("builds a resolved delegate-trader ix synchronously", () => {
    const ix = buildDelegateTraderIxResolved({
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
      },
      trader: {
        authority: "trader-authority" as never,
        traderAccount: "trader-account" as never,
      },
      newPositionAuthority: "new-position-authority" as never,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("trader-authority");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts[5]?.address).toBe("new-position-authority");
  });

  it("builds a resolved deposit funds ix synchronously", () => {
    const ix = buildDepositFundsIxResolved({
      exchange: {
        phoenixProgramAddress: resolvedDepositExchange.phoenixProgramAddress,
        logAuthorityAddress: resolvedDepositExchange.logAuthorityAddress,
        globalConfigurationAddress:
          resolvedDepositExchange.globalConfigurationAddress,
        canonicalMint: resolvedDepositExchange.canonicalMint,
        globalVault: resolvedDepositExchange.globalVault,
        globalTraderIndex: resolvedDepositExchange.globalTraderIndex,
        activeTraderBuffer: resolvedDepositExchange.activeTraderBuffer,
      },
      trader: {
        authority: resolvedDepositTrader.authority,
        traderAccount: resolvedDepositTrader.traderAccount,
        traderTokenAccount: resolvedDepositTrader.phoenixTokenAccount,
      },
      amount: 1000n,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("trader-authority");
    expect(ix.accounts[4]?.address).toBe("phoenix-token-account");
    expect(ix.accounts[5]?.address).toBe("trader-account");
    expect(ix.accounts[6]?.address).toBe("global-vault");
  });

  it("builds a resolved withdraw funds ix synchronously", () => {
    const ix = buildWithdrawFundsIxResolved({
      exchange: {
        phoenixProgramAddress: resolvedDepositExchange.phoenixProgramAddress,
        logAuthorityAddress: resolvedDepositExchange.logAuthorityAddress,
        globalConfigurationAddress:
          resolvedDepositExchange.globalConfigurationAddress,
        canonicalMint: resolvedDepositExchange.canonicalMint,
        perpAssetMap: resolvedDepositExchange.perpAssetMap,
        globalVault: resolvedDepositExchange.globalVault,
        withdrawQueue: resolvedDepositExchange.withdrawQueue,
        globalTraderIndex: resolvedDepositExchange.globalTraderIndex,
        activeTraderBuffer: resolvedDepositExchange.activeTraderBuffer,
      },
      trader: {
        authority: resolvedDepositTrader.authority,
        traderAccount: resolvedDepositTrader.traderAccount,
        destinationTokenAccount: resolvedDepositTrader.phoenixTokenAccount,
      },
      amount: 1000n,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("trader-authority");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts[7]?.address).toBe("phoenix-token-account");
    expect(ix.accounts.at(-1)?.address).toBe("withdraw-queue");
  });

  it("builds a resolved transfer collateral ix synchronously", () => {
    const ix = buildTransferCollateralIxResolved({
      exchange: {
        phoenixProgramAddress: resolvedDepositExchange.phoenixProgramAddress,
        logAuthorityAddress: resolvedDepositExchange.logAuthorityAddress,
        globalConfigurationAddress:
          resolvedDepositExchange.globalConfigurationAddress,
        perpAssetMap: resolvedDepositExchange.perpAssetMap,
        globalTraderIndex: resolvedDepositExchange.globalTraderIndex,
        activeTraderBuffer: resolvedDepositExchange.activeTraderBuffer,
      },
      trader: {
        authority: resolvedDepositTrader.authority,
        positionAuthority: "position-authority" as never,
        permissionAddress: "permission-account" as never,
        srcTraderAccount: "src-trader-account" as never,
        dstTraderAccount: "dst-trader-account" as never,
      },
      amount: 1000n,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("src-trader-account");
    expect(ix.accounts[5]?.address).toBe("dst-trader-account");
    expect(ix.accounts.at(-1)?.address).toBe("permission-account");
    expect(ix.accounts.at(-1)?.role).toBe(AccountRole.WRITABLE);
  });

  it("builds a resolved transfer collateral child-to-parent ix synchronously", () => {
    const ix = buildTransferCollateralChildToParentIxResolved({
      exchange: {
        phoenixProgramAddress: resolvedDepositExchange.phoenixProgramAddress,
        logAuthorityAddress: resolvedDepositExchange.logAuthorityAddress,
        globalConfigurationAddress:
          resolvedDepositExchange.globalConfigurationAddress,
        perpAssetMap: resolvedDepositExchange.perpAssetMap,
        globalTraderIndex: resolvedDepositExchange.globalTraderIndex,
        activeTraderBuffer: resolvedDepositExchange.activeTraderBuffer,
      },
      trader: {
        authority: resolvedDepositTrader.authority,
        positionAuthority: "position-authority" as never,
        childTraderAccount: "child-trader-account" as never,
        parentTraderAccount: "parent-trader-account" as never,
      },
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("child-trader-account");
    expect(ix.accounts[5]?.address).toBe("parent-trader-account");
  });

  it("builds a resolved ember deposit ix synchronously", () => {
    const ix = buildEmberDepositIxResolved({
      exchange: {
        usdcMint: resolvedDepositExchange.usdcMint,
        canonicalMint: resolvedDepositExchange.canonicalMint,
        emberState: resolvedDepositExchange.emberState,
        emberVault: resolvedDepositExchange.emberVault,
      },
      trader: {
        authority: resolvedDepositTrader.authority,
        usdcTokenAccount: resolvedDepositTrader.usdcTokenAccount,
        phoenixTokenAccount: resolvedDepositTrader.phoenixTokenAccount,
      },
      amount: 1000n,
    });

    expect(ix.accounts[2]?.address).toBe("usdc-mint");
    expect(ix.accounts[3]?.address).toBe("phoenix-mint");
    expect(ix.accounts[4]?.address).toBe("usdc-token-account");
    expect(ix.accounts[5]?.address).toBe("phoenix-token-account");
  });

  it("builds a resolved ember withdraw ix synchronously", () => {
    const ix = buildEmberWithdrawIxResolved({
      exchange: {
        usdcMint: resolvedDepositExchange.usdcMint,
        canonicalMint: resolvedDepositExchange.canonicalMint,
        emberState: resolvedDepositExchange.emberState,
        emberVault: resolvedDepositExchange.emberVault,
      },
      trader: {
        authority: resolvedDepositTrader.authority,
        usdcTokenAccount: resolvedDepositTrader.usdcTokenAccount,
        phoenixTokenAccount: resolvedDepositTrader.phoenixTokenAccount,
      },
      amount: 1000n,
    });

    expect(ix.accounts[2]?.address).toBe("usdc-mint");
    expect(ix.accounts[3]?.address).toBe("phoenix-mint");
    expect(ix.accounts[4]?.address).toBe("usdc-token-account");
    expect(ix.accounts[5]?.address).toBe("phoenix-token-account");
  });

  it("builds a resolved stop-loss ix synchronously", () => {
    const ix = buildPlaceStopLossIxResolved({
      ...resolvedOrderContext,
      stopLossAccount: "stop-loss-account" as never,
      triggerPrice: 1000n,
      executionPrice: 1001n,
      tradeSide: Side.Bid,
      executionDirection: Direction.GreaterThan,
      orderKind: StopLossOrderKind.IOC,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts.at(-3)?.address).toBe("position-authority");
    expect(ix.accounts.at(-2)?.address).toBe("stop-loss-account");
  });

  it("builds a resolved stop-loss ix with a separate funder", () => {
    const ix = buildPlaceStopLossIxResolved({
      ...resolvedOrderContext,
      trader: {
        ...resolvedOrderContext.trader,
        funder: "stop-loss-funder" as never,
      },
      stopLossAccount: "stop-loss-account" as never,
      triggerPrice: 1000n,
      executionPrice: 1001n,
      tradeSide: Side.Bid,
      executionDirection: Direction.GreaterThan,
      orderKind: StopLossOrderKind.IOC,
    });

    expect(ix.accounts[3]?.address).toBe("stop-loss-funder");
    expect(ix.accounts.at(-3)?.address).toBe("position-authority");
  });

  it("builds a resolved register-trader ix synchronously", () => {
    const ix = buildRegisterTraderIxResolved({
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
      },
      trader: {
        payer: "fee-payer" as never,
        authority: "trader-authority" as never,
        traderAccount: "trader-account" as never,
      },
      maxPositions: 1n,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("fee-payer");
    expect(ix.accounts[4]?.address).toBe("trader-authority");
    expect(ix.accounts[5]?.address).toBe("trader-account");
  });

  it("sets disable-collateral-sweep on register-trader preference bits", () => {
    const ix = buildRegisterTraderIxResolved({
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
      },
      trader: {
        payer: "fee-payer" as never,
        authority: "trader-authority" as never,
        traderAccount: "trader-account" as never,
      },
      maxPositions: 1n,
      traderPreferenceBits: 2,
      traderPreferences: [TraderPreferenceKind.DisableCollateralSweep],
      traderPdaIndex: 4,
      traderSubaccountIndex: 5,
    });

    expect(Array.from(ix.data.slice(8))).toEqual([
      1, 0, 0, 0, 3, 0, 0, 0, 4, 5,
    ]);
  });

  it("rejects non-u8 register-trader indexes before encoding", () => {
    const baseParams = {
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
      },
      trader: {
        payer: "fee-payer" as never,
        authority: "trader-authority" as never,
        traderAccount: "trader-account" as never,
      },
      maxPositions: 1n,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
    } as const;

    expect(() =>
      buildRegisterTraderIxResolved({
        ...baseParams,
        traderPdaIndex: 1.5,
      })
    ).toThrow("Trader PDA index must be an integer between 0 and 255");
    expect(() =>
      buildRegisterTraderIxResolved({
        ...baseParams,
        traderSubaccountIndex: 101,
      })
    ).toThrow("Trader subaccount index must be an integer between 0 and 100");
  });

  it("builds a resolved delegated trader-onboarding ix synchronously", () => {
    const ix = buildOnboardTraderDelegatedIxResolved({
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
        globalTraderIndex: ["gti-0", "gti-1"] as never,
        activeTraderBuffer: ["atb-0", "atb-1"] as never,
      },
      trader: {
        authority: "delegated-authority" as never,
        permissionAccount: "permission-account" as never,
        traderAccount: "trader-account" as never,
      },
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("delegated-authority");
    expect(ix.accounts[3]?.role).toBe(AccountRole.READONLY_SIGNER);
    expect(ix.accounts[4]?.address).toBe("permission-account");
    expect(ix.accounts[4]?.role).toBe(AccountRole.WRITABLE);
    expect(ix.accounts[5]?.address).toBe("trader-account");
    expect(ix.accounts[6]?.address).toBe("gti-0");
    expect(ix.accounts[8]?.address).toBe("atb-0");
    expect(Array.from(ix.data.slice(8))).toEqual([
      6, 0, 0, 0, 0, 1, 1, 1, 3, 1, 2, 1, 4, 1, 5, 1,
    ]);
  });

  it("builds a resolved sync-parent-to-child ix synchronously", () => {
    const ix = buildSyncParentToChildIxResolved({
      exchange: {
        phoenixProgramAddress: "phoenix-program" as never,
        logAuthorityAddress: "log-authority" as never,
        globalConfigurationAddress: "global-config" as never,
        globalTraderIndex: ["gti-0"] as never,
      },
      trader: {
        authority: "trader-authority" as never,
        parentTraderAccount: "parent-trader-account" as never,
        childTraderAccount: "child-trader-account" as never,
      },
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("trader-authority");
    expect(ix.accounts[4]?.address).toBe("parent-trader-account");
    expect(ix.accounts[5]?.address).toBe("child-trader-account");
  });

  it("builds a resolved deposit ix bundle synchronously with an overridden fee payer", () => {
    const result = buildDepositIxsResolved({
      feePayer: "fee-payer" as never,
      exchange: resolvedDepositExchange,
      trader: resolvedDepositTrader,
      amount: 1000n,
    });

    expect(result.instructions).toHaveLength(3);
    expect(result.named.createPhoenixAta.accounts[0]?.address).toBe(
      "fee-payer"
    );
    expect(result.named.createPhoenixAta.accounts[1]?.address).toBe(
      "phoenix-token-account"
    );
    expect(result.named.emberDeposit.accounts[4]?.address).toBe(
      "usdc-token-account"
    );
    expect(result.named.depositFunds.accounts[4]?.address).toBe(
      "phoenix-token-account"
    );
  });

  it("builds a resolved withdraw ix bundle synchronously with an overridden fee payer", () => {
    const result = buildWithdrawIxsResolved({
      feePayer: "fee-payer" as never,
      exchange: resolvedDepositExchange,
      trader: resolvedDepositTrader,
      amount: 1000n,
    });

    expect(result.instructions).toHaveLength(5);
    expect(result.named.createPhoenixAta.accounts[0]?.address).toBe(
      "fee-payer"
    );
    expect(result.named.approveToken.accounts[0]?.address).toBe(
      "phoenix-token-account"
    );
    expect(result.named.approveToken.accounts[1]?.address).toBe("ember-state");
    expect(result.named.createUsdcAta.accounts[0]?.address).toBe("fee-payer");
    expect(result.named.createUsdcAta.accounts[1]?.address).toBe(
      "usdc-token-account"
    );
    expect(result.named.withdrawFunds.accounts[7]?.address).toBe(
      "phoenix-token-account"
    );
    expect(result.named.emberWithdraw.accounts[2]?.address).toBe("usdc-mint");
    expect(result.named.emberWithdraw.accounts[3]?.address).toBe(
      "phoenix-mint"
    );
  });
});

describe("buildPlaceMultiLimitOrderIx", () => {
  const buildIx = () =>
    buildPlaceMultiLimitOrderIx({
      programAddress: "phoenix-program" as never,
      logAuthorityAddress: "log-authority" as never,
      globalConfigurationAddress: "global-config" as never,
      trader: "trader" as never,
      traderAccount: "trader-account" as never,
      perpAssetMap: "perp-asset-map" as never,
      orderbook: "orderbook" as never,
      splineCollection: "spline-collection" as never,
      globalTraderIndex: ["gti-0"] as never,
      activeTraderBuffer: ["atb-0"] as never,
      multipleOrderPacket: {
        bids: [],
        asks: [{ priceInTicks: 1n, sizeInBaseLots: 1n, lastValidSlot: null }],
        clientOrderId: null,
        slide: false,
      },
    });

  // The on-chain program loads `LimitOrderInstructionGroup`, which binds the
  // global configuration as `GlobalConfigurationAccount<Writable>`. A readonly
  // flag here makes the loader reject the tx with InvalidAccountData (the same
  // account layout single `place_limit_order` uses).
  it("marks the global configuration account writable", () => {
    const ix = buildIx();
    expect(ix.accounts[2]?.address).toBe("global-config");
    expect(ix.accounts[2]?.role).toBe(AccountRole.WRITABLE);
  });

  it("keeps the trader at index 3 as a readonly signer", () => {
    const ix = buildIx();
    expect(ix.accounts[3]?.address).toBe("trader");
    expect(ix.accounts[3]?.role).toBe(AccountRole.READONLY_SIGNER);
  });
});
