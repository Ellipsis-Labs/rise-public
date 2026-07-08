import { describe, expect, it } from "vitest";
import { createMarginCalculator } from "@/margin";
import type { MarketParams } from "@/margin";

// Mirrors the canonical Rust test
// `limit_order_margin_includes_adverse_fill_loss_against_mark`
// (program-core/exchange/src/margin.rs): tick size 10, mark price 100 ticks
// (= 1000 quote lots per base lot), flat 10x leverage, 100% limit-order risk
// factor so the leverage-based order margin is undiscounted.
const fillLossMarketParams = (): MarketParams => ({
  symbol: "TEST-PERP",
  assetId: 1,
  markPriceTicks: "100",
  tickSize: "10",
  baseLotDecimals: 0,
  leverageTiers: [
    {
      upperBoundSize: "1000000",
      maxLeverage: "10",
      limitOrderRiskFactorBps: "10000",
    },
  ],
  riskFactors: {
    maintenanceMarginFactorBps: "5000",
    backstopMarginFactorBps: "4000",
    highRiskMarginFactorBps: "3000",
  },
  cancelOrderRiskFactorBps: "5500",
  upnlRiskFactor: "5000",
  upnlRiskFactorForWithdrawals: "7500",
  isolatedOnly: false,
});

const computeInitialMargin = (
  orders: Array<{
    side: "bid" | "ask";
    priceTicks: string;
    sizeRemainingLots: string;
    reduceOnly: boolean;
  }>
): { initialMargin: string; limitOrderMargin: string } => {
  const calculator = createMarginCalculator([fillLossMarketParams()]);
  const result = calculator.computeSubaccountMarginFromInputs({
    subaccountIndex: 0,
    collateralBalanceQuoteLots: "0",
    markets: [
      {
        symbol: "TEST-PERP",
        limitOrders: orders.map((order, index) => ({
          orderSequenceNumber: String(index),
          side: order.side,
          priceTicks: order.priceTicks,
          sizeRemainingLots: order.sizeRemainingLots,
          initialSizeLots: order.sizeRemainingLots,
          reduceOnly: order.reduceOnly,
        })),
      },
    ],
  });
  return {
    initialMargin: result.margin.initialMarginQuoteLots,
    limitOrderMargin: result.margin.limitOrderMarginQuoteLots,
  };
};

describe("limit order adverse-fill-loss reserve", () => {
  it("reserves fill loss for a resting bid above mark", () => {
    // Bid at 200 ticks vs mark at 100 ticks: leverage margin 1_000 plus an
    // adverse fill loss of 10 * (2000 - 1000) = 10_000.
    const { initialMargin, limitOrderMargin } = computeInitialMargin([
      {
        side: "bid",
        priceTicks: "200",
        sizeRemainingLots: "10",
        reduceOnly: false,
      },
    ]);
    expect(initialMargin).toBe("11000");
    expect(limitOrderMargin).toBe("11000");
  });

  it("reserves fill loss for a resting ask below mark", () => {
    // Ask at 50 ticks vs mark at 100 ticks: leverage margin 1_000 plus an
    // adverse fill loss of 10 * (1000 - 500) = 5_000.
    const { initialMargin } = computeInitialMargin([
      {
        side: "ask",
        priceTicks: "50",
        sizeRemainingLots: "10",
        reduceOnly: false,
      },
    ]);
    expect(initialMargin).toBe("6000");
  });

  it("reserves nothing for orders resting on the passive side of mark", () => {
    // Bid below mark and ask above mark fill out-of-the-money, so no fill-loss
    // reserve; only the leverage-based order margin (1_000 each, max = 1_000).
    const { initialMargin } = computeInitialMargin([
      {
        side: "bid",
        priceTicks: "50",
        sizeRemainingLots: "10",
        reduceOnly: false,
      },
      {
        side: "ask",
        priceTicks: "200",
        sizeRemainingLots: "10",
        reduceOnly: false,
      },
    ]);
    expect(initialMargin).toBe("1000");
  });
});

const computeInitialMarginWithPosition = (
  basePositionLots: string,
  orders: Array<{
    side: "bid" | "ask";
    priceTicks: string;
    sizeRemainingLots: string;
    reduceOnly: boolean;
  }>
): string => {
  const calculator = createMarginCalculator([fillLossMarketParams()]);
  const result = calculator.computeSubaccountMarginFromInputs({
    subaccountIndex: 0,
    collateralBalanceQuoteLots: "0",
    markets: [
      {
        symbol: "TEST-PERP",
        position: {
          basePositionLots,
          virtualQuotePositionLots: "0",
          entryPriceTicks: "0",
          unsettledFundingQuoteLots: "0",
          accumulatedFundingQuoteLots: "0",
        },
        limitOrders: orders.map((order, index) => ({
          orderSequenceNumber: String(index),
          side: order.side,
          priceTicks: order.priceTicks,
          sizeRemainingLots: order.sizeRemainingLots,
          initialSizeLots: order.sizeRemainingLots,
          reduceOnly: order.reduceOnly,
        })),
      },
    ],
  });
  return result.margin.initialMarginQuoteLots;
};

describe("reduce-only fill-loss capping by opposite position", () => {
  it("caps a huge reduce-only order to the small opposite position", () => {
    // Long 5 lots with a reduce-only ask of 1_000 lots at 50 ticks. Only 5 lots
    // can actually reduce the long, so the reserve is 5 * (1000 - 500) = 2_500,
    // plus position margin 500 -> 3_000. Uncapped it would be 500_500.
    const initialMargin = computeInitialMarginWithPosition("5", [
      {
        side: "ask",
        priceTicks: "50",
        sizeRemainingLots: "1000",
        reduceOnly: true,
      },
    ]);
    expect(initialMargin).toBe("3000");
  });

  it("reserves nothing for a reduce-only order on the position's own side", () => {
    // Long 5 lots with a reduce-only bid: a bid would increase the long, so it
    // is capped to zero and reserves no fill loss. Only position margin (500).
    const initialMargin = computeInitialMarginWithPosition("5", [
      {
        side: "bid",
        priceTicks: "200",
        sizeRemainingLots: "1000",
        reduceOnly: true,
      },
    ]);
    expect(initialMargin).toBe("500");
  });

  it("reserves the full fill loss when reduce-only fits within the position", () => {
    // Short 10 lots with a reduce-only bid of 10 lots at 200 ticks: not capped,
    // reserve 10 * (2000 - 1000) = 10_000 plus position margin 1_000 -> 11_000.
    const initialMargin = computeInitialMarginWithPosition("-10", [
      {
        side: "bid",
        priceTicks: "200",
        sizeRemainingLots: "10",
        reduceOnly: true,
      },
    ]);
    expect(initialMargin).toBe("11000");
  });
});
