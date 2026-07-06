import { describe, expect, it } from "vitest";
import {
  computeSubaccountMarginFromInputs,
  computeTraderMarginFromInputs,
  createMarginCalculator,
  type MarketParams,
  type SubaccountMarginInputs,
  type TraderMarginInputs,
} from "@/margin";

const marketParams: MarketParams = {
  symbol: "SOL-PERP",
  assetId: 1,
  markPriceTicks: "100",
  tickSize: "1",
  baseLotDecimals: 0,
  leverageTiers: [
    {
      upperBoundSize: "1000",
      maxLeverage: "20",
      limitOrderRiskFactorBps: "10000",
    },
  ],
  riskFactors: {
    maintenanceMarginFactorBps: "5000",
    backstopMarginFactorBps: "2000",
    highRiskMarginFactorBps: "1000",
  },
  cancelOrderRiskFactorBps: "7000",
  upnlRiskFactor: "10000",
  upnlRiskFactorForWithdrawals: "10000",
  isolatedOnly: false,
};

const positionInputs = (): SubaccountMarginInputs => ({
  subaccountIndex: 0,
  collateralBalanceQuoteLots: "1000",
  markets: [
    {
      symbol: "SOL-PERP",
      position: {
        basePositionLots: "10",
        virtualQuotePositionLots: "-1000",
        entryPriceTicks: "100",
        unsettledFundingQuoteLots: "0",
        accumulatedFundingQuoteLots: "0",
      },
    },
  ],
});

const limitOrderInputs = (): SubaccountMarginInputs => ({
  subaccountIndex: 0,
  collateralBalanceQuoteLots: "1000",
  markets: [
    {
      symbol: "SOL-PERP",
      limitOrders: [
        {
          orderSequenceNumber: "1",
          side: "bid",
          priceTicks: "100",
          sizeRemainingLots: "10",
          initialSizeLots: "10",
          reduceOnly: false,
        },
      ],
    },
  ],
});

const calculator = createMarginCalculator([marketParams]);

const secondMarketParams: MarketParams = {
  ...marketParams,
  symbol: "ETH-PERP",
  assetId: 2,
  markPriceTicks: "200",
};

const multiMarketInputs = (): SubaccountMarginInputs => ({
  subaccountIndex: 0,
  collateralBalanceQuoteLots: "1000",
  markets: [
    {
      symbol: "SOL-PERP",
      position: {
        basePositionLots: "10",
        virtualQuotePositionLots: "-1000",
        entryPriceTicks: "100",
        unsettledFundingQuoteLots: "0",
        accumulatedFundingQuoteLots: "0",
      },
    },
    {
      symbol: "ETH-PERP",
      position: {
        basePositionLots: "5",
        virtualQuotePositionLots: "-1000",
        entryPriceTicks: "200",
        unsettledFundingQuoteLots: "0",
        accumulatedFundingQuoteLots: "0",
      },
    },
  ],
});

const limitOrderOnPositionInputs = (): SubaccountMarginInputs => ({
  subaccountIndex: 0,
  collateralBalanceQuoteLots: "1000",
  markets: [
    {
      symbol: "SOL-PERP",
      position: {
        basePositionLots: "10",
        virtualQuotePositionLots: "-1000",
        entryPriceTicks: "100",
        unsettledFundingQuoteLots: "0",
        accumulatedFundingQuoteLots: "0",
      },
      limitOrders: [
        {
          orderSequenceNumber: "1",
          side: "bid",
          priceTicks: "100",
          sizeRemainingLots: "10",
          initialSizeLots: "10",
          reduceOnly: false,
        },
      ],
    },
  ],
});

const traderInputs = (): TraderMarginInputs => ({
  authority: "authority",
  traderPdaIndex: 0,
  subaccounts: [positionInputs()],
});

const expectNoMarketLevelOrderLeverageAdjustedFields = (
  marketMargin: unknown
) => {
  expect(marketMargin).toBeDefined();
  expect(marketMargin).not.toHaveProperty(
    "orderLeverageAdjustedPositionInitialMarginQuoteLots"
  );
  expect(marketMargin).not.toHaveProperty(
    "orderLeverageAdjustedInitialMarginQuoteLots"
  );
  expect(marketMargin).not.toHaveProperty(
    "orderLeverageAdjustedLimitOrderMarginQuoteLots"
  );
};

const expectNoAggregateOrderLeverageAdjustedLimitOrderMargin = (
  margin: unknown
) => {
  expect(margin).not.toHaveProperty(
    "orderLeverageAdjustedLimitOrderMarginQuoteLots"
  );
};

describe("order leverage limits in margin calculations", () => {
  it("preserves protocol leverage outputs when no options are provided", () => {
    const baseline =
      calculator.computeSubaccountMarginFromInputs(positionInputs());
    const emptyOptions = calculator.computeSubaccountMarginFromInputs(
      positionInputs(),
      {}
    );

    expect(emptyOptions).toEqual(baseline);
    expect(baseline.margin.initialMarginQuoteLots).toBe("50");
    expect(
      baseline.margin.orderLeverageAdjustedInitialMarginQuoteLots
    ).toBeUndefined();
    expect(baseline.marketMargins[0]?.positionInitialMarginQuoteLots).toBe(
      "50"
    );
    expectNoAggregateOrderLeverageAdjustedLimitOrderMargin(baseline.margin);
    expectNoMarketLevelOrderLeverageAdjustedFields(baseline.marketMargins[0]);
  });

  it("uses a lower floored limit for order-leverage-adjusted display margin", () => {
    const result = calculator.computeSubaccountMarginFromInputs(
      positionInputs(),
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 5.9,
        },
      }
    );

    expect(result.margin.initialMarginQuoteLots).toBe("50");
    expect(result.margin.orderLeverageAdjustedInitialMarginQuoteLots).toBe(
      "200"
    );
    expect(result.marketMargins[0]?.initialMarginQuoteLots).toBe("50");
    expect(result.marketMargins[0]?.positionInitialMarginQuoteLots).toBe("50");
    expectNoAggregateOrderLeverageAdjustedLimitOrderMargin(result.margin);
    expectNoMarketLevelOrderLeverageAdjustedFields(result.marketMargins[0]);
  });

  it("ignores limits above the market max leverage", () => {
    const baseline =
      calculator.computeSubaccountMarginFromInputs(positionInputs());
    const result = calculator.computeSubaccountMarginFromInputs(
      positionInputs(),
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 50,
        },
      }
    );

    expect(result).toEqual(baseline);
  });

  it("ignores invalid limits", () => {
    const invalidLimits = [
      Number.NaN,
      Infinity,
      Number.MAX_SAFE_INTEGER + 1,
      Number.MAX_VALUE,
      0.99,
      0,
      -1,
    ];

    for (const invalidLimit of invalidLimits) {
      const result = calculator.computeSubaccountMarginFromInputs(
        positionInputs(),
        {
          orderLeverageLimitsBySymbol: {
            "SOL-PERP": invalidLimit,
          },
        }
      );

      expect(result.margin.initialMarginQuoteLots).toBe("50");
      expect(
        result.margin.orderLeverageAdjustedInitialMarginQuoteLots
      ).toBeUndefined();
      expect(result.marketMargins[0]?.positionInitialMarginQuoteLots).toBe(
        "50"
      );
      expectNoAggregateOrderLeverageAdjustedLimitOrderMargin(result.margin);
      expectNoMarketLevelOrderLeverageAdjustedFields(result.marketMargins[0]);
    }
  });

  it("does not apply order leverage limits to withdrawal initial margin", () => {
    const baseline =
      calculator.computeSubaccountMarginFromInputs(positionInputs());
    const result = calculator.computeSubaccountMarginFromInputs(
      positionInputs(),
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 5,
        },
      }
    );

    expect(result.margin.initialMarginQuoteLots).toBe("50");
    expect(result.margin.orderLeverageAdjustedInitialMarginQuoteLots).toBe(
      "200"
    );
    expect(result.margin.initialMarginForWithdrawalsQuoteLots).toBe(
      baseline.margin.initialMarginForWithdrawalsQuoteLots
    );
    expect(result.margin.initialMarginForWithdrawalsQuoteLots).toBe("50");
  });

  it("does not apply order leverage limits to withdrawal initial margin for open limit orders", () => {
    const baseline =
      calculator.computeSubaccountMarginFromInputs(limitOrderInputs());
    const result = calculator.computeSubaccountMarginFromInputs(
      limitOrderInputs(),
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 5,
        },
      }
    );

    expect(result.margin.initialMarginQuoteLots).toBe("50");
    expect(result.margin.orderLeverageAdjustedInitialMarginQuoteLots).toBe(
      "200"
    );
    expect(result.margin.initialMarginForWithdrawalsQuoteLots).toBe(
      baseline.margin.initialMarginForWithdrawalsQuoteLots
    );
    expect(result.margin.initialMarginForWithdrawalsQuoteLots).toBe("50");
  });

  it("applies order leverage limits per symbol", () => {
    const multiMarketCalculator = createMarginCalculator([
      marketParams,
      secondMarketParams,
    ]);

    const result = multiMarketCalculator.computeSubaccountMarginFromInputs(
      multiMarketInputs(),
      {
        orderLeverageLimitsBySymbol: {
          "DOGE-PERP": 2,
          "SOL-PERP": 5,
        },
      }
    );

    const solMargin = result.marketMargins.find(
      (market) => market.symbol === "SOL-PERP"
    );
    const ethMargin = result.marketMargins.find(
      (market) => market.symbol === "ETH-PERP"
    );

    expect(result.margin.initialMarginQuoteLots).toBe("100");
    expect(result.margin.orderLeverageAdjustedInitialMarginQuoteLots).toBe(
      "250"
    );
    expect(solMargin?.initialMarginQuoteLots).toBe("50");
    expect(ethMargin?.initialMarginQuoteLots).toBe("50");
    expectNoMarketLevelOrderLeverageAdjustedFields(solMargin);
    expectNoMarketLevelOrderLeverageAdjustedFields(ethMargin);
  });

  it("threads order leverage limits through trader-level margin calls", () => {
    const result = computeTraderMarginFromInputs(
      traderInputs(),
      calculator.markets,
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 5,
        },
      }
    );
    const calculatorResult = calculator.computeTraderMarginFromInputs(
      traderInputs(),
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 5,
        },
      }
    );

    expect(result.subaccounts[0]?.margin.initialMarginQuoteLots).toBe("50");
    expect(
      result.subaccounts[0]?.margin.orderLeverageAdjustedInitialMarginQuoteLots
    ).toBe("200");
    expect(calculatorResult.subaccounts[0]?.margin.initialMarginQuoteLots).toBe(
      "50"
    );
    expect(
      calculatorResult.subaccounts[0]?.margin
        .orderLeverageAdjustedInitialMarginQuoteLots
    ).toBe("200");
  });

  it("uses effective leverage for order-leverage-adjusted limit-order margin", () => {
    const result = computeSubaccountMarginFromInputs(
      limitOrderInputs(),
      calculator.markets,
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 5,
        },
      }
    );

    expect(result.margin.initialMarginQuoteLots).toBe("50");
    expect(result.margin.orderLeverageAdjustedInitialMarginQuoteLots).toBe(
      "200"
    );
    expect(result.margin.limitOrderMarginQuoteLots).toBe("50");
    expectNoAggregateOrderLeverageAdjustedLimitOrderMargin(result.margin);
    expect(result.limitOrders[0]?.marginRequirementQuoteLots).toBe("50");
    expect(
      result.limitOrders[0]?.orderLeverageAdjustedMarginRequirementQuoteLots
    ).toBe("200");
  });

  it("uses effective leverage for limit-order margin over existing positions", () => {
    const result = calculator.computeSubaccountMarginFromInputs(
      limitOrderOnPositionInputs(),
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 5,
        },
      }
    );

    expect(result.margin.initialMarginQuoteLots).toBe("100");
    expect(result.margin.orderLeverageAdjustedInitialMarginQuoteLots).toBe(
      "400"
    );
    expect(result.marketMargins[0]?.positionInitialMarginQuoteLots).toBe("50");
    expectNoMarketLevelOrderLeverageAdjustedFields(result.marketMargins[0]);
    expect(result.margin.limitOrderMarginQuoteLots).toBe("50");
    expectNoAggregateOrderLeverageAdjustedLimitOrderMargin(result.margin);
    expect(result.limitOrders[0]?.marginRequirementQuoteLots).toBe("50");
    expect(
      result.limitOrders[0]?.orderLeverageAdjustedMarginRequirementQuoteLots
    ).toBe("200");
  });

  it("keeps protocol risk thresholds and labels on unclamped initial margin", () => {
    const result = calculator.computeSubaccountMarginFromInputs(
      {
        ...positionInputs(),
        collateralBalanceQuoteLots: "100",
      },
      {
        orderLeverageLimitsBySymbol: {
          "SOL-PERP": 5,
        },
      }
    );

    expect(result.margin.initialMarginQuoteLots).toBe("50");
    expect(result.margin.orderLeverageAdjustedInitialMarginQuoteLots).toBe(
      "200"
    );
    expect(result.margin.maintenanceMarginQuoteLots).toBe("25");
    expect(result.margin.riskState).toBe("healthy");
    expect(result.margin.riskTier).toBe("safe");
    expect(result.marketMargins[0]?.positionInitialMarginQuoteLots).toBe("50");
    expectNoMarketLevelOrderLeverageAdjustedFields(result.marketMargins[0]);
    expect(result.margin.cancelMarginQuoteLots).toBe("35");
    expect(result.margin.backstopMarginQuoteLots).toBe("10");
    expect(result.margin.highRiskMarginQuoteLots).toBe("5");
  });
});
