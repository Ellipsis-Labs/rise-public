import { describe, expect, it } from "vitest";
import {
  calculateLiquidationPriceUsd,
  computeMarketLiquidationPriceFromMargin,
  computeSubaccountLiquidationPricesFromMargin,
  computeSubaccountLiquidationPricesFromInputs,
  createMarginCalculator,
  simulatePositionFillFromInputs,
  type MarketParams,
  type MarginPositionState,
  type SubaccountMarginInputs,
} from "@/margin";

const MICRO_USD = 1_000_000;
const BASE_LOTS_PER_UNIT = 100;

const quoteLots = (usd: number): string =>
  Math.round(usd * MICRO_USD).toString();

const priceTicks = (priceUsd: number): string =>
  Math.round((priceUsd * MICRO_USD) / BASE_LOTS_PER_UNIT).toString();

const priceTicksWithParams = (
  priceUsd: number,
  tickSize: number,
  baseLotDecimals: number
): string =>
  Math.round(
    (priceUsd * MICRO_USD) / (tickSize * Math.pow(10, baseLotDecimals))
  ).toString();

const market = (
  symbol: string,
  assetId: number,
  markPriceUsd: number,
  maxLeverage: number,
  limitOrderRiskFactorBps: string = "5000"
): MarketParams => ({
  symbol,
  assetId,
  markPriceTicks: priceTicks(markPriceUsd),
  tickSize: "1",
  baseLotDecimals: 2,
  leverageTiers: [
    {
      upperBoundSize: "1000000000",
      maxLeverage: maxLeverage.toString(),
      limitOrderRiskFactorBps,
    },
  ],
  riskFactors: {
    maintenanceMarginFactorBps: "5000",
    backstopMarginFactorBps: "10000",
    highRiskMarginFactorBps: "20000",
  },
  cancelOrderRiskFactorBps: "5000",
  upnlRiskFactor: "10000",
  upnlRiskFactorForWithdrawals: "10000",
  isolatedOnly: false,
});

const snapshotMarket = (
  symbol: string,
  assetId: number,
  markPriceUsd: number,
  upperBoundSize: string
): MarketParams => ({
  ...market(symbol, assetId, markPriceUsd, 10, "10000"),
  markPriceTicks: priceTicksWithParams(markPriceUsd, 10, 2),
  tickSize: "10",
  leverageTiers: [
    {
      upperBoundSize,
      maxLeverage: "10",
      limitOrderRiskFactorBps: "10000",
    },
  ],
});

const position = (
  symbol: string,
  baseUnits: number,
  virtualQuoteUsd: number,
  unsettledFundingUsd: number = 0
): { symbol: string; position: MarginPositionState } => ({
  symbol,
  position: {
    basePositionLots: Math.round(baseUnits * BASE_LOTS_PER_UNIT).toString(),
    virtualQuotePositionLots: quoteLots(virtualQuoteUsd),
    entryPriceTicks: priceTicks(Math.abs(virtualQuoteUsd / baseUnits)),
    unsettledFundingQuoteLots: quoteLots(unsettledFundingUsd),
    accumulatedFundingQuoteLots: "0",
  },
});

const findPosition = (
  result: ReturnType<typeof computeSubaccountLiquidationPricesFromInputs>,
  symbol: string
) => result.positions.find((marketResult) => marketResult.symbol === symbol);

describe("margin liquidation price", () => {
  it.each([
    {
      name: "long with other-market losses and maintenance",
      positionSize: 1,
      entryPrice: 100,
      leverage: 50,
      collateralUsd: 100,
      otherAssetUnrealizedPnlUsd: -50,
      otherAssetMaintenanceMarginUsd: 300,
      expectedPriceUsd: 353.5353535353535,
    },
    {
      name: "short with other-market gains and maintenance",
      positionSize: -2,
      entryPrice: 200,
      leverage: 25,
      collateralUsd: 250,
      otherAssetUnrealizedPnlUsd: 40,
      otherAssetMaintenanceMarginUsd: 150,
      expectedPriceUsd: 264.70588235294116,
    },
    {
      name: "sample BTC short",
      positionSize: -0.166,
      entryPrice: 101458,
      leverage: 20,
      collateralUsd: 841,
      otherAssetUnrealizedPnlUsd: 0,
      otherAssetMaintenanceMarginUsd: 0,
      expectedPriceUsd: 103926.11225389363,
    },
    {
      name: "NVDA short with unsettled funding in collateral",
      positionSize: -42.21,
      entryPrice: 201.55,
      leverage: 20,
      collateralUsd: 700.421673 + 14.05593,
      otherAssetUnrealizedPnlUsd: 0,
      otherAssetMaintenanceMarginUsd: 0,
      expectedPriceUsd: 213.14803688872712,
    },
  ])("matches phoenix-state solver case: $name", (testCase) => {
    expect(
      calculateLiquidationPriceUsd({
        positionSize: testCase.positionSize,
        entryPriceUsd: testCase.entryPrice,
        leverage: testCase.leverage,
        maintenanceMarginBps: 5000,
        collateralUsd: testCase.collateralUsd,
        otherAssetUnrealizedPnlUsd: testCase.otherAssetUnrealizedPnlUsd,
        otherAssetMaintenanceMarginUsd: testCase.otherAssetMaintenanceMarginUsd,
      })
    ).toBeCloseTo(testCase.expectedPriceUsd, 8);
  });

  it.each([
    {
      name: "long",
      positionSize: 1,
      expectedPriceUsd: 101.01010101010101,
    },
    {
      name: "short",
      positionSize: -1,
      expectedPriceUsd: 79.20792079207921,
    },
  ])(
    "discounts target-market positive uPnL in the $name liquidation regime",
    ({ positionSize, expectedPriceUsd }) => {
      expect(
        calculateLiquidationPriceUsd({
          positionSize,
          entryPriceUsd: 100,
          leverage: 10,
          maintenanceMarginBps: 500,
          upnlRiskFactorBps: 5000,
          collateralUsd: positionSize > 0 ? 100 : 0,
          otherAssetUnrealizedPnlUsd: positionSize > 0 ? -100 : -10,
          otherAssetMaintenanceMarginUsd: 0,
        })
      ).toBeCloseTo(expectedPriceUsd, 8);
    }
  );

  it("returns the current mark for already-liquidatable long positions", () => {
    const calculator = createMarginCalculator([
      market("SOL-PERP", 1, 100, 50),
      market("ETH-PERP", 2, 100, 10),
    ]);
    const inputs: SubaccountMarginInputs = {
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(100),
      markets: [position("SOL-PERP", 1, -100), position("ETH-PERP", 60, -6050)],
    };

    const result =
      calculator.computeSubaccountLiquidationPricesFromInputs(inputs);
    const standalone = computeSubaccountLiquidationPricesFromInputs(
      inputs,
      calculator.markets
    );

    expect(standalone).toEqual(result);
    const sol = findPosition(result, "SOL-PERP");
    expect(sol?.basePositionLots).toBe("100");
    expect(sol?.basePositionUnits).toBe(1);
    expect(sol?.entryPriceUsd).toBe(100);
    expect(sol?.liquidationPriceUsd).toBe(100);
    expect(sol?.liquidationPriceTicks).toBe("1000000");
  });

  it("returns the current mark for already-liquidatable short positions", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 200, 25)]);
    const result = calculator.computeSubaccountLiquidationPricesFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(0),
      markets: [position("SOL-PERP", -1, 200)],
    });

    const sol = findPosition(result, "SOL-PERP");
    expect(sol?.liquidationPriceUsd).toBe(200);
    expect(sol?.liquidationPriceTicks).toBe("2000000");
  });

  it("solves short liquidation prices from portfolio margin and provided marks", () => {
    const calculator = createMarginCalculator([
      market("SOL-PERP", 1, 200, 25),
      market("ETH-PERP", 2, 100, 10),
    ]);
    const result = calculator.computeSubaccountLiquidationPricesFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(250),
      markets: [position("SOL-PERP", -2, 400), position("ETH-PERP", 30, -2960)],
    });

    const sol = findPosition(result, "SOL-PERP");
    expect(sol?.basePositionLots).toBe("-200");
    expect(sol?.basePositionUnits).toBe(-2);
    expect(sol?.entryPriceUsd).toBe(200);
    expect(sol?.liquidationPriceUsd).toBeCloseTo(264.7059, 8);
    expect(sol?.liquidationPriceTicks).toBe("2647059");
  });

  it("includes unsettled funding in the collateral term", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 200, 25)]);
    const withoutFunding =
      calculator.computeSubaccountLiquidationPricesFromInputs({
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(250),
        markets: [position("SOL-PERP", -2, 400)],
      });
    const withFunding = calculator.computeSubaccountLiquidationPricesFromInputs(
      {
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(250),
        markets: [position("SOL-PERP", -2, 400, 100)],
      }
    );

    expect(
      findPosition(withoutFunding, "SOL-PERP")?.liquidationPriceUsd
    ).toBeCloseTo(318.6275, 8);
    expect(
      findPosition(withFunding, "SOL-PERP")?.liquidationPriceUsd
    ).toBeCloseTo(367.6471, 8);
  });

  it("matches the dashboard NVDA liquidation price case", () => {
    expect(
      calculateLiquidationPriceUsd({
        positionSize: -42.21,
        entryPriceUsd: 201.55,
        leverage: 20,
        maintenanceMarginBps: 5000,
        collateralUsd: 700.421673 + 14.05593,
      })
    ).toBeCloseTo(213.14803688872712, 8);

    const calculator = createMarginCalculator([
      {
        ...market("NVDA", 1, 214.18, 20),
        markPriceTicks: "21418",
        tickSize: "10",
        baseLotDecimals: 3,
      },
    ]);
    const subaccount = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 1,
      collateralBalanceQuoteLots: "700421673",
      markets: [
        {
          symbol: "NVDA",
          position: {
            basePositionLots: "-42210",
            virtualQuotePositionLots: "8507693100",
            entryPriceTicks: "20155",
            unsettledFundingQuoteLots: "14055930",
            accumulatedFundingQuoteLots: "12838230",
          },
        },
      ],
    });

    const marketMargin = subaccount.marketMargins[0];
    expect(marketMargin).toBeDefined();

    const liquidationPrice =
      marketMargin === undefined
        ? null
        : computeMarketLiquidationPriceFromMargin(
            marketMargin,
            subaccount,
            calculator.markets.NVDA
          );

    expect(liquidationPrice?.liquidationPriceUsd).toBe(214.18);
    expect(
      liquidationPrice?.liquidationPriceUsd?.toLocaleString("en-US", {
        style: "currency",
        currency: "USD",
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      })
    ).toBe("$214.18");
  });

  it("rejects inconsistent margin totals instead of clamping underflow", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 100, 50)]);
    const margin = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(100),
      markets: [position("SOL-PERP", 1, -100)],
    });

    expect(() =>
      computeSubaccountLiquidationPricesFromMargin(
        {
          ...margin,
          margin: {
            ...margin.margin,
            maintenanceMarginQuoteLots: "0",
          },
        },
        calculator.markets
      )
    ).toThrow(/Portfolio maintenance margin underflow for symbol SOL-PERP/);
  });

  it("uses current-mark limit-order fill-loss sentinels for same-market liquidation", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 100, 50)]);
    const result = calculator.computeSubaccountLiquidationPricesFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(99),
      markets: [
        {
          ...position("SOL-PERP", 1, -100),
          limitOrders: [
            {
              orderSequenceNumber: "1",
              side: "bid",
              priceTicks: priceTicks(100),
              sizeRemainingLots: "100",
              initialSizeLots: "100",
              reduceOnly: false,
            },
          ],
        },
      ],
    });

    const sol = findPosition(result, "SOL-PERP");
    expect(sol?.liquidationPriceUsd).toBeCloseTo(1.0152, 8);
    expect(sol?.liquidationPriceTicks).toBe("10152");
  });

  it("handles negative base-lot decimals when same-market limit orders affect liquidation", () => {
    const calculator = createMarginCalculator([
      {
        ...market("XAU-PERP", 1, 100, 10, "10000"),
        markPriceTicks: "10000",
        tickSize: "1000000",
        baseLotDecimals: -2,
      },
    ]);

    const result = calculator.computeSubaccountLiquidationPricesFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(2_000),
      markets: [
        {
          symbol: "XAU-PERP",
          position: {
            basePositionLots: "1",
            virtualQuotePositionLots: quoteLots(-10_000),
            entryPriceTicks: "10000",
            unsettledFundingQuoteLots: "0",
            accumulatedFundingQuoteLots: "0",
          },
          limitOrders: [
            {
              orderSequenceNumber: "1",
              side: "bid",
              priceTicks: "10000",
              sizeRemainingLots: "1",
              initialSizeLots: "1",
              reduceOnly: false,
            },
          ],
        },
      ],
    });

    const xau = findPosition(result, "XAU-PERP");
    expect(xau?.basePositionUnits).toBe(100);
    expect(xau?.liquidationPriceUsd).toBeCloseTo(88.88, 8);
    expect(xau?.liquidationPriceTicks).toBe("8888");
  });

  it("matches provided HYPE liquidation price with target and other-market orders", () => {
    const calculator = createMarginCalculator([
      snapshotMarket("HYPE", 4, 71.9, "4000000"),
      snapshotMarket("ZEC", 10, 506.16, "1250657"),
    ]);

    const result = calculator.computeSubaccountLiquidationPricesFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "4559664733",
      markets: [
        {
          symbol: "HYPE",
          position: {
            basePositionLots: "12488",
            entryPriceTicks: "71394",
            virtualQuotePositionLots: "-8915750122",
            unsettledFundingQuoteLots: "0",
            accumulatedFundingQuoteLots: "-2312216",
          },
          limitOrders: [
            {
              orderSequenceNumber: "18446744073683662222",
              side: "bid",
              priceTicks: "70400",
              sizeRemainingLots: "1500",
              initialSizeLots: "1500",
              reduceOnly: false,
            },
          ],
        },
        {
          symbol: "ZEC",
          limitOrders: [
            {
              orderSequenceNumber: "18446744073709527386",
              side: "bid",
              priceTicks: "47900",
              sizeRemainingLots: "1035",
              initialSizeLots: "1035",
              reduceOnly: false,
            },
            {
              orderSequenceNumber: "18446744073709527387",
              side: "bid",
              priceTicks: "48700",
              sizeRemainingLots: "1035",
              initialSizeLots: "1035",
              reduceOnly: false,
            },
          ],
        },
      ],
    });

    const hype = findPosition(result, "HYPE");
    expect(hype?.liquidationPriceUsd).toBeCloseTo(41.395, 8);
    expect(hype?.liquidationPriceTicks).toBe("41395");
  });

  it("matches Hawkeye for a ZEC long with passive same-market bids", () => {
    const calculator = createMarginCalculator([
      {
        symbol: "ZEC",
        assetId: 10,
        markPriceTicks: "46100",
        tickSize: "100",
        baseLotDecimals: 2,
        leverageTiers: [
          {
            upperBoundSize: "1250657",
            maxLeverage: "10",
            limitOrderRiskFactorBps: "10000",
          },
        ],
        riskFactors: {
          maintenanceMarginFactorBps: "5000",
          backstopMarginFactorBps: "2000",
          highRiskMarginFactorBps: "1000",
        },
        cancelOrderRiskFactorBps: "7500",
        upnlRiskFactor: "10000",
        upnlRiskFactorForWithdrawals: "100",
        isolatedOnly: false,
      },
      {
        symbol: "ETH",
        assetId: 2,
        markPriceTicks: "17353",
        tickSize: "100",
        baseLotDecimals: 3,
        leverageTiers: [
          {
            upperBoundSize: "12894433",
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
        upnlRiskFactorForWithdrawals: "100",
        isolatedOnly: false,
      },
    ]);

    const result = calculator.computeSubaccountLiquidationPricesFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "4358740700",
      markets: [
        {
          symbol: "ZEC",
          position: {
            basePositionLots: "1500",
            virtualQuotePositionLots: "-6811070900",
            entryPriceTicks: "45407",
            unsettledFundingQuoteLots: "0",
            accumulatedFundingQuoteLots: "16320",
          },
          limitOrders: [
            {
              orderSequenceNumber: "18446744073709527200",
              side: "bid",
              priceTicks: "42322",
              sizeRemainingLots: "5045",
              initialSizeLots: "5045",
              reduceOnly: false,
            },
            {
              orderSequenceNumber: "18446744073709527199",
              side: "bid",
              priceTicks: "44485",
              sizeRemainingLots: "500",
              initialSizeLots: "500",
              reduceOnly: false,
            },
          ],
        },
        {
          symbol: "ETH",
          limitOrders: [
            {
              orderSequenceNumber: "58234622",
              side: "ask",
              priceTicks: "17480",
              sizeRemainingLots: "4500",
              initialSizeLots: "4500",
              reduceOnly: false,
            },
          ],
        },
      ],
    });

    const zec = findPosition(result, "ZEC");
    expect(zec?.liquidationPriceUsd).toBeCloseTo(230.67, 8);
    expect(zec?.liquidationPriceTicks).toBe("23067");
  });

  it("returns null for invalid and non-actionable solver inputs by default", () => {
    expect(
      calculateLiquidationPriceUsd({
        positionSize: 0,
        entryPriceUsd: 100,
        leverage: 50,
        maintenanceMarginBps: 5000,
        collateralUsd: 100,
      })
    ).toBeNull();

    expect(
      calculateLiquidationPriceUsd({
        positionSize: 1,
        entryPriceUsd: 100,
        leverage: 50,
        maintenanceMarginBps: 5000,
        collateralUsd: 200,
      })
    ).toBeNull();
  });

  it("exposes the raw equation root when requested for diagnostics", () => {
    expect(
      calculateLiquidationPriceUsd({
        positionSize: 1,
        entryPriceUsd: 100,
        leverage: 50,
        maintenanceMarginBps: 5000,
        collateralUsd: 200,
        allowNonPositive: true,
      })
    ).toBeCloseTo(-101.010101, 5);
  });

  it("simulates increasing a cross-margin position from an existing entry", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 200, 50)]);
    const subaccount: SubaccountMarginInputs = {
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(100),
      markets: [position("SOL-PERP", 1, -100)],
    };

    const result = calculator.simulatePositionFill({
      subaccount,
      symbol: "SOL-PERP",
      side: "bid",
      baseLots: "100",
      priceTicks: priceTicks(200),
    });
    const standalone = simulatePositionFillFromInputs(
      {
        subaccount,
        symbol: "SOL-PERP",
        side: "bid",
        baseLots: "100",
        priceTicks: priceTicks(200),
      },
      calculator.markets
    );

    expect(standalone).toEqual(result);
    expect(result.marginMode).toBe("cross");
    expect(result.realizedPnlQuoteLots).toBe("0");
    expect(result.projectedPosition).toMatchObject({
      basePositionLots: "200",
      virtualQuotePositionLots: quoteLots(-300),
      entryPriceTicks: priceTicks(150),
    });
    expect(result.margin.margin.collateralBalanceQuoteLots).toBe(
      quoteLots(100)
    );
    expect(result.liquidationPrice?.entryPriceUsd).toBe(150);
    expect(result.liquidationPrice?.liquidationPriceUsd).toBeCloseTo(
      101.010101,
      5
    );
  });

  it("simulates reducing through flat and opening the other side", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 120, 50)]);

    const result = calculator.simulatePositionFill({
      subaccount: {
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(100),
        markets: [position("SOL-PERP", 2, -200)],
      },
      symbol: "SOL-PERP",
      side: "ask",
      baseLots: "300",
      priceTicks: priceTicks(120),
    });

    expect(result.realizedPnlQuoteLots).toBe(quoteLots(40));
    expect(result.margin.margin.collateralBalanceQuoteLots).toBe(
      quoteLots(140)
    );
    expect(result.projectedPosition).toMatchObject({
      basePositionLots: "-100",
      virtualQuotePositionLots: quoteLots(120),
      entryPriceTicks: priceTicks(120),
    });
    expect(result.liquidationPrice?.entryPriceUsd).toBe(120);
    expect(result.liquidationPrice?.liquidationPriceUsd).toBeCloseTo(
      257.4258,
      8
    );
  });

  it("deducts caller-provided fill fees from projected collateral", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 120, 50)]);

    const result = calculator.simulatePositionFill({
      subaccount: {
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(100),
        markets: [position("SOL-PERP", 2, -200)],
      },
      symbol: "SOL-PERP",
      side: "ask",
      baseLots: "300",
      priceTicks: priceTicks(120),
      feeQuoteLots: quoteLots(1),
    });

    expect(result.realizedPnlQuoteLots).toBe(quoteLots(40));
    expect(result.feeQuoteLots).toBe(quoteLots(1));
    expect(result.margin.margin.collateralBalanceQuoteLots).toBe(
      quoteLots(139)
    );
  });

  it("settles current funding into projected collateral when simulating a fill", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 120, 50)]);

    const result = calculator.simulatePositionFill({
      subaccount: {
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(100),
        markets: [position("SOL-PERP", 1, -100, 5)],
      },
      symbol: "SOL-PERP",
      side: "ask",
      baseLots: "100",
      priceTicks: priceTicks(120),
    });

    expect(result.realizedPnlQuoteLots).toBe(quoteLots(20));
    expect(result.projectedPosition).toBeNull();
    expect(result.margin.margin.collateralBalanceQuoteLots).toBe(
      quoteLots(125)
    );
    expect(result.margin.margin.unsettledFundingQuoteLots).toBe("0");
    expect(result.liquidationPrice).toBeNull();
  });

  it("simulates isolated margin without carrying other markets into the result", () => {
    const calculator = createMarginCalculator([
      market("SOL-PERP", 1, 100, 50),
      market("ETH-PERP", 2, 100, 10),
    ]);
    const subaccount: SubaccountMarginInputs = {
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(100),
      markets: [position("SOL-PERP", 1, -100), position("ETH-PERP", 10, -1050)],
    };

    const cross = calculator.simulatePositionFill({
      subaccount,
      symbol: "SOL-PERP",
      side: "bid",
      baseLots: "100",
      priceTicks: priceTicks(100),
      marginMode: "cross",
    });
    const isolated = calculator.simulatePositionFill({
      subaccount,
      symbol: "SOL-PERP",
      side: "bid",
      baseLots: "100",
      priceTicks: priceTicks(100),
      marginMode: "isolated",
      isolatedCollateralBalanceQuoteLots: quoteLots(100),
    });

    expect(
      cross.projectedSubaccountInput.markets.map(({ symbol }) => symbol)
    ).toEqual(["SOL-PERP", "ETH-PERP"]);
    expect(
      isolated.projectedSubaccountInput.markets.map(({ symbol }) => symbol)
    ).toEqual(["SOL-PERP"]);
    expect(isolated.margin.margin.collateralBalanceQuoteLots).toBe(
      quoteLots(100)
    );
    expect(cross.liquidationPrice?.liquidationPriceUsd).toBeGreaterThan(
      isolated.liquidationPrice?.liquidationPriceUsd ?? Number.POSITIVE_INFINITY
    );
    expect(isolated.liquidationPrice?.liquidationPriceUsd).toBeCloseTo(
      50.505,
      8
    );
  });

  it("simulates canceling an order and removing its margin requirement", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 100, 50)]);
    const result = calculator.simulateMargin({
      subaccount: {
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(1_000),
        markets: [
          {
            symbol: "SOL-PERP",
            limitOrders: [
              {
                orderSequenceNumber: "10",
                side: "bid",
                priceTicks: priceTicks(100),
                sizeRemainingLots: "100",
                initialSizeLots: "100",
                reduceOnly: false,
              },
            ],
          },
        ],
      },
      actions: [
        {
          type: "cancelOrder",
          symbol: "SOL-PERP",
          orderSequenceNumber: "10",
        },
      ],
    });

    expect(BigInt(result.before.margin.initialMarginQuoteLots)).toBeGreaterThan(
      0n
    );
    expect(result.after.margin.initialMarginQuoteLots).toBe("0");
    expect(result.after.limitOrders).toHaveLength(0);
    expect(result.actionReports).toContainEqual({
      type: "cancelOrder",
      symbol: "SOL-PERP",
      orderSequenceNumber: "10",
      removed: true,
      settledFundingQuoteLots: "0",
    });
  });

  it("settles funding when simulating order cancellation", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 100, 50)]);
    const result = calculator.simulateMargin({
      subaccount: {
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(100),
        markets: [
          {
            ...position("SOL-PERP", 1, -100),
            limitOrders: [
              {
                orderSequenceNumber: "10",
                side: "bid",
                priceTicks: priceTicks(90),
                sizeRemainingLots: "100",
                initialSizeLots: "100",
                reduceOnly: false,
              },
            ],
            position: {
              ...position("SOL-PERP", 1, -100).position!,
              unsettledFundingQuoteLots: quoteLots(5),
            },
          },
        ],
      },
      actions: [
        {
          type: "cancelOrder",
          symbol: "SOL-PERP",
          orderSequenceNumber: "10",
        },
      ],
    });

    expect(result.after.margin.collateralBalanceQuoteLots).toBe(quoteLots(105));
    expect(result.after.margin.unsettledFundingQuoteLots).toBe("0");
    expect(result.actionReports).toContainEqual({
      type: "cancelOrder",
      symbol: "SOL-PERP",
      orderSequenceNumber: "10",
      removed: true,
      settledFundingQuoteLots: quoteLots(5),
    });
  });

  it("cancels aggregate-only order margin without inventing an order count", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 100, 50)]);
    const result = calculator.simulateMargin({
      subaccount: {
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(1_000),
        markets: [
          {
            symbol: "SOL-PERP",
            limitOrderMargin: {
              numAskOrders: 0,
              numBidOrders: 0,
              lowestAsk: "0",
              highestBid: "0",
              totalNonReduceOnlyBidBaseLots: "100",
              totalNonReduceOnlyAskBaseLots: "0",
              totalReduceOnlyBidBaseLots: "0",
              totalReduceOnlyAskBaseLots: "0",
            },
          },
        ],
      },
      actions: [{ type: "cancelAllOrders", symbol: "SOL-PERP" }],
    });

    expect(BigInt(result.before.margin.initialMarginQuoteLots)).toBeGreaterThan(
      0n
    );
    expect(result.after.margin.initialMarginQuoteLots).toBe("0");
    expect(result.actionReports).toContainEqual({
      type: "cancelAllOrders",
      symbol: "SOL-PERP",
      removed: null,
      settledFundingQuoteLots: "0",
    });
  });

  it("simulates adding reduce-only and non-reduce-only limit orders", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 100, 50)]);
    const subaccount: SubaccountMarginInputs = {
      subaccountIndex: 0,
      collateralBalanceQuoteLots: quoteLots(1_000),
      markets: [{ symbol: "SOL-PERP" }],
    };

    const reduceOnly = calculator.simulateMargin({
      subaccount,
      actions: [
        {
          type: "placeLimitOrder",
          symbol: "SOL-PERP",
          orderSequenceNumber: "11",
          side: "ask",
          priceTicks: priceTicks(100),
          sizeLots: "100",
          reduceOnly: true,
        },
      ],
    });
    const nonReduceOnly = calculator.simulateMargin({
      subaccount,
      actions: [
        {
          type: "placeLimitOrder",
          symbol: "SOL-PERP",
          orderSequenceNumber: "12",
          side: "bid",
          priceTicks: priceTicks(100),
          sizeLots: "100",
        },
      ],
    });

    expect(reduceOnly.after.margin.initialMarginQuoteLots).toBe("0");
    expect(nonReduceOnly.after.margin.initialMarginQuoteLots).not.toBe("0");
  });

  it("simulates mark moves, estimated funding, and collateral changes", () => {
    const calculator = createMarginCalculator([market("SOL-PERP", 1, 100, 50)]);
    const result = calculator.simulateMargin({
      subaccount: {
        subaccountIndex: 0,
        collateralBalanceQuoteLots: quoteLots(100),
        markets: [position("SOL-PERP", 1, -100)],
      },
      actions: [
        {
          type: "setMarkPrice",
          symbol: "SOL-PERP",
          markPriceTicks: priceTicks(120),
        },
        {
          type: "applyFundingPayment",
          symbol: "SOL-PERP",
          quoteLots: quoteLots(5),
        },
        {
          type: "settleFunding",
          symbol: "SOL-PERP",
        },
        {
          type: "adjustCollateral",
          quoteLotsDelta: quoteLots(-10),
        },
      ],
    });

    expect(result.after.margin.collateralBalanceQuoteLots).toBe(quoteLots(95));
    expect(result.after.margin.unsettledFundingQuoteLots).toBe("0");
    expect(result.after.marketMargins[0]?.unrealizedPnlQuoteLots).toBe(
      quoteLots(20)
    );
    expect(result.delta.collateralBalanceQuoteLots).toBe(quoteLots(-5));
  });
});
