import { describe, expect, it } from "vitest";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import {
  TraderStateServerMessageSchema,
  TraderStateServerMessageToUpdateSchema,
  TraderStateUpdateSchema,
} from "@/ws/adapters/trader-state/wire";

const capabilities = {
  flags: 127,
  state: "active" as const,
  capabilities: {
    placeLimitOrder: { immediate: true, viaColdActivation: true },
    placeMarketOrder: { immediate: true, viaColdActivation: true },
    riskIncreasingTrade: { immediate: true, viaColdActivation: true },
    riskReducingTrade: { immediate: true, viaColdActivation: true },
    depositCollateral: { immediate: true, viaColdActivation: true },
    withdrawCollateral: { immediate: true, viaColdActivation: true },
  },
};

const samplePosition = {
  positionSequenceNumber: "1",
  basePositionLots: "10",
  entryPriceTicks: "100",
  virtualQuotePositionLots: "-1000",
  unsettledFundingQuoteLots: "0",
  accumulatedFundingQuoteLots: "0",
  takeProfitTriggers: [
    {
      takeProfitId: "tp-1-1-gt",
      trigger: {
        triggerPriceTicks: "120",
        executionPriceTicks: "119",
        side: "ask" as const,
        kind: "ioc" as const,
      },
      status: "active",
    },
  ],
  stopLossTriggers: [
    {
      stopLossId: "sl-1-1-lt",
      trigger: {
        triggerPriceTicks: "95",
        executionPriceTicks: "94",
        side: "ask" as const,
        kind: "ioc" as const,
      },
      status: "active",
    },
  ],
  conditionalTakeProfitTriggers: [
    {
      conditionalTakeProfitId: "ctp-1-1-gt",
      trigger: {
        triggerPriceTicks: "130",
        executionPriceTicks: "129",
        side: "ask" as const,
        kind: "ioc" as const,
        attachedOrderSequenceNumber: "7",
        maxSizeLots: "10",
        fillableSizeLots: "9",
        filledSizeLots: "1",
        usePercent: true,
        percent: 25,
      },
      status: "active",
    },
  ],
  conditionalStopLossTriggers: [
    {
      conditionalStopLossId: "csl-1-1-lt",
      trigger: {
        triggerPriceTicks: "90",
        executionPriceTicks: "89",
        side: "ask" as const,
        kind: "limit" as const,
        attachedOrderSequenceNumber: null,
        maxSizeLots: "10",
        fillableSizeLots: "10",
        filledSizeLots: "0",
        usePercent: false,
        percent: 0,
      },
      status: "active",
    },
  ],
};

const sampleOrder = {
  orderSequenceNumber: "7",
  side: "ask" as const,
  orderType: "limit",
  conditionalKind: "trigger",
  priceTicks: "129",
  priceUsd: "1.29",
  sizeRemainingLots: "9",
  initialSizeLots: "10",
  reduceOnly: false,
  isStopLoss: false,
  isStopLossDirection: false,
  isConditionalOrder: true,
  status: "active",
};

describe("trader-state wire", () => {
  it("preserves TP/SL arrays for snapshot payloads", () => {
    const parsed = TraderStateServerMessageSchema.parse({
      channel: "traderState",
      authority: "auth",
      traderPdaIndex: 0,
      slot: "123",
      messageType: "snapshot",
      version: 1,
      capabilities,
      makerFeeOverrideMultiplier: 1,
      takerFeeOverrideMultiplier: 1,
      subaccounts: [
        {
          subaccountIndex: 0,
          sequence: 1,
          collateral: "0",
          positions: [{ symbol: "SOL-PERP", ...samplePosition }],
          orders: [{ symbol: "SOL-PERP", orders: [sampleOrder] }],
          splines: [],
        },
      ],
      deltas: [],
    });

    const position = parsed.subaccounts?.[0]?.positions?.[0];
    const order = parsed.subaccounts?.[0]?.orders?.[0]?.orders?.[0];
    expect(parsed.slot).toBe(123n);
    expect(position?.takeProfitTriggers[0]?.takeProfitId).toBe("tp-1-1-gt");
    expect(position?.stopLossTriggers[0]?.stopLossId).toBe("sl-1-1-lt");
    expect(
      position?.conditionalTakeProfitTriggers[0]?.trigger.maxSizeLots
    ).toBe("10");
    expect(
      position?.conditionalStopLossTriggers[0]?.conditionalStopLossId
    ).toBe("csl-1-1-lt");
    expect(order?.isConditionalOrder).toBe(true);
  });

  it("preserves TP/SL arrays for delta payloads", () => {
    const parsed = TraderStateServerMessageSchema.parse({
      channel: "traderState",
      authority: "auth",
      traderPdaIndex: 0,
      slot: "124",
      messageType: "delta",
      deltas: [
        {
          subaccountIndex: 0,
          sequence: 2,
          collateral: "0",
          positions: [
            {
              symbol: "SOL-PERP",
              change: "updated",
              position: samplePosition,
            },
          ],
          orders: [{ symbol: "SOL-PERP", orders: [sampleOrder] }],
          splines: [],
          tradeHistory: [],
          orderHistory: [],
        },
      ],
    });

    const position = parsed.deltas?.[0]?.positions?.[0]?.position;
    const order = parsed.deltas?.[0]?.orders?.[0]?.orders?.[0];
    expect(parsed.slot).toBe(124n);
    expect(position?.takeProfitTriggers[0]?.takeProfitId).toBe("tp-1-1-gt");
    expect(position?.stopLossTriggers[0]?.stopLossId).toBe("sl-1-1-lt");
    expect(
      position?.conditionalTakeProfitTriggers[0]?.trigger
        .attachedOrderSequenceNumber
    ).toBe("7");
    expect(order?.isConditionalOrder).toBe(true);
  });

  it("preserves trade history signatures and defaults missing signatures to null", () => {
    const parsed = TraderStateServerMessageSchema.parse({
      channel: "traderState",
      authority: "auth",
      traderPdaIndex: 0,
      slot: "128",
      messageType: "delta",
      deltas: [
        {
          subaccountIndex: 0,
          sequence: 2,
          collateral: "0",
          positions: [],
          orders: [],
          splines: [],
          triggers: [],
          tradeHistory: [
            {
              timestamp: 1,
              slot: 2,
              slotIndex: 3,
              instructionIndex: 4,
              eventIndex: 5,
              market: "SOL-PERP",
              instructionType: "Fill",
              tradeType: "limit",
              fillId: "fill-1",
              baseQtyBefore: "0",
              baseQtyAfter: "1",
              size: "1",
              liquidity: "maker",
              price: "10",
              fee: "0.01",
              realizedPnl: "0",
              signature: "trade-sig",
            },
            {
              timestamp: 2,
              slot: 3,
              slotIndex: 4,
              instructionIndex: 5,
              eventIndex: 6,
              market: "BTC-PERP",
              instructionType: "Fill",
              tradeType: "market",
              baseQtyBefore: "0",
              baseQtyAfter: "1",
              size: "1",
              liquidity: "taker",
              price: "20",
              fee: "0.02",
              realizedPnl: "0",
            },
          ],
          orderHistory: [],
        },
      ],
    });

    expect(parsed.deltas?.[0]?.tradeHistory[0]?.signature).toBe("trade-sig");
    expect(parsed.deltas?.[0]?.tradeHistory[0]?.fillId).toBe("fill-1");
    expect(parsed.deltas?.[0]?.tradeHistory[1]?.signature).toBeNull();
    expect(parsed.deltas?.[0]?.tradeHistory[1]?.fillId).toBeNull();
  });

  it("accepts conditional fields in strict mode without dropping them", () => {
    const strictSchema = applyStrictModeRecursive(
      TraderStateServerMessageSchema
    );
    const parsed = strictSchema.parse({
      channel: "traderState",
      authority: "auth",
      traderPdaIndex: 0,
      slot: "125",
      messageType: "snapshot",
      version: 1,
      capabilities,
      makerFeeOverrideMultiplier: 1,
      takerFeeOverrideMultiplier: 1,
      subaccounts: [
        {
          subaccountIndex: 0,
          sequence: 3,
          collateral: "0",
          positions: [{ symbol: "SOL-PERP", ...samplePosition }],
          orders: [{ symbol: "SOL-PERP", orders: [sampleOrder] }],
          splines: [],
        },
      ],
      deltas: [],
    });

    const position = parsed.subaccounts?.[0]?.positions?.[0];
    const order = parsed.subaccounts?.[0]?.orders?.[0]?.orders?.[0];
    expect(position?.conditionalTakeProfitTriggers).toHaveLength(1);
    expect(position?.conditionalStopLossTriggers).toHaveLength(1);
    expect(order?.conditionalKind).toBe("trigger");
  });

  it("accepts adapter output without a channel field", () => {
    const parsed = TraderStateUpdateSchema.parse({
      authority: "auth",
      traderPdaIndex: 0,
      slot: 126n,
      messageType: "snapshot",
      version: 1,
      capabilities,
      makerFeeOverrideMultiplier: 1,
      takerFeeOverrideMultiplier: 1,
      subaccounts: [
        {
          subaccountIndex: 0,
          sequence: 3,
          collateral: "0",
          positions: [{ symbol: "SOL-PERP", ...samplePosition }],
          orders: [{ symbol: "SOL-PERP", orders: [sampleOrder] }],
          splines: [],
          triggers: [],
        },
      ],
      deltas: [],
    });

    expect(parsed.slot).toBe(126n);
    expect(
      parsed.subaccounts[0]?.positions[0]?.takeProfitTriggers
    ).toHaveLength(1);
  });

  it("still transforms raw server messages into adapter updates", () => {
    const parsed = TraderStateServerMessageToUpdateSchema.parse({
      channel: "traderState",
      authority: "auth",
      traderPdaIndex: 0,
      slot: "127",
      messageType: "delta",
      deltas: [
        {
          subaccountIndex: 0,
          sequence: 2,
          collateral: "0",
          positions: [],
          orders: [],
          splines: [],
          triggers: [],
          tradeHistory: [],
          orderHistory: [],
        },
      ],
    });

    expect(parsed.slot).toBe(127n);
    expect(parsed.deltas).toHaveLength(1);
    expect(parsed.subaccounts).toEqual([]);
  });
});
