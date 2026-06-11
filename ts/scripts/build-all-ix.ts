#!/usr/bin/env bun
/// Build all shared instruction builders and output JSON with hex-encoded data.
/// This script tests parity between Rust and TS instruction builders.

import {
  buildPlaceLimitOrderIx,
  buildPlaceMarketOrderDelegatedIx,
  buildPlaceMarketOrderIx,
  buildPlaceMultiLimitOrderIx,
  buildCancelOrdersByIdIx,
  buildCancelStopLossIx,
  buildPlaceStopLossIx,
  buildCancelConditionalOrderIx,
  buildCreateConditionalOrdersAccountIx,
  buildPlaceAttachedConditionalOrderIx,
  buildPlaceLimitOrderWithConditionalsIx,
  buildPlacePositionConditionalOrderIx,
  buildDepositFundsIx,
  buildWithdrawFundsIx,
  buildRegisterTraderIx,
  buildEmberDepositIx,
  buildEmberWithdrawIx,
  buildTransferCollateralIx,
  buildTransferCollateralChildToParentIx,
  buildSyncParentToChildIx,
  buildOnboardTraderDelegatedIx,
  Side,
  SelfTradeBehavior,
  OrderFlags,
  baseLots,
  ticks,
  quoteLots,
  Direction,
  StopLossOrderKind,
  flight,
  PHOENIX_PROGRAM_ADDRESS,
  PHOENIX_LOG_AUTHORITY_ADDRESS,
  PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
  USDC_MINT_ADDRESS,
} from "@/index";
import { address } from "@solana/kit";

// 20 dummy Solana addresses (matches Rust: bytes [0..31] with bytes[31] = i)
const pubkeys: string[] = [
  "11111111111111111111111111111111", // 0
  "11111111111111111111111111111112", // 1
  "11111111111111111111111111111113", // 2
  "11111111111111111111111111111114", // 3
  "11111111111111111111111111111115", // 4
  "11111111111111111111111111111116", // 5
  "11111111111111111111111111111117", // 6
  "11111111111111111111111111111118", // 7
  "11111111111111111111111111111119", // 8
  "1111111111111111111111111111111A", // 9
  "1111111111111111111111111111111B", // 10
  "1111111111111111111111111111111C", // 11
  "1111111111111111111111111111111D", // 12
  "1111111111111111111111111111111E", // 13
  "1111111111111111111111111111111F", // 14
  "1111111111111111111111111111111G", // 15
  "1111111111111111111111111111111H", // 16
  "1111111111111111111111111111111J", // 17
  "1111111111111111111111111111111K", // 18
  "1111111111111111111111111111111L", // 19
];

const p = (i: number) => address(pubkeys[i]) as any;
const vec2 = (a: number, b: number) => [p(a), p(b)] as any;

console.error("Building all shared instruction builders...");

const results: Record<string, string> = {};

function hexEncode(data: ReadonlyUint8Array): string {
  return Array.from(data)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

try {
  // 1. PlaceLimitOrder
  {
    console.error("Building PlaceLimitOrder...");
    const ix = buildPlaceLimitOrderIx({
      trader: p(0),
      traderAccount: p(1),
      perpAssetMap: p(2),
      orderbook: p(3),
      splineCollection: p(4),
      globalTraderIndex: vec2(5, 6),
      activeTraderBuffer: vec2(7, 8),
      orderPacket: {
        side: Side.Bid,
        priceInTicks: ticks(1000n),
        numBaseLots: baseLots(100n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
    });
    results["PlaceLimitOrder"] = hexEncode(ix.data);
  }

  // 2. PlaceMarketOrder
  {
    console.error("Building PlaceMarketOrder...");
    const ix = buildPlaceMarketOrderIx({
      trader: p(0),
      traderAccount: p(1),
      perpAssetMap: p(2),
      orderbook: p(3),
      splineCollection: p(4),
      globalTraderIndex: vec2(5, 6),
      activeTraderBuffer: vec2(7, 8),
      orderPacket: {
        side: Side.Ask,
        priceInTicks: null,
        numBaseLots: baseLots(100n),
        numQuoteLots: null,
        minBaseLotsToFill: baseLots(0n),
        minQuoteLotsToFill: quoteLots(0n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
    });
    results["PlaceMarketOrder"] = hexEncode(ix.data);
  }

  // 3. PlaceMultiLimitOrder
  {
    console.error("Building PlaceMultiLimitOrder...");
    const ix = buildPlaceMultiLimitOrderIx({
      trader: p(0),
      traderAccount: p(1),
      perpAssetMap: p(2),
      orderbook: p(3),
      splineCollection: p(4),
      globalTraderIndex: vec2(5, 6),
      activeTraderBuffer: vec2(7, 8),
      multipleOrderPacket: {
        bids: [],
        asks: [],
        clientOrderId: null,
        slide: false,
      },
    });
    results["PlaceMultiLimitOrder"] = hexEncode(ix.data);
  }

  // 4. CancelOrdersById
  {
    console.error("Building CancelOrdersById...");
    const ix = buildCancelOrdersByIdIx({
      traderWallet: p(0),
      traderAccount: p(1),
      perpAssetMap: p(2),
      globalTraderIndex: vec2(5, 6),
      activeTraderBuffer: vec2(7, 8),
      orderbook: p(3),
      splineCollection: p(4),
      orderIds: [
        {
          nodePointer: null,
          orderId: { priceInTicks: ticks(1000n), orderSequenceNumber: 0n },
        },
      ],
    });
    results["CancelOrdersById"] = hexEncode(ix.data);
  }

  // 5. CancelStopLoss
  {
    console.error("Building CancelStopLoss...");
    const ix = buildCancelStopLossIx({
      funder: p(0),
      traderWallet: p(1),
      traderAccount: p(1),
      stopLossAccount: p(2),
      executionDirection: Direction.GreaterThan,
    });
    results["CancelStopLoss"] = hexEncode(ix.data);
  }

  // 6. PlaceStopLoss
  {
    const ix = buildPlaceStopLossIx({
      funder: p(0),
      traderAccount: p(1),
      perpAssetMap: p(3),
      orderbook: p(4),
      splineCollection: p(5),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      positionAuthority: p(2),
      stopLossAccount: p(10),
      triggerPrice: ticks(1000n),
      executionPrice: ticks(999n),
      tradeSide: Side.Ask,
      executionDirection: Direction.LessThan,
      orderKind: StopLossOrderKind.IOC,
    });
    results["PlaceStopLoss"] = hexEncode(ix.data);
  }

  // 7. CreateConditionalOrdersAccount
  {
    const ix = buildCreateConditionalOrdersAccountIx({
      payer: p(0),
      traderWallet: p(1),
      traderAccount: p(2),
      traderConditionalOrders: p(3),
      capacity: 32,
    });
    results["CreateConditionalOrdersAccount"] = hexEncode(ix.data);
  }

  // 8. PlacePositionConditionalOrder
  {
    const ix = buildPlacePositionConditionalOrderIx({
      payer: p(0),
      traderAccount: p(1),
      perpAssetMap: p(3),
      orderbook: p(4),
      splineCollection: p(5),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      traderWallet: p(2),
      traderConditionalOrders: p(10),
      assetId: 0,
      greaterTriggerOrder: {
        triggerDirection: Direction.GreaterThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(1000n),
        executionPrice: ticks(999n),
      },
      lessTriggerOrder: null,
      sizeBaseLots: baseLots(100n),
      sizePercent: null,
    });
    results["PlacePositionConditionalOrder"] = hexEncode(ix.data);
  }

  // 9. PlaceAttachedConditionalOrder
  {
    const ix = buildPlaceAttachedConditionalOrderIx({
      traderAccount: p(1),
      traderWallet: p(2),
      orderbook: p(4),
      traderConditionalOrders: p(10),
      payer: p(0),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      orderId: {
        priceInTicks: ticks(1000n),
        orderSequenceNumber: 0n,
      },
      assetId: 0,
      greaterTriggerOrder: null,
      lessTriggerOrder: {
        triggerDirection: Direction.LessThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(900n),
        executionPrice: ticks(899n),
      },
    });
    results["PlaceAttachedConditionalOrder"] = hexEncode(ix.data);
  }

  // 10. PlaceLimitOrderWithConditionals
  {
    const ix = buildPlaceLimitOrderWithConditionalsIx({
      traderWallet: p(2),
      traderAccount: p(1),
      perpAssetMap: p(3),
      orderbook: p(4),
      splineCollection: p(5),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      payer: p(0),
      traderConditionalOrders: p(10),
      orderPacket: {
        __kind: "Limit",
        side: Side.Bid,
        priceInTicks: ticks(1000n),
        numBaseLots: baseLots(100n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
      slot: 0n,
      greaterTriggerOrder: {
        triggerDirection: Direction.GreaterThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(1100n),
        executionPrice: ticks(1099n),
      },
      lessTriggerOrder: {
        triggerDirection: Direction.LessThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(900n),
        executionPrice: ticks(899n),
      },
    });
    results["PlaceLimitOrderWithConditionals"] = hexEncode(ix.data);
  }

  // 11. CancelConditionalOrder
  {
    const ix = buildCancelConditionalOrderIx({
      traderAccount: p(1),
      traderWallet: p(2),
      orderbook: p(4),
      traderConditionalOrders: p(10),
      conditionalOrderIndex: 1,
      disableFirst: true,
      disableSecond: false,
    });
    results["CancelConditionalOrder"] = hexEncode(ix.data);
  }

  // 12. DepositFunds
  {
    const ix = buildDepositFundsIx({
      trader: p(0),
      mint: p(2),
      traderAccount: p(1),
      traderTokenAccount: p(4),
      globalVault: p(3),
      globalTraderIndex: vec2(5, 6),
      activeTraderBuffer: vec2(7, 8),
      amount: 1n,
    });
    results["DepositFunds"] = hexEncode(ix.data);
  }

  // 13. WithdrawFunds
  {
    const ix = buildWithdrawFundsIx({
      trader: p(0),
      traderAccount: p(1),
      mint: p(2),
      perpAssetMap: p(2),
      destinationTokenAccount: p(4),
      globalVault: p(3),
      withdrawQueue: p(5),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      amount: 1n,
    });
    results["WithdrawFunds"] = hexEncode(ix.data);
  }

  // 14. RegisterTrader
  {
    const ix = buildRegisterTraderIx({
      payer: p(0),
      trader: p(1),
      traderAccount: p(2),
      maxPositions: 1n,
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
    });
    results["RegisterTrader"] = hexEncode(ix.data);
  }

  // 15. EmberDeposit
  {
    const ix = buildEmberDepositIx({
      owner: p(0),
      inputMint: p(3),
      outputMint: p(4),
      inputTokenAccount: p(5),
      outputTokenAccount: p(6),
      emberState: p(1),
      emberVault: p(2),
      amount: 1n,
    });
    results["EmberDeposit"] = hexEncode(ix.data);
  }

  // 16. EmberWithdraw
  {
    const ix = buildEmberWithdrawIx({
      owner: p(0),
      inputMint: p(3),
      outputMint: p(4),
      inputTokenAccount: p(5),
      outputTokenAccount: p(6),
      emberState: p(1),
      emberVault: p(2),
      amount: 1n,
    });
    results["EmberWithdraw"] = hexEncode(ix.data);
  }

  // 17. TransferCollateral
  {
    const ix = buildTransferCollateralIx({
      trader: p(0),
      srcTraderAccount: p(1),
      dstTraderAccount: p(2),
      perpAssetMap: p(3),
      globalTraderIndex: vec2(4, 5),
      activeTraderBuffer: vec2(6, 7),
      amount: 1n,
    });
    results["TransferCollateral"] = hexEncode(ix.data);
  }

  // 18. TransferCollateralChildToParent
  {
    const ix = buildTransferCollateralChildToParentIx({
      trader: p(0),
      childTraderAccount: p(1),
      parentTraderAccount: p(2),
      perpAssetMap: p(3),
      globalTraderIndex: vec2(4, 5),
      activeTraderBuffer: vec2(6, 7),
    });
    results["TransferCollateralChildToParent"] = hexEncode(ix.data);
  }

  // 19. SyncParentToChild
  {
    const ix = buildSyncParentToChildIx({
      traderWallet: p(0),
      parentTraderAccount: p(1),
      childTraderAccount: p(2),
      globalTraderIndex: vec2(3, 4),
    });
    results["SyncParentToChild"] = hexEncode(ix.data);
  }

  // 20. OnboardTraderDelegated
  {
    const ix = buildOnboardTraderDelegatedIx({
      authority: p(0),
      permissionAccount: p(1),
      traderAccount: p(2),
      globalTraderIndex: vec2(3, 4),
      activeTraderBuffer: vec2(5, 6),
    });
    results["OnboardTraderDelegated"] = hexEncode(ix.data);
  }

  // 21. Flight RegisterBuilder
  {
    const ix = await flight.buildRegisterBuilderIx({
      traderAuthority: p(0),
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
      feeBps: 1n,
    });
    results["RegisterBuilder"] = hexEncode(ix.data);
  }

  // 22. Flight UpdateFee
  {
    const ix = await flight.buildUpdateFeeIx({
      traderAuthority: p(0),
      feeBps: 1n,
    });
    results["UpdateFee"] = hexEncode(ix.data);
  }

  // 23. Flight ProxyInstruction (wrapping a deterministic PlaceLimitOrder)
  {
    const inner = buildPlaceLimitOrderIx({
      trader: p(0),
      traderAccount: p(1),
      perpAssetMap: p(2),
      orderbook: p(3),
      splineCollection: p(4),
      globalTraderIndex: vec2(5, 6),
      activeTraderBuffer: vec2(7, 8),
      orderPacket: {
        side: Side.Bid,
        priceInTicks: ticks(1000n),
        numBaseLots: baseLots(100n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
    });

    const ix = await flight.buildProxyInstructionIx({
      builderAuthority: p(9),
      builderTraderAccount: p(10),
      traderWallet: p(0),
      innerInstruction: inner,
    });
    results["ProxyInstruction"] = hexEncode(ix.data);
  }

  // 24. Flight client wrapper for deterministic Flight-supported instructions
  {
    const flightClient = new flight.PhoenixFlightClient(
      {
        addresses: {
          phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
          logAuthorityAddress: PHOENIX_LOG_AUTHORITY_ADDRESS,
          globalConfigurationAddress: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
          usdcMintAddress: USDC_MINT_ADDRESS,
          emberStateAddress: p(11),
        },
        fetchAccount: async () => ({ data: new Uint8Array() }),
      } as any,
      {
        builderAuthority: p(9),
        builderPdaIndex: 0,
        builderSubaccountIndex: 0,
      }
    );

    const stopLossIx = buildPlaceStopLossIx({
      funder: p(0),
      traderAccount: p(1),
      perpAssetMap: p(3),
      orderbook: p(4),
      splineCollection: p(5),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      positionAuthority: p(2),
      stopLossAccount: p(10),
      triggerPrice: ticks(1000n),
      executionPrice: ticks(999n),
      tradeSide: Side.Ask,
      executionDirection: Direction.LessThan,
      orderKind: StopLossOrderKind.IOC,
    });
    const wrappedStopLossIx = await flightClient.tryWrapFlightInstruction(
      stopLossIx,
      p(0)
    );
    results["FlightPlaceStopLoss"] = hexEncode(wrappedStopLossIx.data);

    const positionConditionalIx = buildPlacePositionConditionalOrderIx({
      payer: p(0),
      traderAccount: p(1),
      perpAssetMap: p(3),
      orderbook: p(4),
      splineCollection: p(5),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      traderWallet: p(2),
      traderConditionalOrders: p(10),
      assetId: 0,
      greaterTriggerOrder: {
        triggerDirection: Direction.GreaterThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(1000n),
        executionPrice: ticks(999n),
      },
      lessTriggerOrder: null,
      sizeBaseLots: baseLots(100n),
      sizePercent: null,
    });
    const wrappedPositionConditionalIx =
      await flightClient.tryWrapFlightInstruction(positionConditionalIx, p(0));
    results["FlightPlacePositionConditionalOrder"] = hexEncode(
      wrappedPositionConditionalIx.data
    );

    const attachedConditionalIx = buildPlaceAttachedConditionalOrderIx({
      traderAccount: p(1),
      traderWallet: p(2),
      orderbook: p(4),
      traderConditionalOrders: p(10),
      payer: p(0),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      orderId: {
        priceInTicks: ticks(1000n),
        orderSequenceNumber: 0n,
      },
      assetId: 0,
      greaterTriggerOrder: null,
      lessTriggerOrder: {
        triggerDirection: Direction.LessThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(900n),
        executionPrice: ticks(899n),
      },
    });
    const wrappedAttachedConditionalIx =
      await flightClient.tryWrapFlightInstruction(attachedConditionalIx, p(0));
    results["FlightPlaceAttachedConditionalOrder"] = hexEncode(
      wrappedAttachedConditionalIx.data
    );

    const limitWithConditionalsIx = buildPlaceLimitOrderWithConditionalsIx({
      traderWallet: p(2),
      traderAccount: p(1),
      perpAssetMap: p(3),
      orderbook: p(4),
      splineCollection: p(5),
      globalTraderIndex: vec2(6, 7),
      activeTraderBuffer: vec2(8, 9),
      payer: p(0),
      traderConditionalOrders: p(10),
      orderPacket: {
        __kind: "Limit",
        side: Side.Bid,
        priceInTicks: ticks(1000n),
        numBaseLots: baseLots(100n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
      slot: 0n,
      greaterTriggerOrder: {
        triggerDirection: Direction.GreaterThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(1100n),
        executionPrice: ticks(1099n),
      },
      lessTriggerOrder: {
        triggerDirection: Direction.LessThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(900n),
        executionPrice: ticks(899n),
      },
    });
    const wrappedLimitWithConditionalsIx =
      await flightClient.tryWrapFlightInstruction(
        limitWithConditionalsIx,
        p(0)
      );
    results["FlightPlaceLimitOrderWithConditionals"] = hexEncode(
      wrappedLimitWithConditionalsIx.data
    );
  }

  // 19. PlaceMarketOrderDelegated
  {
    console.error("Building PlaceMarketOrderDelegated...");
    const delegatedIx = buildPlaceMarketOrderDelegatedIx({
      traderWallet: p(9),
      permissionAccount: p(10),
      traderAccount: p(1),
      perpAssetMap: p(2),
      orderbook: p(3),
      splineCollection: p(4),
      globalTraderIndex: vec2(5, 6),
      activeTraderBuffer: vec2(7, 8),
      orderPacket: {
        side: Side.Ask,
        priceInTicks: null,
        numBaseLots: baseLots(100n),
        numQuoteLots: null,
        minBaseLotsToFill: baseLots(0n),
        minQuoteLotsToFill: quoteLots(0n),
        selfTradeBehavior: SelfTradeBehavior.Abort,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: OrderFlags.None,
        cancelExisting: false,
      },
    });
    results["PlaceMarketOrderDelegated"] = hexEncode(delegatedIx.data);

    // 20. Flight wrapper for PlaceMarketOrderDelegated
    console.error("Building FlightPlaceMarketOrderDelegated...");
    const flightClient = new flight.PhoenixFlightClient(
      {
        addresses: {
          phoenixProgramAddress: PHOENIX_PROGRAM_ADDRESS,
          logAuthorityAddress: PHOENIX_LOG_AUTHORITY_ADDRESS,
          globalConfigurationAddress: PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
          usdcMintAddress: USDC_MINT_ADDRESS,
          emberStateAddress: p(11),
        },
        fetchAccount: async () => ({ data: new Uint8Array() }),
      } as any,
      {
        builderAuthority: p(9),
        builderPdaIndex: 0,
        builderSubaccountIndex: 0,
      }
    );
    const wrappedDelegatedIx = await flightClient.tryWrapFlightInstruction(
      delegatedIx,
      p(0)
    );
    results["FlightPlaceMarketOrderDelegated"] = hexEncode(
      wrappedDelegatedIx.data
    );
  }

  // Output JSON in sorted order
  const json = JSON.stringify(
    Object.fromEntries(
      Object.entries(results).sort(([a], [b]) => a.localeCompare(b))
    ),
    null,
    0
  );
  console.log(json);
  console.error(
    `✓ All ${Object.keys(results).length} shared instruction builders succeeded`
  );
} catch (e) {
  console.error("✗ Error:", (e as Error).message);
  console.error((e as Error).stack);
  process.exit(1);
}
