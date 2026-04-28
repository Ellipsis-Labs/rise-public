import { createStore } from "zustand/vanilla";
import { describe, expect, it } from "vitest";
import type {
  AllMidsUpdate,
  ExchangeSnapshotView,
  PhoenixExchangeMetadata,
  PhoenixExchangeStoreState,
  MarkPriceUpdate,
  MarketStatsUpdate,
} from "@/index";
import { createPhoenixMarketData, selectMarketDataRow } from "@/index";

const buildSnapshot = (): ExchangeSnapshotView => ({
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
    canonicalMint: "So11111111111111111111111111111111111111112",
    usdcMint: "EPjFWdd5AufqSSqeM2qAEyFnjAu1pzsVbW8UVzA73NA",
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
  markets: [
    {
      symbol: "SOL-PERP",
      assetId: 1,
      marketStatus: "active",
      marketPubkey: "sol-market",
      splinePubkey: "sol-spline",
      tickSize: 100,
      baseLotsDecimals: 3,
      takerFee: 0.0005,
      makerFee: -0.0001,
      leverageTiers: [],
      riskFactors: {
        maintenance: 5,
        backstop: 8,
        highRisk: 12,
        upnl: 90,
        upnlForWithdrawals: 80,
        cancelOrder: 2.5,
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
    },
    {
      symbol: "BTC-PERP",
      assetId: 2,
      marketStatus: "active",
      marketPubkey: "btc-market",
      splinePubkey: "btc-spline",
      tickSize: 100,
      baseLotsDecimals: 3,
      takerFee: 0.0005,
      makerFee: -0.0001,
      leverageTiers: [],
      riskFactors: {
        maintenance: 5,
        backstop: 8,
        highRisk: 12,
        upnl: 90,
        upnlForWithdrawals: 80,
        cancelOrder: 2.5,
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
    },
  ],
});

const createExchangeStub = (): PhoenixExchangeMetadata => {
  const snapshot = buildSnapshot();
  const marketsBySymbol = Object.fromEntries(
    snapshot.markets.map((market) => [market.symbol, market])
  );
  const marketsByAssetId = Object.fromEntries(
    snapshot.markets.map((market) => [market.assetId, market])
  );
  const marketsByPubkey = Object.fromEntries(
    snapshot.markets.map((market) => [market.marketPubkey, market])
  );
  const store = createStore<PhoenixExchangeStoreState>(() => ({
    health: "bootstrapped",
    source: {
      active: "api",
      priority: "api",
      fallbackAvailable: false,
      lastSyncAt: Date.now(),
    },
    snapshot,
    marketsBySymbol,
    marketsByAssetId,
    marketsByPubkey,
    marketSymbols: ["BTC-PERP", "SOL-PERP"],
    activeMarketSymbols: ["BTC-PERP", "SOL-PERP"],
    gatedMarketSymbols: [],
    closedMarketSymbols: [],
    marketStatusBySymbol: {
      "BTC-PERP": {
        symbol: "BTC-PERP",
        marketStatus: "active",
        lifecycle: "active",
        isActive: true,
        isGated: false,
        isClosed: false,
      },
      "SOL-PERP": {
        symbol: "SOL-PERP",
        marketStatus: "active",
        lifecycle: "active",
        isActive: true,
        isGated: false,
        isClosed: false,
      },
    },
    latestChange: null,
    recentChanges: [],
    latestChangeBySymbol: {},
    lastUpdatedMs: Date.now(),
  }));

  return {
    store,
    ready: async () => snapshot,
    health: () => "bootstrapped",
    snapshot: () => snapshot,
    market: (symbol) => marketsBySymbol[symbol.toUpperCase()],
    marketByAssetId: (assetId) => marketsByAssetId[assetId],
    marketByPubkey: (pubkey) => marketsByPubkey[pubkey],
    instructionContext: (symbol) => {
      const market = marketsBySymbol[symbol.toUpperCase()];
      return market ? { exchange: snapshot.exchange, market } : undefined;
    },
    source: () => ({
      active: "api",
      priority: "api",
      fallbackAvailable: false,
      lastSyncAt: Date.now(),
    }),
    subscribe: () => () => undefined,
    onEvent: () => () => undefined,
    close: () => undefined,
  };
};

const createGlobalPort = <T>() => {
  const values: T[] = [];
  const waiters: Array<(result: IteratorResult<T>) => void> = [];

  const push = (value: T) => {
    const waiter = waiters.shift();
    if (waiter) {
      waiter({ value, done: false });
      return;
    }
    values.push(value);
  };

  const port = (signal?: AbortSignal) => {
    const queue = {
      [Symbol.asyncIterator]() {
        return {
          next: async (): Promise<IteratorResult<T>> => {
            if (values.length > 0) {
              return { value: values.shift()!, done: false };
            }
            if (signal?.aborted) {
              return { value: undefined, done: true };
            }
            return new Promise<IteratorResult<T>>((resolve) => {
              waiters.push(resolve);
            });
          },
          return: async () => ({ value: undefined, done: true }),
        };
      },
    };

    signal?.addEventListener(
      "abort",
      () => {
        while (waiters.length > 0) {
          waiters.shift()?.({ value: undefined, done: true });
        }
      },
      { once: true }
    );

    return queue;
  };

  return { push, port };
};

const createMarkPricePort = () => {
  const streams = new Map<
    string,
    {
      values: MarkPriceUpdate[];
      waiters: Array<(result: IteratorResult<MarkPriceUpdate>) => void>;
    }
  >();
  const calls: string[] = [];

  const getStream = (symbol: string) => {
    const normalized = symbol.toUpperCase();
    const existing = streams.get(normalized);
    if (existing) {
      return existing;
    }
    const next = { values: [], waiters: [] };
    streams.set(normalized, next);
    return next;
  };

  const push = (symbol: string, value: MarkPriceUpdate) => {
    const stream = getStream(symbol);
    const waiter = stream.waiters.shift();
    if (waiter) {
      waiter({ value, done: false });
      return;
    }
    stream.values.push(value);
  };

  const port = (symbol: string, signal?: AbortSignal) => {
    const normalized = symbol.toUpperCase();
    calls.push(normalized);
    const stream = getStream(normalized);

    const queue = {
      [Symbol.asyncIterator]() {
        return {
          next: async (): Promise<IteratorResult<MarkPriceUpdate>> => {
            if (stream.values.length > 0) {
              return { value: stream.values.shift()!, done: false };
            }
            if (signal?.aborted) {
              return { value: undefined, done: true };
            }
            return new Promise<IteratorResult<MarkPriceUpdate>>((resolve) => {
              stream.waiters.push(resolve);
            });
          },
          return: async () => ({ value: undefined, done: true }),
        };
      },
    };

    signal?.addEventListener(
      "abort",
      () => {
        while (stream.waiters.length > 0) {
          stream.waiters.shift()?.({ value: undefined, done: true });
        }
      },
      { once: true }
    );

    return queue;
  };

  return { push, port, calls };
};

const waitUntil = async (
  predicate: () => boolean,
  message = "condition not met before timeout"
): Promise<void> => {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error(message);
};

describe("createPhoenixMarketData", () => {
  it("seeds symbols from exchange metadata and merges market data streams", async () => {
    const allMids = createGlobalPort<AllMidsUpdate>();
    const marketStats = createGlobalPort<MarketStatsUpdate>();
    const markPrice = createMarkPricePort();

    const marketData = createPhoenixMarketData({
      exchange: createExchangeStub(),
      allMids: allMids.port,
      marketStats: marketStats.port as never,
      markPrice: markPrice.port,
    });

    const release = marketData.retain();
    await marketData.ready();

    expect(marketData.store.getState().symbols).toEqual([
      "BTC-PERP",
      "SOL-PERP",
    ]);

    await waitUntil(
      () =>
        markPrice.calls.includes("SOL-PERP") &&
        markPrice.calls.includes("BTC-PERP"),
      "mark price subscriptions were not started for exchange symbols"
    );

    marketStats.push({
      symbol: "SOL-PERP",
      stats: {
        timestamp: 1_700_000_000_000n,
        openInterest: 12,
        markPrice: 100,
        oraclePrice: 99.5,
        prevDayMarkPrice: 95,
        dayVolumeUsd: 1_000,
        dayVolumeBase: 10,
        currentFundingRate: 0.01,
        eightHourFundingRate: 0.08,
        annualizedFundingRate: 1.2,
      },
    });

    await waitUntil(
      () =>
        marketData.store.getState().marketsBySymbol["SOL-PERP"]?.markPrice ===
        100
    );

    markPrice.push("SOL-PERP", {
      symbol: "SOL-PERP",
      slot: 2n,
      slotIndex: 1,
      markPrice: 101,
    });

    await waitUntil(
      () =>
        marketData.store.getState().marketsBySymbol["SOL-PERP"]?.markPrice ===
        101
    );

    allMids.push({
      mids: {
        "SOL-PERP": 100.5,
        "BTC-PERP": 200.25,
      },
      slot: 3n,
      slotIndex: 2,
    });

    await waitUntil(
      () =>
        marketData.store.getState().marketsBySymbol["BTC-PERP"]?.mid === 200.25
    );

    const solBeforeBtcStats =
      marketData.store.getState().marketsBySymbol["SOL-PERP"];

    marketStats.push({
      symbol: "BTC-PERP",
      stats: {
        timestamp: 1_700_000_000_500n,
        openInterest: 8,
        markPrice: 200,
        oraclePrice: 199.5,
        prevDayMarkPrice: 190,
        dayVolumeUsd: 800,
        dayVolumeBase: 4,
        currentFundingRate: 0.02,
        eightHourFundingRate: 0.12,
        annualizedFundingRate: 1.8,
      },
    });

    await waitUntil(
      () =>
        marketData.store.getState().marketsBySymbol["BTC-PERP"]
          ?.openInterest === 8
    );

    const state = marketData.store.getState();
    expect(state.status.health).toBe("live");
    expect(state.status.isConnected).toBe(true);
    expect(state.marketsBySymbol["SOL-PERP"]?.mid).toBe(100.5);
    expect(state.marketsBySymbol["SOL-PERP"]?.oraclePrice).toBe(99.5);
    expect(state.marketsBySymbol["SOL-PERP"]).toBe(solBeforeBtcStats);
    expect(state.latestChange?.symbol).toBe("BTC-PERP");
    expect(state.recentChanges).toHaveLength(4);

    release();
    marketData.close();
  });

  it("exposes per-symbol resources and selector helpers", async () => {
    const allMids = createGlobalPort<AllMidsUpdate>();
    const marketData = createPhoenixMarketData({
      exchange: createExchangeStub(),
      allMids: allMids.port,
    });

    const resource = marketData.resource("sol-perp");
    const sameResource = marketData.resource("SOL-PERP");

    expect(sameResource).toBe(resource);

    const release = resource.retain();
    await resource.ready();

    expect(resource.store.getState().symbol).toBe("SOL-PERP");
    expect(
      selectMarketDataRow("sol-perp")(marketData.store.getState())?.symbol
    ).toBe("SOL-PERP");

    allMids.push({
      mids: {
        "SOL-PERP": 101.25,
      },
      slot: 9n,
      slotIndex: 4,
    });

    await waitUntil(() => resource.snapshot()?.mid === 101.25);

    expect(resource.store.getState().row).toBe(
      marketData.store.getState().marketsBySymbol["SOL-PERP"]
    );

    release();
    resource.close();
    marketData.close();
  });
});
