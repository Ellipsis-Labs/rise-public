import {
  Direction,
  OrderFlags,
  SelfTradeBehavior,
  Side,
  StopLossOrderKind,
  baseLots,
  buildCancelAllIxResolved,
  buildCancelOrdersByIdIxResolved,
  buildCancelStopLossIxResolved,
  buildCreateEscrowRequestIxResolved,
  buildDelegateTraderIxResolved,
  buildDepositFundsIxResolved,
  buildDepositIxsResolved,
  buildEmberDepositIxResolved,
  buildEmberWithdrawIxResolved,
  buildPlaceLimitOrderIxResolved,
  buildPlaceMarketOrderIxResolved,
  buildPlacePostOnlyOrderIxResolved,
  buildPlaceStopLossIxResolved,
  buildRegisterTraderIxResolved,
  buildSyncParentToChildIxResolved,
  buildTransferCollateralChildToParentIxResolved,
  buildTransferCollateralIxResolved,
  buildWithdrawFundsIxResolved,
  buildWithdrawIxsResolved,
  quoteLots,
  ticks,
} from "@/index";
import { describe, expect, it } from "vitest";

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
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts[6]?.address).toBe("stop-loss-account");
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
        srcTraderAccount: "src-trader-account" as never,
        dstTraderAccount: "dst-trader-account" as never,
      },
      amount: 1000n,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("src-trader-account");
    expect(ix.accounts[5]?.address).toBe("dst-trader-account");
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

    expect(ix.accounts[2]?.address).toBe("phoenix-mint");
    expect(ix.accounts[3]?.address).toBe("usdc-mint");
    expect(ix.accounts[4]?.address).toBe("phoenix-token-account");
    expect(ix.accounts[5]?.address).toBe("usdc-token-account");
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
    expect(result.named.emberWithdraw.accounts[2]?.address).toBe(
      "phoenix-mint"
    );
    expect(result.named.emberWithdraw.accounts[3]?.address).toBe("usdc-mint");
  });
});
