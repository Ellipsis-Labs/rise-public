import { describe, expect, it } from "vitest";
import {
  buildSubaccountMarginInputsFromSnapshot,
  createMarginCalculator,
  type MarketParams,
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
      maxLeverage: "10",
      limitOrderRiskFactorBps: "0",
    },
  ],
  riskFactors: {
    maintenanceMarginFactorBps: "500",
    backstopMarginFactorBps: "1000",
    highRiskMarginFactorBps: "2000",
  },
  cancelOrderRiskFactorBps: "500",
  upnlRiskFactor: "10000",
  upnlRiskFactorForWithdrawals: "10000",
  isolatedOnly: false,
};

describe("margin funding sign handling", () => {
  it("keeps positive snapshot unsettled funding as effective collateral", () => {
    const inputs = buildSubaccountMarginInputsFromSnapshot({
      subaccountIndex: 0,
      collateral: "1000",
      positions: [
        {
          symbol: "SOL-PERP",
          basePositionLots: "1",
          virtualQuotePositionLots: "-100",
          entryPriceTicks: "100",
          unsettledFundingQuoteLots: "25",
          accumulatedFundingQuoteLots: "5",
        },
      ],
    });

    expect(inputs.markets[0]?.position?.unsettledFundingQuoteLots).toBe("25");

    const margin = createMarginCalculator([
      marketParams,
    ]).computeSubaccountMarginFromInputs(inputs);

    expect(margin.marketMargins[0]?.unsettledFundingQuoteLots).toBe("25");
    expect(margin.marketMargins[0]?.accumulatedFundingQuoteLots).toBe("5");
    expect(margin.margin.unsettledFundingQuoteLots).toBe("25");
    expect(margin.margin.accumulatedFundingQuoteLots).toBe("5");
    expect(margin.margin.effectiveCollateralQuoteLots).toBe("1025");
    expect(margin.margin.portfolioValueQuoteLots).toBe("1025");
  });
});
