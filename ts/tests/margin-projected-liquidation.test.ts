import { describe, expect, it } from "vitest";
import {
  aggregateLimitOrderStateFromOrders,
  computeProjectedLiquidation,
  createProjectedLiquidationCalculator,
  type NormalizedMarketParams,
  type ProjectedLiquidationInput,
  type ProjectedLiquidationOrderInput,
} from "@/margin";

/**
 * ZEC market parameters and account state from
 * `tools/margin-math-parity/fixtures/diff/snapshot_1783529040.wincode.zst`
 * for authority `4nHxBXGUdoTG262JXfyH56G7vFaQgAPrauLfHQ8zTu2k`; see
 * `tools/margin-math-parity/docs/liquidation-price-calculation.md`.
 */
const zecMarketParams = (markPriceTicks: bigint): NormalizedMarketParams => ({
  symbol: "ZEC",
  assetId: 10,
  markPriceTicks,
  tickSize: 100n,
  baseLotDecimals: 2,
  leverageTiers: [
    {
      upperBoundSize: 1_250_657n,
      maxLeverage: 10n,
      limitOrderRiskFactorBps: 10_000n,
    },
    {
      upperBoundSize: 1_250_658n,
      maxLeverage: 1n,
      limitOrderRiskFactorBps: 10_000n,
    },
    {
      upperBoundSize: 1_250_659n,
      maxLeverage: 1n,
      limitOrderRiskFactorBps: 10_000n,
    },
    {
      upperBoundSize: 1_250_660n,
      maxLeverage: 1n,
      limitOrderRiskFactorBps: 10_000n,
    },
  ],
  riskFactors: {
    maintenanceMarginFactorBps: 5_000n,
    backstopMarginFactorBps: 2_000n,
    highRiskMarginFactorBps: 1_000n,
  },
  cancelOrderRiskFactorBps: 7_500n,
  upnlRiskFactor: 10_000n,
  upnlRiskFactorForWithdrawals: 100n,
  isolatedOnly: false,
});

const zecVisibleOrders: ProjectedLiquidationOrderInput[] = [
  {
    side: "bid",
    priceTicks: 44_485n,
    baseLotsRemaining: 500n,
    reduceOnly: false,
  },
  {
    side: "bid",
    priceTicks: 42_322n,
    baseLotsRemaining: 5_045n,
    reduceOnly: false,
  },
];

const zecInput = (markPriceTicks: bigint): ProjectedLiquidationInput => ({
  basePositionLots: 1_500n,
  virtualQuotePositionLots: -6_811_070_900n,
  limitOrderState: aggregateLimitOrderStateFromOrders(
    zecVisibleOrders,
    1_500n,
    markPriceTicks
  ),
  visibleOrders: zecVisibleOrders,
  collateralBalanceQuoteLots: 4_358_740_700n,
  portfolioUnsettledFundingQuoteLots: 0n,
  portfolioDiscountedUnrealizedPnlQuoteLots: 103_929_100n,
  portfolioMaintenanceMarginQuoteLots: 1_819_093_750n,
  targetDiscountedUnrealizedPnlQuoteLots: 103_929_100n,
  targetMaintenanceMarginQuoteLots: 1_623_872_500n,
});

describe("computeProjectedLiquidation", () => {
  it("fills passive bids before solving the boundary for the ZEC snapshot account", () => {
    const result = computeProjectedLiquidation(
      zecInput(46_100n),
      zecMarketParams(46_100n)
    );

    expect(result.fills).toEqual([
      { side: "bid", priceTicks: 44_485n, baseLots: 500n },
      { side: "bid", priceTicks: 42_322n, baseLots: 5_045n },
    ]);
    expect(result.staticLiquidationPriceTicks).toBe(23_067n);
    expect(result.liquidationPriceTicks).toBe(39_181n);
  });

  it("matches the static boundary when no position-side orders rest", () => {
    const input: ProjectedLiquidationInput = {
      ...zecInput(46_100n),
      limitOrderState: aggregateLimitOrderStateFromOrders([], 1_500n, 46_100n),
      visibleOrders: [],
      portfolioMaintenanceMarginQuoteLots: 540_971_250n,
      targetMaintenanceMarginQuoteLots: 345_750_000n,
    };
    const result = computeProjectedLiquidation(input, zecMarketParams(46_100n));

    expect(result.fills).toEqual([]);
    expect(result.liquidationPriceTicks).toBe(
      result.staticLiquidationPriceTicks
    );
    expect(result.liquidationPriceTicks).not.toBeNull();
  });

  it("reports no liquidation price when the account can never be liquidated", () => {
    const input: ProjectedLiquidationInput = {
      ...zecInput(46_100n),
      collateralBalanceQuoteLots: 1_000_000_000_000_000n,
    };
    const result = computeProjectedLiquidation(input, zecMarketParams(46_100n));

    expect(result.liquidationPriceTicks).toBeNull();
    expect(result.staticLiquidationPriceTicks).toBeNull();
    expect(result.fills).toHaveLength(2);
  });

  it("returns the mark when the account is already liquidatable", () => {
    const result = computeProjectedLiquidation(
      zecInput(23_000n),
      zecMarketParams(23_000n)
    );

    expect(result.liquidationPriceTicks).toBe(23_000n);
    expect(result.staticLiquidationPriceTicks).toBe(23_000n);
    expect(result.fills).toEqual([]);
  });
});

describe("createProjectedLiquidationCalculator", () => {
  it("reuses the cached result across target-market mark moves", () => {
    const compute = createProjectedLiquidationCalculator();

    const first = compute(zecInput(46_100n), zecMarketParams(46_100n));
    // A small mark move changes neither the fill ladder nor the boundary;
    // the cached result object is returned as-is.
    const second = compute(zecInput(45_900n), zecMarketParams(45_900n));
    expect(second).toBe(first);
    expect(second.liquidationPriceTicks).toBe(39_181n);
  });

  it("recomputes when the mark crosses the cached static boundary", () => {
    const compute = createProjectedLiquidationCalculator();

    const first = compute(zecInput(46_100n), zecMarketParams(46_100n));
    expect(first.staticLiquidationPriceTicks).toBe(23_067n);

    // At a mark at or below the static boundary the account is already
    // liquidatable, so the cached result no longer applies.
    const crossed = compute(zecInput(23_067n), zecMarketParams(23_067n));
    expect(crossed).not.toBe(first);
    expect(crossed.liquidationPriceTicks).toBe(23_067n);
  });

  it("recomputes when account state changes", () => {
    const compute = createProjectedLiquidationCalculator();

    const first = compute(zecInput(46_100n), zecMarketParams(46_100n));
    const withMoreCollateral = compute(
      {
        ...zecInput(46_100n),
        collateralBalanceQuoteLots: 5_000_000_000n,
      },
      zecMarketParams(46_100n)
    );
    expect(withMoreCollateral).not.toBe(first);
  });
});
