import { describe, expect, it } from "vitest";
import type { MarketResponse } from "@/api/markets/types";
import {
  MarginMarketParamsStore,
  priceUsdToTicks,
  buildMarketParamsBySymbol,
  buildMarketParamsFromSummary,
} from "@/margin";
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

const mixedMarkets = [
  market,
  { ...market, symbol: symbol("kBONK"), assetId: 2 },
];
const mixedClient = {
  markets: {
    getMarkets: async () => ({ slot: 10, markets: mixedMarkets }),
    getMarket: async (): Promise<MarketResponse> => {
      throw new Error("Unexpected detail request");
    },
  },
};
const mixedPrices = {
  getMarkPrices: async () => ({ "sol-perp": 100, KBONK: 2 }),
  getIndexPrices: async () => ({ "sol-perp": 90, kbonk: 1 }),
};

it("builds canonical margin symbols from mixed-case mark and index prices", () => {
  const result = buildMarketParamsBySymbol(
    mixedMarkets,
    { "sol-perp": 100, " KBONK ": 2 },
    {
      indexPricesBySymbol: { "SOL-perp": 90, kbonk: 1 },
    }
  );
  expect(Object.keys(result)).toEqual(["SOL-PERP", "kBONK"]);
  expect(result.kBONK).toMatchObject({
    symbol: "kBONK",
    markPriceTicks: "2000000",
    indexPriceTicks: "1000000",
  });
  expect(
    buildMarketParamsFromSummary(market, 100, {
      indexPricesBySymbol: { "sol-perp": 90 },
    })?.indexPriceTicks
  ).toBe("90000000");
});

it.each(["skip", "zero", "error"] as const)(
  "merges partial prices and handles explicit null with %s",
  async (missingPriceBehavior) => {
    const store = new MarginMarketParamsStore({
      client: mixedClient,
      priceSource: mixedPrices,
      missingPriceBehavior,
    });
    await store.refresh();
    store.updateMarkPrices({ kbonk: 3 });
    store.updateMarkPrices({ "SOL-perp": 110 });
    const before = await store.getSnapshot();
    expect(before.paramsBySymbol.kBONK).toMatchObject({
      markPriceTicks: "3000000",
      indexPriceTicks: "1000000",
    });
    expect(before.paramsBySymbol["SOL-PERP"]).toMatchObject({
      markPriceTicks: "110000000",
      indexPriceTicks: "90000000",
    });
    if (missingPriceBehavior === "error") {
      expect(() => store.updateMarkPrices({ KBONK: null })).toThrow(
        "Missing mark price for kBONK"
      );
      expect(await store.getSnapshot()).toBe(before);
    } else {
      store.updateMarkPrices({ KBONK: null });
      expect(
        (await store.getSnapshot()).paramsBySymbol.kBONK?.markPriceTicks
      ).toBe(missingPriceBehavior === "zero" ? "0" : undefined);
    }
    store.updateMarkPrices({ "sol-perp": 120 });
    const next = await store.getSnapshot();
    expect(next.paramsBySymbol["SOL-PERP"]?.indexPriceTicks).toBe("90000000");
    expect(next.paramsBySymbol.kBONK?.markPriceTicks).toBe(
      missingPriceBehavior === "error"
        ? "3000000"
        : missingPriceBehavior === "zero"
          ? "0"
          : undefined
    );
  }
);

it.each([true, false])(
  "prefers exact canonical prices regardless of insertion order (canonical first: %s)",
  (canonicalFirst) => {
    const prices = canonicalFirst
      ? { kBONK: 2, KBONK: 9 }
      : { KBONK: 9, kBONK: 2 };
    const result = buildMarketParamsBySymbol([mixedMarkets[1]], prices, {
      indexPricesBySymbol: prices,
    });
    expect(Object.keys(result)).toEqual(["kBONK"]);
    expect(result.kBONK).toMatchObject({
      markPriceTicks: "2000000",
      indexPriceTicks: "2000000",
    });
    expect(prices.kBONK).toBe(2);
    expect(prices.KBONK).toBe(9);
  }
);

it("honors exact null prices instead of falling back to an alias", () => {
  expect(
    buildMarketParamsBySymbol(
      [mixedMarkets[1]],
      { kBONK: null, KBONK: 9 },
      { missingPriceBehavior: "skip" }
    )
  ).toEqual({});
  const result = buildMarketParamsBySymbol(
    [mixedMarkets[1]],
    { kBONK: 2 },
    { indexPricesBySymbol: { kBONK: null, KBONK: 9 } }
  );
  expect(result.kBONK.indexPriceTicks).toBeUndefined();
});

it.each(["mark", "index"] as const)(
  "rejects ambiguous %s aliases without changing the last successful state",
  async (priceKind) => {
    let ambiguous = false;
    const aliases = { KBONK: 2, kbonk: 9 };
    const store = new MarginMarketParamsStore({
      client: mixedClient,
      priceSource: {
        getMarkPrices: async () =>
          ambiguous && priceKind === "mark"
            ? { "SOL-PERP": 100, ...aliases }
            : { "SOL-PERP": 100, kBONK: 2 },
        getIndexPrices: async () =>
          ambiguous && priceKind === "index" ? aliases : { kBONK: 1 },
      },
    });
    const before = await store.refresh();
    expect(() => store.updateMarkPrices(aliases)).toThrow(
      "Ambiguous price aliases for kBONK"
    );
    expect(await store.getSnapshot()).toBe(before);
    ambiguous = true;
    await expect(store.refresh()).rejects.toThrow(
      "Ambiguous price aliases for kBONK"
    );
    expect(await store.getSnapshot()).toBe(before);
    store.updateMarkPrices({ " KBONK ": 3, unknown: 9 });
    const after = await store.getSnapshot();
    expect(Object.keys(after.paramsBySymbol)).toEqual(["SOL-PERP", "kBONK"]);
    expect(after.paramsBySymbol.kBONK).toMatchObject({
      markPriceTicks: "3000000",
      indexPriceTicks: "1000000",
    });
    expect(after.paramsBySymbol["SOL-PERP"].markPriceTicks).toBe("100000000");
  }
);
