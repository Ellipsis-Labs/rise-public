import { describe, expect, it } from "vitest";
import { buildMarketParamsFromSummary, createMarginCalculator } from "@/margin";
import { symbol } from "@/primitives/Symbol";
import type { MarketSummary } from "@/types";

const tokenAmount = { value: 0, decimals: 0, ui: "0" };

const marketSummary = (
  overrides: Partial<MarketSummary> = {}
): MarketSummary => ({
  symbol: symbol("SOL-PERP"),
  assetId: 1,
  marketStatus: "active",
  units: {
    tickSizeInQuoteLotsPerBaseLot: 1,
    baseLotsDecimals: 0,
  },
  fees: {
    takerFeeMicro: 0,
    makerFeeMicro: 0,
  },
  openInterest: tokenAmount,
  openInterestCap: tokenAmount,
  leverageTiers: [
    {
      maxLeverage: 20,
      maxSizeBaseLots: 1_000,
      limitOrderRiskFactor: 6_000,
    },
  ],
  fundingIntervalInSlots: 0,
  fundingPeriodInSlots: 0,
  fundingStartIntervalSlot: 0,
  cumulativeFundingRate: 0,
  maxLiquidationSize: tokenAmount,
  riskFactors: {
    maintenance: 5_000,
    backstop: 2_000,
    highRisk: 1_000,
    upnl: 10_000,
    upnlForWithdrawals: 100,
    cancelOrder: 7_000,
  },
  isolatedOnly: false,
  ...overrides,
});

describe("margin risk factor bps handling", () => {
  it("preserves public market-summary bps risk factors for margin math", () => {
    const params = buildMarketParamsFromSummary(marketSummary(), "0.00242");

    expect(params).not.toBeNull();
    expect(params?.leverageTiers[0]?.limitOrderRiskFactorBps).toBe("6000");
    expect(params?.riskFactors.maintenanceMarginFactorBps).toBe("5000");
    expect(params?.riskFactors.backstopMarginFactorBps).toBe("2000");
    expect(params?.riskFactors.highRiskMarginFactorBps).toBe("1000");
    expect(params?.upnlRiskFactor).toBe("10000");
    expect(params?.upnlRiskFactorForWithdrawals).toBe("100");

    const calculator = createMarginCalculator([params!]);
    const result = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "1",
      markets: [
        {
          symbol: "SOL-PERP",
          position: {
            basePositionLots: "1",
            virtualQuotePositionLots: "-2420",
            entryPriceTicks: "2420",
            unsettledFundingQuoteLots: "0",
            accumulatedFundingQuoteLots: "0",
          },
        },
      ],
    });

    expect(result.margin.initialMarginQuoteLots).toBe("121");
    expect(result.margin.maintenanceMarginQuoteLots).toBe("60");
  });

  it("preserves already-bps risk factors above the percentage range", () => {
    const params = buildMarketParamsFromSummary(
      marketSummary({
        leverageTiers: [
          {
            maxLeverage: 20,
            maxSizeBaseLots: 1_000,
            limitOrderRiskFactor: 6_000,
          },
        ],
        riskFactors: {
          maintenance: 5_000,
          backstop: 2_000,
          highRisk: 1_000,
          upnl: 10_000,
          upnlForWithdrawals: 3_000,
          cancelOrder: 7_000,
        },
      }),
      "0.00242"
    );

    expect(params?.leverageTiers[0]?.limitOrderRiskFactorBps).toBe("6000");
    expect(params?.riskFactors.maintenanceMarginFactorBps).toBe("5000");
    expect(params?.riskFactors.backstopMarginFactorBps).toBe("2000");
    expect(params?.riskFactors.highRiskMarginFactorBps).toBe("1000");
    expect(params?.upnlRiskFactor).toBe("10000");
    expect(params?.upnlRiskFactorForWithdrawals).toBe("3000");
    expect(params?.cancelOrderRiskFactorBps).toBe("7000");
  });
});
