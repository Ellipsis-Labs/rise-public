import { describe, expect, it } from "vitest";
import {
  buildSubaccountMarginInputsFromSnapshot,
  computeSubaccountProjectedLiquidationFromMargin,
  createMarginCalculator,
  type MarketParams,
} from "@/margin";

// SOL market: tickSize 10, baseLotDecimals 3, mark 5000 ticks. One base lot =
// 1e6 lamports and prices at 5000 * 10 = 50_000 quote lots, so 1 SOL
// (1e9 lamports = 1000 base lots) = 50_000_000 quote lots ($50).
const solMarketParams: MarketParams = {
  symbol: "SOL",
  assetId: 1,
  markPriceTicks: "5000",
  tickSize: "10",
  baseLotDecimals: 3,
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

// Discount curve: 5% at zero balance -> 20% at the 10 SOL global cap
// (retention 9500 -> 8000 bps).
const solSpotInput = {
  assetIndex: 4294901760,
  symbol: "SOL",
  balance: "2000000000", // 2 SOL
  decimals: 9,
  maxGlobalBalance: "10000000000", // 10 SOL
  minMarginDiscountBps: 500,
  maxMarginDiscountBps: 2000,
};

describe("margin spot collateral valuation", () => {
  const calculator = createMarginCalculator([solMarketParams]);

  it("adds discounted spot to effective collateral and notional to portfolio value", () => {
    const margin = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "1000000",
      markets: [],
      spotCollaterals: [solSpotInput],
    });

    // notional: 2000 base lots * 50_000 = 100_000_000.
    // retention at 2/10 of the cap: 9500 - 1500 * 2/10 = 9200 bps.
    expect(margin.margin.spotCollateralNotionalQuoteLots).toBe("100000000");
    expect(margin.margin.spotCollateralDiscountedQuoteLots).toBe("92000000");
    expect(margin.margin.portfolioValueQuoteLots).toBe("101000000");
    expect(margin.margin.effectiveCollateralQuoteLots).toBe("93000000");
    // Spot never backs quote withdrawals.
    expect(margin.margin.effectiveCollateralForWithdrawalsQuoteLots).toBe(
      "1000000"
    );
    expect(margin.spotCollaterals).toEqual([
      {
        assetIndex: 4294901760,
        symbol: "SOL",
        pricingMarketSymbol: "SOL",
        balance: "2000000000",
        nativeUnitsPerBaseLot: "1000000",
        retainedBps: "9200",
        notionalQuoteLots: "100000000",
        discountedQuoteLots: "92000000",
      },
    ]);
  });

  it("values sub-base-lot dust with truncating division", () => {
    const margin = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "0",
      markets: [],
      spotCollaterals: [
        { ...solSpotInput, balance: "1500000500" }, // 1.5 SOL + 500 lamports
      ],
    });

    // 1500 base lots * 50_000 + floor(500 * 50_000 / 1e6) = 75_000_000 + 25.
    expect(margin.margin.spotCollateralNotionalQuoteLots).toBe("75000025");
  });

  it("uses the curve endpoints at zero balance and at the global cap", () => {
    const atCap = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "0",
      markets: [],
      spotCollaterals: [{ ...solSpotInput, balance: "10000000000" }],
    });
    // 10 SOL notional 500_000_000, retention at the cap = 8000 bps.
    expect(atCap.margin.spotCollateralDiscountedQuoteLots).toBe("400000000");

    const empty = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "0",
      markets: [],
      spotCollaterals: [{ ...solSpotInput, balance: "0" }],
    });
    expect(empty.margin.spotCollateralNotionalQuoteLots).toBe("0");
    expect(empty.margin.spotCollateralDiscountedQuoteLots).toBe("0");
  });

  it("prefers an explicit index price over the market mark", () => {
    const margin = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "0",
      markets: [],
      spotCollaterals: [{ ...solSpotInput, indexPriceTicks: "6000" }],
    });
    // 2000 base lots * 6000 * 10 = 120_000_000.
    expect(margin.margin.spotCollateralNotionalQuoteLots).toBe("120000000");
  });

  it("omits the spot fields when no spot collaterals are supplied", () => {
    const margin = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "1000000",
      markets: [],
    });
    expect(margin.margin.spotCollateralNotionalQuoteLots).toBeUndefined();
    expect(margin.margin.spotCollateralDiscountedQuoteLots).toBeUndefined();
    expect(margin.spotCollaterals).toBeUndefined();
    expect(margin.margin.effectiveCollateralQuoteLots).toBe("1000000");
  });

  it("builds spot inputs from a traderState snapshot with asset params", () => {
    const inputs = buildSubaccountMarginInputsFromSnapshot(
      {
        subaccountIndex: 0,
        collateral: "1000000",
        spotCollaterals: [
          { assetIndex: 4294901760, symbol: "SOL", balance: "2000000000" },
          // No params registered for this asset -> left unvalued.
          { assetIndex: 4294901761, symbol: "XYZ", balance: "5" },
        ],
      },
      {
        spotCollateralParamsByIndex: {
          4294901760: {
            assetIndex: 4294901760,
            decimals: 9,
            maxGlobalBalance: "10000000000",
            minMarginDiscountBps: 500,
            maxMarginDiscountBps: 2000,
          },
        },
      }
    );

    expect(inputs.spotCollaterals).toHaveLength(1);
    const margin = calculator.computeSubaccountMarginFromInputs(inputs);
    expect(margin.margin.effectiveCollateralQuoteLots).toBe("93000000");
  });

  it("reprices spot collateral along the liquidation price path", () => {
    const liquidationCalculator = createMarginCalculator(
      [
        {
          ...solMarketParams,
          riskFactors: {
            ...solMarketParams.riskFactors,
            maintenanceMarginFactorBps: "5000",
          },
        },
      ],
      [
        {
          assetIndex: 4294901760,
          symbol: "SOL",
          perpSymbol: "SOL",
          decimals: 9,
          maxPerTraderBalance: 10_000_000_000n,
          maxGlobalBalance: 10_000_000_000n,
          currGlobalBalance: 2_000_000_000n,
          minMarginDiscountBps: 500,
          maxMarginDiscountBps: 2000,
        },
      ]
    );
    const inputs = {
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "-20000000",
      nativeSolCollateralLamports: "2000000000",
      markets: [
        {
          symbol: "SOL",
          position: {
            basePositionLots: "1000",
            virtualQuotePositionLots: "-50000000",
            entryPriceTicks: "5000",
            unsettledFundingQuoteLots: "0",
            accumulatedFundingQuoteLots: "0",
          },
        },
      ],
    };
    const result =
      liquidationCalculator.computeSubaccountLiquidationPricesFromInputs(
        inputs
      );

    const liquidationTicks = Number(result.positions[0]?.liquidationPriceTicks);
    // Revaluing the two SOL of collateral at each candidate price produces a
    // lower boundary near $25 rather than treating its current $92 value as
    // fixed throughout the search.
    expect(liquidationTicks).toBeGreaterThan(2500);
    expect(liquidationTicks).toBeLessThan(2600);

    const margin =
      liquidationCalculator.computeSubaccountMarginFromInputs(inputs);
    const projected = computeSubaccountProjectedLiquidationFromMargin(
      margin,
      liquidationCalculator.markets
    );
    expect(projected.positions[0]?.staticLiquidationPriceTicks).toBe(
      result.positions[0]?.liquidationPriceTicks
    );
    expect(projected.positions[0]?.liquidationPriceTicks).toBe(
      result.positions[0]?.liquidationPriceTicks
    );
  });
});
