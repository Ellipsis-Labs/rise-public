import { createStore } from "zustand/vanilla";
import { describe, expect, it, vi } from "vitest";
import type {
  ExchangeMarketSnapshot,
  MarketPublicMetadata,
  PhoenixExchangeStoreState,
} from "@/index";
import {
  createPhoenixExchangeMarketSelection,
  projectPhoenixExchangeMarketState,
  projectPhoenixMarket,
  projectPhoenixMarketsBySymbol,
  selectPhoenixExchangeMarket,
  selectPhoenixExchangeMarketChange,
  selectPhoenixExchangeMarketMetadata,
  selectPhoenixExchangeMarketStatus,
} from "@/index";

const buildMarket = (
  symbol: string,
  overrides: Partial<ExchangeMarketSnapshot> = {}
): ExchangeMarketSnapshot => ({
  symbol,
  assetId: symbol === "BTC" ? 2 : 1,
  marketStatus: "active",
  marketPubkey: `${symbol.toLowerCase()}-market`,
  splinePubkey: `${symbol.toLowerCase()}-spline`,
  tickSize: 100,
  baseLotsDecimals: 3,
  takerFee: 0.0005,
  makerFee: -0.0001,
  leverageTiers: [
    {
      maxLeverage: 20,
      maxSizeBaseLots: 1_000n,
      limitOrderRiskFactor: 250,
    },
  ],
  riskFactors: {
    maintenance: 5,
    maintenanceBps: 500,
    backstop: 8,
    backstopBps: 800,
    highRisk: 12,
    highRiskBps: 1200,
    upnl: 90,
    upnlBps: 9000,
    upnlForWithdrawals: 80,
    upnlForWithdrawalsBps: 8000,
    cancelOrder: 2.5,
    cancelOrderBps: 250,
  },
  fundingConfig: {
    fundingIntervalSeconds: 3600,
    fundingPeriodSeconds: 28800,
    maxFundingRatePerInterval: 2500,
  },
  openInterestCapBaseLots: 5_000n,
  maxLiquidationSizeBaseLots: 250n,
  isolatedOnly: false,
  markPriceParameters: {
    emaPeriodSlots: 1n,
    emaDiffRadius: 1n,
    bookPriceRadius: 1n,
    commoditiesAfterHoursRadius: 0n,
    adjustedExchangeSpotPriceWeight: 1n,
    bookPriceWeight: 1n,
    exchangePerpPriceWeight: 1n,
    spotPriceStaleThreshold: 1n,
    bookPriceStaleThreshold: 1n,
    perpPriceStaleThreshold: 1n,
    riskActionPriceValidityRules: Array.from({ length: 8 }, () =>
      Array.from({ length: 4 }, () =>
        Array.from({ length: 8 }, () => "ignore" as const)
      )
    ),
    oracleDivergenceRadius: 1,
    minOracleResponses: 1,
  },
  commodityMetadata: null,
  ...overrides,
});

const createExchangeStoreState = (
  markets: readonly ExchangeMarketSnapshot[]
): PhoenixExchangeStoreState => {
  const marketsBySymbol = Object.fromEntries(
    markets.map((market) => [market.symbol, market])
  );
  const marketsByAssetId = Object.fromEntries(
    markets.map((market) => [market.assetId, market])
  );
  const marketsByPubkey = Object.fromEntries(
    markets.map((market) => [market.marketPubkey, market])
  );
  const marketMetadataBySymbol = Object.fromEntries(
    markets.map((market) => [market.symbol, market.metadata ?? null])
  );
  const marketMetadataByAssetId = Object.fromEntries(
    markets.map((market) => [market.assetId, market.metadata ?? null])
  );
  const marketMetadataByPubkey = Object.fromEntries(
    markets.map((market) => [market.marketPubkey, market.metadata ?? null])
  );
  const marketStatusBySymbol = Object.fromEntries(
    markets.map((market) => [
      market.symbol,
      {
        symbol: market.symbol,
        marketStatus: market.marketStatus,
        lifecycle: market.marketStatus === "active" ? "active" : "gated",
        isActive: market.marketStatus === "active",
        isGated: market.marketStatus !== "active",
        isClosed: market.marketStatus === "closed",
      } as const,
    ])
  );

  return {
    health: "bootstrapped",
    source: {
      active: "api",
      priority: "api",
      fallbackAvailable: false,
    },
    snapshot: {
      version: 1,
      slot: 1n,
      slotIndex: 0,
      exchange: {
        programId: "program-id",
        globalConfig: "global-config",
        currentAuthorities: {
          rootAuthority: "root",
          riskAuthority: "risk",
          marketAuthority: "market",
          oracleAuthority: "oracle",
          adlAuthority: "adl",
          cancelAuthority: "cancel",
          backstopAuthority: "backstop",
        },
        canonicalMint: "canonical-mint",
        usdcMint: "usdc-mint",
        globalVault: "global-vault",
        perpAssetMap: "perp-asset-map",
        globalTraderIndex: ["gti-0"],
        activeTraderBuffer: ["atb-0"],
        withdrawQueue: "withdraw-queue",
        exchangeStatusBits: 129,
        exchangeStatusFeatures: ["initialized", "active"],
        active: true,
        gated: false,
      },
      markets: [...markets],
    },
    marketsBySymbol,
    marketsByAssetId,
    marketsByPubkey,
    marketMetadataBySymbol,
    marketMetadataByAssetId,
    marketMetadataByPubkey,
    marketSymbols: markets.map((market) => market.symbol).sort(),
    activeMarketSymbols: markets
      .filter((market) => market.marketStatus === "active")
      .map((market) => market.symbol)
      .sort(),
    gatedMarketSymbols: markets
      .filter((market) => market.marketStatus !== "active")
      .map((market) => market.symbol)
      .sort(),
    closedMarketSymbols: markets
      .filter((market) => market.marketStatus === "closed")
      .map((market) => market.symbol)
      .sort(),
    marketStatusBySymbol,
    latestChange: null,
    recentChanges: [],
    latestChangeBySymbol: {},
    lastUpdatedMs: 1_700_000_000_000,
  };
};

describe("exchange selectors", () => {
  it("projects exchange markets and exposes canonical selectors", async () => {
    const solMetadata: MarketPublicMetadata = {
      name: "Solana",
      logoUri: "https://example.com/sol.png",
      coinGeckoId: "solana",
      displayColor: "#14f195",
    };
    const solMarket = buildMarket("SOL", {
      metadata: solMetadata,
    });
    const btcMarket = buildMarket("BTC", {
      marketStatus: "paused",
    });
    const exchangeStore = createStore<PhoenixExchangeStoreState>(() =>
      createExchangeStoreState([solMarket, btcMarket])
    );
    const exchange = {
      store: exchangeStore,
      ready: vi.fn(async () => exchangeStore.getState().snapshot!),
    };

    expect(
      selectPhoenixExchangeMarket("sol")(exchangeStore.getState())?.symbol
    ).toBe("SOL");
    expect(
      selectPhoenixExchangeMarketStatus("btc")(exchangeStore.getState())
        ?.marketStatus
    ).toBe("paused");
    expect(
      selectPhoenixExchangeMarketChange("sol")(exchangeStore.getState())
    ).toBeNull();
    expect(
      selectPhoenixExchangeMarketMetadata("sol")(exchangeStore.getState())
    ).toBe(solMetadata);

    const projectedA = projectPhoenixExchangeMarketState(
      exchangeStore.getState(),
      "sol"
    );
    const projectedB = projectPhoenixExchangeMarketState(
      exchangeStore.getState(),
      "SOL"
    );
    expect(projectedA).toBe(projectedB);
    expect(projectedA.metadata).toBe(solMetadata);

    const selection = createPhoenixExchangeMarketSelection(
      exchange as never,
      "SOL"
    );
    expect(selection.store.getState().market?.symbol).toBe("SOL");

    exchangeStore.setState((state) => ({
      ...state,
      marketsBySymbol: {
        ...state.marketsBySymbol,
        SOL: {
          ...state.marketsBySymbol.SOL!,
          tickSize: 250,
        },
      },
      latestChangeBySymbol: {
        ...state.latestChangeBySymbol,
        SOL: {
          receivedAtMs: 1_700_000_010_000,
          scope: "market",
          symbol: "SOL",
          eventType: "marketUpdated:tickSize",
          detail: "SOL updated (tickSize)",
          slot: 2n,
          slotIndex: 0,
        },
      },
      lastUpdatedMs: 1_700_000_010_000,
    }));

    expect(selection.store.getState().market?.tickSize).toBe(250);
    expect(selection.store.getState().latestChange?.symbol).toBe("SOL");

    selection.set("BTC");
    expect(selection.store.getState().symbol).toBe("BTC");
    expect(selection.store.getState().market?.symbol).toBe("BTC");

    await selection.ready();
    expect(exchange.ready).toHaveBeenCalledTimes(1);

    selection.close();
  });

  it("projects normalized market display fields from exchange snapshots", () => {
    const market = buildMarket("SOL", {
      riskFactors: {
        maintenance: 0.5,
        maintenanceBps: 5_000,
        backstop: 0.2,
        backstopBps: 2_000,
        highRisk: 0.1,
        highRiskBps: 1_000,
        upnl: 1,
        upnlBps: 10_000,
        upnlForWithdrawals: 0.01,
        upnlForWithdrawalsBps: 100,
        cancelOrder: 0.7,
        cancelOrderBps: 7_000,
      },
    });

    const projectedA = projectPhoenixMarket(market);
    const projectedB = projectPhoenixMarket(market);
    expect(projectedA).toBe(projectedB);
    expect(projectedA.symbol).toBe("SOL");
    expect(projectedA.fees).toEqual({
      takerFeeMicro: 500,
      makerFeeMicro: -100,
    });
    expect(projectedA.units).toEqual({
      baseLotsDecimals: 3,
      tickSizeInQuoteLotsPerBaseLot: 100,
    });
    expect(projectedA.tickSize).toBe(0.1);
    expect(projectedA.priceDecimals).toBe(2);
    expect(projectedA.leverageTiers[0]).toEqual({
      maxLeverage: 20,
      maxSizeBaseLots: 1000,
      limitOrderRiskFactor: 250,
    });
    expect(projectedA.riskFactors).toEqual({
      maintenance: 5_000,
      backstop: 2_000,
      highRisk: 1_000,
      upnl: 10_000,
      upnlForWithdrawals: 100,
      cancelOrder: 7_000,
    });

    const projectedBySymbolA = projectPhoenixMarketsBySymbol({
      SOL: market,
    });
    const projectedBySymbolB = projectPhoenixMarketsBySymbol({
      SOL: market,
    });
    expect(projectedBySymbolA.SOL.symbol).toBe("SOL");
    expect(projectedBySymbolB.SOL.symbol).toBe("SOL");
  });
});
