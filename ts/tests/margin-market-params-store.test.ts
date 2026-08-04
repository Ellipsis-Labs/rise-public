import { describe, expect, it } from "vitest";
import type { MarketResponse } from "@/api/markets/types";
import { MarginMarketParamsStore, priceUsdToTicks } from "@/margin";
import { symbol } from "@/primitives/Symbol";
import type { MarketSummary } from "@/types";

const tokenAmount = { value: 0, decimals: 0, ui: "0" };
const market: MarketSummary = {
  symbol: symbol("SOL-PERP"),
  assetId: 1,
  marketStatus: "active",
  units: {
    tickSizeInQuoteLotsPerBaseLot: 1,
    baseLotsDecimals: 0,
  },
  fees: { takerFeeMicro: 0, makerFeeMicro: 0 },
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
};

describe("margin market params store", () => {
  it("converts sub-micro USD prices without truncating precision", () => {
    expect(
      priceUsdToTicks("0.000009706", {
        baseLotsDecimals: -4,
        tickSizeInQuoteLotsPerBaseLot: 1,
      })
    ).toBe("97060");
    expect(
      priceUsdToTicks("0.0000005", {
        baseLotsDecimals: 0,
        tickSizeInQuoteLotsPerBaseLot: 1,
      })
    ).toBe("1");
  });

  it("discovers index prices and active spot collateral from public APIs", async () => {
    let getMarketCalls = 0;
    const client = {
      markets: {
        async getMarkets() {
          return { slot: 10, markets: [market] };
        },
        async getMarket() {
          getMarketCalls += 1;
          return {
            slot: 10,
            market: {
              markPrice: { price: 100, slot: 10 },
              spotPrice: { price: 90, slot: 10 },
            },
          } as unknown as MarketResponse;
        },
      },
      collateral: {
        async getAssets() {
          return {
            assets: [
              {
                assetIndex: 0xffff0000,
                symbol: "SOL",
                decimals: 9,
                spot: {
                  isActive: true,
                  perpAssetIndex: 1,
                  maxPerTraderBalance: 1_000_000_000_000n,
                  maxGlobalBalance: 10_000_000_000_000n,
                  currGlobalBalance: 1_000_000_000n,
                  minMarginDiscountBps: 500,
                  maxMarginDiscountBps: 1_000,
                  maxLiquidationDiscountBps: 1_000,
                  minLiquidationSlippageBps: 100,
                  maxLiquidationSize: 1_000_000_000n,
                },
              },
            ],
          };
        },
      },
    };
    const store = new MarginMarketParamsStore({ client });

    const snapshot = await store.getSnapshot();
    expect(snapshot.paramsBySymbol["SOL-PERP"]?.markPriceTicks).toBe(
      "100000000"
    );
    expect(snapshot.paramsBySymbol["SOL-PERP"]?.indexPriceTicks).toBe(
      "90000000"
    );
    expect(getMarketCalls).toBe(1);
    expect(snapshot.spotCollaterals).toEqual([
      expect.objectContaining({
        assetIndex: 0xffff0000,
        symbol: "SOL",
        perpSymbol: "SOL-PERP",
      }),
    ]);

    const calculator = await store.getCalculator();
    const margin = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "0",
      nativeSolCollateralLamports: "1000000000",
      markets: [],
    });
    expect(
      BigInt(margin.margin.spotCollateralDiscountedQuoteLots ?? "0")
    ).toBeGreaterThan(0n);
  });

  it("degrades to zero spot collateral when the assets endpoint is unavailable", async () => {
    // Servers that predate /v1/collateral/assets must not take the whole
    // market-params refresh down: mark and index prices still flow, and spot
    // collateral is valued at zero (fail-open to under-valuation).
    const client = {
      markets: {
        async getMarkets() {
          return { slot: 10, markets: [market] };
        },
        async getMarket() {
          return {
            slot: 10,
            market: {
              markPrice: { price: 100, slot: 10 },
              spotPrice: { price: 90, slot: 10 },
            },
          } as unknown as MarketResponse;
        },
      },
      collateral: {
        async getAssets(): Promise<never> {
          throw new Error("404 Not Found");
        },
      },
    };
    const store = new MarginMarketParamsStore({ client });

    const snapshot = await store.getSnapshot();
    expect(snapshot.paramsBySymbol["SOL-PERP"]?.markPriceTicks).toBe(
      "100000000"
    );
    expect(snapshot.paramsBySymbol["SOL-PERP"]?.indexPriceTicks).toBe(
      "90000000"
    );
    expect(snapshot.spotCollaterals).toEqual([]);

    const calculator = await store.getCalculator();
    const margin = calculator.computeSubaccountMarginFromInputs({
      subaccountIndex: 0,
      collateralBalanceQuoteLots: "0",
      nativeSolCollateralLamports: "1000000000",
      markets: [],
    });
    expect(margin.margin.spotCollateralDiscountedQuoteLots).toBeUndefined();
    expect(margin.spotCollaterals).toBeUndefined();
  });
});
