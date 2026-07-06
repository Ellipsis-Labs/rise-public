import { describe, expect, it } from "vitest";
import {
  createMarginCalculator,
  simulateMarginFromInputs,
  simulateMarginScenariosFromInputs,
  type MarginSimulationActionReport,
  type MarketParams,
  type SubaccountMarginInputs,
} from "@/margin";

const MICRO_USD = 1_000_000;
const BASE_LOTS_PER_UNIT = 100;

const quoteLots = (usd: number): string =>
  Math.round(usd * MICRO_USD).toString();

const priceTicks = (priceUsd: number): string =>
  Math.round((priceUsd * MICRO_USD) / BASE_LOTS_PER_UNIT).toString();

const baseLots = (units: number): string =>
  Math.round(units * BASE_LOTS_PER_UNIT).toString();

const market = (
  symbol: string,
  assetId: number,
  markPriceUsd: number,
  maxLeverage: number
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
      limitOrderRiskFactorBps: "5000",
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

const longSubaccount = (): SubaccountMarginInputs => ({
  subaccountIndex: 0,
  collateralBalanceQuoteLots: quoteLots(1_000),
  markets: [
    {
      symbol: "SOL-PERP",
      position: {
        basePositionLots: baseLots(1),
        virtualQuotePositionLots: quoteLots(-90),
        entryPriceTicks: priceTicks(90),
        unsettledFundingQuoteLots: "0",
        accumulatedFundingQuoteLots: "0",
      },
    },
  ],
});

const calculator = createMarginCalculator([market("SOL-PERP", 1, 100, 10)]);

const findReport = <Type extends MarginSimulationActionReport["type"]>(
  reports: MarginSimulationActionReport[],
  type: Type
): Extract<MarginSimulationActionReport, { type: Type }> => {
  const report = reports.find(
    (
      candidate
    ): candidate is Extract<MarginSimulationActionReport, { type: Type }> =>
      candidate.type === type
  );
  if (!report) {
    throw new Error(`Missing ${type} report`);
  }
  return report;
};

describe("margin simulation actions", () => {
  it("closes the full position at the scenario mark price by default", () => {
    const result = calculator.simulateMargin({
      subaccount: longSubaccount(),
      actions: [{ type: "closePosition", symbol: "SOL-PERP" }],
    });

    const report = findReport(result.actionReports, "closePosition");
    expect(report.closedBaseLots).toBe(baseLots(1));
    expect(report.priceTicks).toBe(priceTicks(100));
    // Long 1 unit from $90 closed at the $100 mark realizes +$10.
    expect(report.realizedPnlQuoteLots).toBe(quoteLots(10));
    expect(report.projectedPosition).toBeNull();
    expect(result.after.margin.collateralBalanceQuoteLots).toBe(
      quoteLots(1_010)
    );
    expect(result.after.margin.initialMarginQuoteLots).toBe("0");
  });

  it("closes a fraction at an explicit price and deducts the fee", () => {
    const result = calculator.simulateMargin({
      subaccount: longSubaccount(),
      actions: [
        {
          type: "closePosition",
          symbol: "SOL-PERP",
          fractionBps: "5000",
          priceTicks: priceTicks(80),
          feeQuoteLots: quoteLots(1),
        },
      ],
    });

    const report = findReport(result.actionReports, "closePosition");
    expect(report.closedBaseLots).toBe(baseLots(0.5));
    // Closing half the long at $80 realizes -$5; the $1 fee also leaves
    // collateral.
    expect(report.realizedPnlQuoteLots).toBe(quoteLots(-5));
    expect(report.feeQuoteLots).toBe(quoteLots(1));
    expect(result.after.margin.collateralBalanceQuoteLots).toBe(quoteLots(994));
    expect(report.projectedPosition?.basePositionLots).toBe(baseLots(0.5));
    expect(report.projectedPosition?.virtualQuotePositionLots).toBe(
      quoteLots(-45)
    );
  });

  it("rejects closing a position that does not exist", () => {
    expect(() =>
      calculator.simulateMargin({
        subaccount: {
          subaccountIndex: 0,
          collateralBalanceQuoteLots: quoteLots(1_000),
          markets: [],
        },
        actions: [{ type: "closePosition", symbol: "SOL-PERP" }],
      })
    ).toThrow(/without an open position/);
  });

  it("moves the mark price by a signed basis-point delta", () => {
    const result = calculator.simulateMargin({
      subaccount: longSubaccount(),
      actions: [
        { type: "moveMarkPrice", symbol: "SOL-PERP", percentDeltaBps: "1000" },
      ],
    });

    const report = findReport(result.actionReports, "moveMarkPrice");
    expect(report.previousMarkPriceTicks).toBe(priceTicks(100));
    expect(report.newMarkPriceTicks).toBe(priceTicks(110));
    // uPnL moves from mark 100 (+$10) to mark 110 (+$20).
    expect(result.before.margin.unrealizedPnlQuoteLots).toBe(quoteLots(10));
    expect(result.after.margin.unrealizedPnlQuoteLots).toBe(quoteLots(20));
  });

  it("rejects mark moves that do not produce a positive price", () => {
    expect(() =>
      calculator.simulateMargin({
        subaccount: longSubaccount(),
        actions: [
          {
            type: "moveMarkPrice",
            symbol: "SOL-PERP",
            percentDeltaBps: "-10000",
          },
        ],
      })
    ).toThrow(/positive in-range tick price/);
  });

  it("applies mark price overrides to before and after margins", () => {
    const result = simulateMarginFromInputs(
      {
        subaccount: longSubaccount(),
        markPriceOverrides: { "SOL-PERP": priceTicks(120) },
        actions: [],
      },
      calculator.markets
    );

    expect(result.before.margin.unrealizedPnlQuoteLots).toBe(quoteLots(30));
    expect(result.after.margin.unrealizedPnlQuoteLots).toBe(quoteLots(30));
    // The caller's market map must stay untouched.
    expect(calculator.markets["SOL-PERP"]?.markPriceTicks.toString()).toBe(
      priceTicks(100)
    );
  });
});

describe("margin simulation scenarios", () => {
  it("branches independent scenarios from one shared baseline", () => {
    const result = simulateMarginScenariosFromInputs(
      {
        subaccount: longSubaccount(),
        baseActions: [
          { type: "adjustCollateral", quoteLotsDelta: quoteLots(500) },
        ],
        scenarios: [
          {
            label: "close everything",
            actions: [{ type: "closePosition", symbol: "SOL-PERP" }],
          },
          {
            label: "price drops 20%",
            actions: [
              {
                type: "moveMarkPrice",
                symbol: "SOL-PERP",
                percentDeltaBps: "-2000",
              },
            ],
          },
        ],
      },
      calculator.markets
    );

    expect(result.before.margin.collateralBalanceQuoteLots).toBe(
      quoteLots(1_000)
    );
    expect(result.base.margin.collateralBalanceQuoteLots).toBe(
      quoteLots(1_500)
    );
    expect(result.baseActionReports).toHaveLength(1);
    expect(result.scenarios).toHaveLength(2);

    const [closeScenario, drawdownScenario] = result.scenarios;
    // Every scenario starts from the shared post-base state.
    expect(closeScenario!.simulation.before).toEqual(result.base);
    expect(drawdownScenario!.simulation.before).toEqual(result.base);

    // Scenario one closes the position at the $100 mark.
    expect(
      closeScenario!.simulation.after.margin.collateralBalanceQuoteLots
    ).toBe(quoteLots(1_510));
    expect(closeScenario!.simulation.after.margin.initialMarginQuoteLots).toBe(
      "0"
    );
    expect(closeScenario!.simulation.liquidationPrices.positions).toHaveLength(
      0
    );

    // Scenario two still holds the position; the close in scenario one must
    // not leak into it.
    expect(
      drawdownScenario!.simulation.after.margin.collateralBalanceQuoteLots
    ).toBe(quoteLots(1_500));
    expect(
      drawdownScenario!.simulation.after.margin.unrealizedPnlQuoteLots
    ).toBe(quoteLots(-10));
    expect(
      drawdownScenario!.simulation.liquidationPrices.positions
    ).toHaveLength(1);
  });

  it("shares mark price overrides across the whole batch", () => {
    const result = simulateMarginScenariosFromInputs(
      {
        subaccount: longSubaccount(),
        markPriceOverrides: { "SOL-PERP": priceTicks(120) },
        scenarios: [
          {
            label: "close at override mark",
            actions: [{ type: "closePosition", symbol: "SOL-PERP" }],
          },
        ],
      },
      calculator.markets
    );

    expect(result.before.margin.unrealizedPnlQuoteLots).toBe(quoteLots(30));
    const report = findReport(
      result.scenarios[0]!.simulation.actionReports,
      "closePosition"
    );
    expect(report.priceTicks).toBe(priceTicks(120));
    expect(
      result.scenarios[0]!.simulation.after.margin.collateralBalanceQuoteLots
    ).toBe(quoteLots(1_030));
  });
});
