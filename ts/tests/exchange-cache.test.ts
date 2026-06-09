import { describe, expect, it } from "vitest";
import {
  ExchangeCacheSequenceError,
  ExchangeWireMsgSchema,
  createPhoenixExchangeCache,
  createPhoenixExchangeCacheStore,
  projectExchangeMarket,
  projectPhoenixExchangeMarketState,
  selectExchangeMarket,
  selectExchangeMarketMetadata,
  selectExchangeMarketStatus,
  selectPhoenixExchangeMarketMetadata,
  type ExchangeCacheEvent,
  type ExchangeDeltaMsg,
  type ExchangeMsg,
  type ExchangeSnapshotMsg,
  type ExchangeSnapshotView,
  type MarketPublicMetadata,
} from "@/index";
import { createAsyncQueue, type AsyncQueue } from "@/ws";

const buildRiskActionPriceValidityRules = (
  fill: "ignore" | "require" | "forbid" = "ignore"
): ("ignore" | "require" | "forbid")[][][] =>
  Array.from({ length: 8 }, () =>
    Array.from({ length: 4 }, () => Array.from({ length: 8 }, () => fill))
  );

const buildSnapshot = (
  slot: bigint,
  slotIndex: number,
  marketOverrides: Partial<ExchangeSnapshotView["markets"][number]>[] = []
): ExchangeSnapshotView => ({
  version: 1,
  sequenceNumber: undefined,
  slot,
  slotIndex,
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
      leverageTiers: [
        {
          maxLeverage: 20,
          maxSizeBaseLots: 1_000n,
          limitOrderRiskFactor: 250,
        },
      ],
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
        emaPeriodSlots: 375n,
        emaDiffRadius: 250n,
        bookPriceRadius: 500n,
        commoditiesAfterHoursRadius: 0n,
        adjustedExchangeSpotPriceWeight: 2000n,
        bookPriceWeight: 4000n,
        exchangePerpPriceWeight: 4000n,
        spotPriceStaleThreshold: 120n,
        bookPriceStaleThreshold: 15n,
        perpPriceStaleThreshold: 15n,
        riskActionPriceValidityRules: buildRiskActionPriceValidityRules(),
        oracleDivergenceRadius: 500,
        minOracleResponses: 1,
      },
    },
    ...marketOverrides.map((market) => ({
      symbol: "BTC-PERP",
      assetId: 2,
      marketStatus: "active",
      marketPubkey: "btc-market",
      splinePubkey: "btc-spline",
      tickSize: 50,
      baseLotsDecimals: 3,
      takerFee: 0.0005,
      makerFee: -0.0001,
      leverageTiers: [
        {
          maxLeverage: 15,
          maxSizeBaseLots: 800n,
          limitOrderRiskFactor: 300,
        },
      ],
      riskFactors: {
        maintenance: 6,
        backstop: 9,
        highRisk: 13,
        upnl: 92,
        upnlForWithdrawals: 82,
        cancelOrder: 2.8,
      },
      fundingConfig: {
        fundingIntervalSeconds: 3600,
        fundingPeriodSeconds: 28800,
        maxFundingRatePerInterval: 2000,
      },
      openInterestCapBaseLots: 3_000n,
      maxLiquidationSizeBaseLots: 400n,
      isolatedOnly: false,
      markPriceParameters: {
        emaPeriodSlots: 375n,
        emaDiffRadius: 200n,
        bookPriceRadius: 450n,
        commoditiesAfterHoursRadius: 0n,
        adjustedExchangeSpotPriceWeight: 2500n,
        bookPriceWeight: 3500n,
        exchangePerpPriceWeight: 4000n,
        spotPriceStaleThreshold: 120n,
        bookPriceStaleThreshold: 10n,
        perpPriceStaleThreshold: 10n,
        riskActionPriceValidityRules: buildRiskActionPriceValidityRules(),
        oracleDivergenceRadius: 450,
        minOracleResponses: 1,
      },
      ...market,
    })),
  ],
});

const buildSnapshotMsg = (
  sequenceNumber: bigint,
  snapshot: ExchangeSnapshotView
): ExchangeSnapshotMsg => ({
  channel: "exchange",
  messageType: "snapshot",
  version: snapshot.version,
  sequenceNumber,
  slot: snapshot.slot,
  slotIndex: snapshot.slotIndex,
  reason: "snapshot",
  exchange: snapshot.exchange,
  markets: snapshot.markets,
});

const buildOpenInterestCapDelta = (
  sequenceNumber: bigint,
  slot: bigint,
  slotIndex: number,
  newBaseLots: bigint
): ExchangeDeltaMsg => ({
  channel: "exchange",
  messageType: "delta",
  version: 1,
  sequenceNumber,
  slot,
  slotIndex,
  ops: [
    {
      kind: "marketParameterUpdated",
      symbol: "SOL-PERP",
      update: {
        kind: "openInterestCapUpdated",
        previousBaseLots: 5_000n,
        newBaseLots,
      },
    },
  ],
});

const buildFundingParametersDelta = (
  sequenceNumber: bigint,
  slot: bigint,
  slotIndex: number,
  nextFundingConfig: ExchangeSnapshotView["markets"][number]["fundingConfig"]
): ExchangeDeltaMsg => ({
  channel: "exchange",
  messageType: "delta",
  version: 1,
  sequenceNumber,
  slot,
  slotIndex,
  ops: [
    {
      kind: "marketParameterUpdated",
      symbol: "SOL-PERP",
      update: {
        kind: "fundingParametersUpdated",
        previous: {
          fundingIntervalSeconds: 3600,
          fundingPeriodSeconds: 28800,
          maxFundingRatePerInterval: 2500,
        },
        new: nextFundingConfig,
      },
    },
  ],
});

const buildMarkPriceParametersDelta = (
  sequenceNumber: bigint,
  slot: bigint,
  slotIndex: number,
  nextMarkPriceParameters: ExchangeSnapshotView["markets"][number]["markPriceParameters"]
): ExchangeDeltaMsg => ({
  channel: "exchange",
  messageType: "delta",
  version: 1,
  sequenceNumber,
  slot,
  slotIndex,
  ops: [
    {
      kind: "marketParameterUpdated",
      symbol: "SOL-PERP",
      update: {
        kind: "markPriceParametersUpdated",
        previous: {
          emaPeriodSlots: 375n,
          emaDiffRadius: 250n,
          bookPriceRadius: 500n,
          commoditiesAfterHoursRadius: 0n,
          adjustedExchangeSpotPriceWeight: 2000n,
          bookPriceWeight: 4000n,
          exchangePerpPriceWeight: 4000n,
          spotPriceStaleThreshold: 120n,
          bookPriceStaleThreshold: 15n,
          perpPriceStaleThreshold: 15n,
          riskActionPriceValidityRules: buildRiskActionPriceValidityRules(),
          oracleDivergenceRadius: 500,
          minOracleResponses: 1,
        },
        new: nextMarkPriceParameters,
      },
    },
  ],
});

const buildCommodityMetadata = (
  overrides: Partial<
    NonNullable<ExchangeSnapshotView["markets"][number]["commodityMetadata"]>
  > = {}
): NonNullable<
  ExchangeSnapshotView["markets"][number]["commodityMetadata"]
> => ({
  isCommodity: true,
  isReopen: false,
  isAfterHours: false,
  status: "active",
  afterHoursRadius: {
    value: 150,
    decimals: 0,
    ui: "150",
  },
  lastKnownIndexPrice: {
    value: 742500,
    decimals: 2,
    ui: "7425.00",
  },
  markPriceBand: {
    lower: {
      value: 741000,
      decimals: 2,
      ui: "7410.00",
    },
    upper: {
      value: 744000,
      decimals: 2,
      ui: "7440.00",
    },
  },
  executionPriceBand: {
    lower: {
      value: 741500,
      decimals: 2,
      ui: "7415.00",
    },
    upper: {
      value: 743500,
      decimals: 2,
      ui: "7435.00",
    },
  },
  lastIndexExpiryTimestamp: 1713200000,
  ...overrides,
});

const sampleMarketMetadata = (name = "Solana Perp"): MarketPublicMetadata => ({
  name,
  description: "Solana perpetual market",
  logoUri: "https://example.com/sol.png",
  coinGeckoId: "solana",
  coinMarketCapId: 5426,
  tokensXyzAssetId: "solana",
  calendar: {
    id: "cme_commodities",
    description: "CME commodities",
    calendarUri: "https://phoenixcdn.trade/calendars/cme_commodities.json",
    contentSha256:
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    nextMarketTransitionUtc: "2026-04-24T21:00:00Z",
  },
  displayColor: "#14f195",
});

const buildMarketMetadataDelta = (
  sequenceNumber: bigint,
  slot: bigint,
  slotIndex: number,
  metadata: MarketPublicMetadata | null
): ExchangeDeltaMsg => ({
  channel: "exchange",
  messageType: "delta",
  version: 1,
  sequenceNumber,
  slot,
  slotIndex,
  ops: [
    {
      kind: "marketMetadataUpdated",
      symbol: "SOL-PERP",
      metadata,
    },
  ],
});

const buildUnknownDelta = (
  sequenceNumber: bigint,
  slot: bigint,
  slotIndex: number
): ExchangeDeltaMsg => {
  const parsed = ExchangeWireMsgSchema.parse({
    channel: "exchange",
    messageType: "delta",
    version: 1,
    sequenceNumber,
    slot,
    slotIndex,
    ops: [
      {
        kind: "futureMarketThingUpdated",
        symbol: "SOL-PERP",
        futureField: true,
      },
    ],
  });
  if (parsed.messageType !== "delta") {
    throw new Error("expected delta message");
  }
  return parsed;
};

const buildUnknownMarketParameterDelta = (
  sequenceNumber: bigint,
  slot: bigint,
  slotIndex: number
): ExchangeDeltaMsg => {
  const parsed = ExchangeWireMsgSchema.parse({
    channel: "exchange",
    messageType: "delta",
    version: 1,
    sequenceNumber,
    slot,
    slotIndex,
    ops: [
      {
        kind: "marketParameterUpdated",
        symbol: "SOL-PERP",
        update: {
          kind: "futureParameterUpdated",
          futureField: true,
        },
      },
    ],
  });
  if (parsed.messageType !== "delta") {
    throw new Error("expected delta message");
  }
  return parsed;
};

const buildCommodityMetadataDelta = (
  sequenceNumber: bigint,
  slot: bigint,
  slotIndex: number,
  previous: NonNullable<
    ExchangeSnapshotView["markets"][number]["commodityMetadata"]
  >,
  next: NonNullable<
    ExchangeSnapshotView["markets"][number]["commodityMetadata"]
  >
): ExchangeDeltaMsg => ({
  channel: "exchange",
  messageType: "delta",
  version: 1,
  sequenceNumber,
  slot,
  slotIndex,
  ops: [
    {
      kind: "marketParameterUpdated",
      symbol: "SOL-PERP",
      update: {
        kind: "commodityMetadataUpdated",
        previous,
        new: next,
      },
    },
  ],
});

const createExchangePort = () => {
  const queues: Array<{
    queue: AsyncQueue<ExchangeMsg>;
    refreshCount: number;
    push: (message: ExchangeMsg) => boolean;
    close: () => void;
  }> = [];

  return {
    queues,
    exchange: (signal?: AbortSignal) => {
      const queue = createAsyncQueue<ExchangeMsg>(64, "drop-oldest", signal);
      const entry = {
        queue,
        refreshCount: 0,
        push: (message: ExchangeMsg) => queue.push(message),
        close: () => queue.close(),
      };
      queues.push(entry);
      return {
        refresh: () => {
          entry.refreshCount += 1;
        },
        [Symbol.asyncIterator]: () => queue[Symbol.asyncIterator](),
      };
    },
  };
};

const waitUntil = async (predicate: () => boolean): Promise<void> => {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error("condition not met before timeout");
};

const healthReasons = (
  events: ExchangeCacheEvent[]
): Array<ExchangeCacheEvent["reason"]> =>
  events
    .filter(
      (
        event
      ): event is Extract<ExchangeCacheEvent, { type: "healthChanged" }> => {
        return event.type === "healthChanged";
      }
    )
    .map((event) => event.reason);

describe("exchange cache", () => {
  it("exposes first-class per-symbol selectors and projections", () => {
    const metadata = sampleMarketMetadata();
    const snapshot = buildSnapshot(1n, 0);
    snapshot.markets[0] = {
      ...snapshot.markets[0],
      metadata,
    };
    const store = createPhoenixExchangeCacheStore(snapshot);
    const state = store.store.getState();

    const market = selectExchangeMarket("sol-perp")(state);
    const status = selectExchangeMarketStatus("sol-perp")(state);
    const selectedMetadata = selectExchangeMarketMetadata("sol-perp")(state);

    expect(market?.symbol).toBe("SOL-PERP");
    expect(status).toEqual({
      symbol: "SOL-PERP",
      marketStatus: "active",
      lifecycle: "active",
      isActive: true,
      isGated: false,
      isClosed: false,
    });
    expect(store.marketMetadata("sol-perp")).toEqual(metadata);
    expect(store.marketMetadataByAssetId(1)).toEqual(metadata);
    expect(store.marketMetadataByPubkey("sol-market")).toEqual(metadata);
    expect(selectedMetadata).toBe(store.marketMetadata("SOL-PERP"));
    expect(selectPhoenixExchangeMarketMetadata("sol-perp")(state)).toBe(
      selectedMetadata
    );
    expect(projectPhoenixExchangeMarketState(state, "sol-perp").metadata).toBe(
      selectedMetadata
    );

    const projected = projectExchangeMarket(market!);
    expect(projected.market).toBe(market);
    expect(projected.fees).toEqual({
      takerFeeRate: 0.0005,
      makerFeeRate: -0.0001,
      takerFeeMicro: 500,
      makerFeeMicro: -100,
    });
    expect(projected.units).toEqual({
      tickSizeInQuoteLotsPerBaseLot: 100,
      baseLotsDecimals: 3,
      quoteDecimals: 6,
      tickSize: 0.1,
    });
    expect(projected.display.priceDecimals).toBe(2);
  });

  it("applies snapshot and delta messages through the standalone store", () => {
    const liveSnapshot = buildSnapshot(2n, 1);
    liveSnapshot.markets[0] = {
      ...liveSnapshot.markets[0],
      metadata: sampleMarketMetadata(),
    };
    const store = createPhoenixExchangeCacheStore(buildSnapshot(1n, 0));
    const events: ExchangeCacheEvent[] = [];
    store.onEvent((event) => {
      events.push(event);
    });

    store.applySnapshotMessage(buildSnapshotMsg(10n, liveSnapshot));
    const metadataBeforeParameterDelta = store.marketMetadata("sol-perp");

    store.applyDelta(buildOpenInterestCapDelta(11n, 3n, 2, 7_500n));
    expect(store.marketMetadata("sol-perp")).toBe(metadataBeforeParameterDelta);

    const updatedMetadata = sampleMarketMetadata("Updated Solana Perp");
    const parsedMetadataDelta = ExchangeWireMsgSchema.parse(
      buildMarketMetadataDelta(12n, 4n, 0, updatedMetadata)
    );
    if (parsedMetadataDelta.messageType !== "delta") {
      throw new Error("expected parsed metadata delta");
    }
    store.applyDelta(parsedMetadataDelta);

    expect(store.snapshot().sequenceNumber).toBe(12n);
    expect(store.market("sol-perp")?.openInterestCapBaseLots).toBe(7_500n);
    expect(store.market("sol-perp")?.metadata).toBe(
      store.marketMetadata("SOL-PERP")
    );
    expect(store.marketMetadata("sol-perp")).toEqual(updatedMetadata);
    expect(store.marketMetadata("sol-perp")?.calendar).toEqual(
      updatedMetadata.calendar
    );
    expect(store.marketMetadata("sol-perp")).not.toBe(
      metadataBeforeParameterDelta
    );
    expect(store.marketMetadataByAssetId(1)).toEqual(updatedMetadata);
    expect(store.marketMetadataByPubkey("sol-market")).toEqual(updatedMetadata);
    expect(store.marketByAssetId(1)?.symbol).toBe("SOL-PERP");
    expect(store.marketByPubkey("sol-market")?.symbol).toBe("SOL-PERP");
    expect(store.instructionContext("SOL-PERP")?.market.assetId).toBe(1);
    expect(
      events.some(
        (event) =>
          event.type === "marketUpdated" &&
          event.change === "openInterestCap" &&
          event.symbol === "SOL-PERP"
      )
    ).toBe(true);
    expect(
      events.some(
        (event) =>
          event.type === "marketUpdated" &&
          event.change === "metadata" &&
          event.symbol === "SOL-PERP"
      )
    ).toBe(true);
  });

  it("ignores unknown exchange delta ops while advancing sequence", () => {
    const store = createPhoenixExchangeCacheStore(buildSnapshot(1n, 0));
    const events: ExchangeCacheEvent[] = [];
    store.onEvent((event) => {
      events.push(event);
    });

    store.applySnapshotMessage(buildSnapshotMsg(10n, buildSnapshot(2n, 1)));
    events.length = 0;
    store.applyDelta(buildUnknownDelta(11n, 5n, 0));

    expect(store.snapshot().sequenceNumber).toBe(11n);
    expect(store.market("sol-perp")?.openInterestCapBaseLots).toBe(5_000n);
    expect(events).toHaveLength(1);
    expect(events[0]).toMatchObject({
      type: "snapshotApplied",
      source: "websocket",
    });
  });

  it("ignores unknown market parameter updates while advancing sequence", () => {
    const store = createPhoenixExchangeCacheStore(buildSnapshot(1n, 0));
    const events: ExchangeCacheEvent[] = [];
    store.onEvent((event) => {
      events.push(event);
    });

    store.applySnapshotMessage(buildSnapshotMsg(10n, buildSnapshot(2n, 1)));
    events.length = 0;
    store.applyDelta(buildUnknownMarketParameterDelta(11n, 5n, 0));

    expect(store.snapshot().sequenceNumber).toBe(11n);
    expect(store.market("sol-perp")?.openInterestCapBaseLots).toBe(5_000n);
    expect(events).toHaveLength(1);
    expect(events[0]).toMatchObject({
      type: "snapshotApplied",
      source: "websocket",
    });
  });

  it("rejects standalone deltas when the sequence baseline is missing or gapped", () => {
    const store = createPhoenixExchangeCacheStore(buildSnapshot(1n, 0));

    expect(() =>
      store.applyDelta(buildOpenInterestCapDelta(2n, 2n, 0, 7_500n))
    ).toThrow(ExchangeCacheSequenceError);

    store.applySnapshotMessage(buildSnapshotMsg(10n, buildSnapshot(2n, 1)));

    expect(() =>
      store.applyDelta(buildOpenInterestCapDelta(12n, 3n, 0, 7_500n))
    ).toThrow(ExchangeCacheSequenceError);
  });

  it("bootstraps from HTTP, becomes live on websocket snapshot, and applies deltas", async () => {
    const initialSnapshot = buildSnapshot(1n, 0);
    const { queues, exchange } = createExchangePort();
    const events: ExchangeCacheEvent[] = [];
    const cache = await createPhoenixExchangeCache({
      api: {
        getSnapshot: async () => initialSnapshot,
      },
      exchange,
    });
    cache.onEvent((event) => {
      events.push(event);
    });

    expect((await cache.ready()).slot).toBe(1n);
    expect(cache.snapshot().sequenceNumber).toBeUndefined();
    expect(cache.health()).toBe("bootstrapped");

    await waitUntil(() => queues.length === 1);
    queues[0].push(buildSnapshotMsg(10n, buildSnapshot(2n, 1)));

    await waitUntil(() => cache.health() === "live");
    expect(cache.snapshot().sequenceNumber).toBe(10n);

    queues[0].push(buildOpenInterestCapDelta(11n, 3n, 2, 7_500n));

    await waitUntil(
      () => cache.market("SOL-PERP")?.openInterestCapBaseLots === 7_500n
    );
    expect(cache.snapshot().sequenceNumber).toBe(11n);

    const context = cache.instructionContext("SOL-PERP");
    expect(context?.exchange.perpAssetMap).toBe("perp-asset-map");
    expect(context?.market.assetId).toBe(1);
    expect(events.some((event) => event.type === "snapshotApplied")).toBe(true);
    expect(
      events.some(
        (event) =>
          event.type === "marketUpdated" &&
          event.symbol === "SOL-PERP" &&
          event.change === "openInterestCap"
      )
    ).toBe(true);

    cache.close();
  });

  it("refreshes the websocket stream before falling back to HTTP after a sequence gap", async () => {
    const initialSnapshot = buildSnapshot(1n, 0);
    let httpCalls = 0;
    const recoveredSnapshot = buildSnapshot(4n, 0, [
      {
        symbol: "BTC-PERP",
      },
    ]);
    const snapshots = [initialSnapshot, recoveredSnapshot];
    const { queues, exchange } = createExchangePort();

    const cache = await createPhoenixExchangeCache({
      api: {
        getSnapshot: async () => {
          httpCalls += 1;
          return snapshots.shift() ?? recoveredSnapshot;
        },
      },
      exchange,
    });

    await waitUntil(() => queues.length === 1);
    queues[0].push(buildSnapshotMsg(20n, buildSnapshot(2n, 1)));

    await waitUntil(() => cache.health() === "live");
    expect(cache.snapshot().sequenceNumber).toBe(20n);

    queues[0].push(buildOpenInterestCapDelta(22n, 3n, 0, 9_000n));

    await waitUntil(() => cache.health() === "recovering");
    await waitUntil(() => queues[0]?.refreshCount === 1);
    expect(queues).toHaveLength(1);
    expect(httpCalls).toBe(1);

    queues[0].push(
      buildSnapshotMsg(30n, buildSnapshot(5n, 0, [{ symbol: "BTC-PERP" }]))
    );

    await waitUntil(() => cache.health() === "live");
    expect(cache.snapshot().slot).toBe(5n);
    expect(cache.snapshot().sequenceNumber).toBe(30n);
    expect(httpCalls).toBe(1);

    cache.close();
  });

  it("refreshes the websocket stream when a delta arrives before the first snapshot", async () => {
    const initialSnapshot = buildSnapshot(1n, 0);
    let httpCalls = 0;
    const recoveredHttpSnapshot = buildSnapshot(4n, 0, [
      { symbol: "BTC-PERP" },
    ]);
    const snapshots = [initialSnapshot, recoveredHttpSnapshot];
    const { queues, exchange } = createExchangePort();
    const cache = await createPhoenixExchangeCache({
      api: {
        getSnapshot: async () => {
          httpCalls += 1;
          return snapshots.shift() ?? recoveredHttpSnapshot;
        },
      },
      exchange,
    });

    await waitUntil(() => queues.length === 1);
    queues[0].push(buildOpenInterestCapDelta(2n, 2n, 0, 7_500n));

    await waitUntil(() => cache.health() === "recovering");
    await waitUntil(() => queues[0]?.refreshCount === 1);
    expect(queues).toHaveLength(1);
    expect(httpCalls).toBe(1);

    queues[0].push(
      buildSnapshotMsg(10n, buildSnapshot(5n, 0, [{ symbol: "BTC-PERP" }]))
    );

    await waitUntil(() => cache.health() === "live");
    expect(cache.snapshot().slot).toBe(5n);
    expect(cache.snapshot().sequenceNumber).toBe(10n);
    expect(httpCalls).toBe(1);

    cache.close();
  });

  it("emits state-machine health transitions for stream end recovery and close", async () => {
    const initialSnapshot = buildSnapshot(1n, 0);
    const resyncedSnapshot = buildSnapshot(4n, 0, [{ symbol: "BTC-PERP" }]);
    const snapshots = [initialSnapshot, resyncedSnapshot];
    const { queues, exchange } = createExchangePort();
    const events: ExchangeCacheEvent[] = [];

    const cache = await createPhoenixExchangeCache({
      api: {
        getSnapshot: async () => snapshots.shift() ?? resyncedSnapshot,
      },
      exchange,
    });
    cache.onEvent((event) => {
      events.push(event);
    });

    await waitUntil(() => queues.length === 1);
    queues[0].push(buildSnapshotMsg(20n, buildSnapshot(2n, 0)));
    await waitUntil(() => cache.health() === "live");
    expect(cache.snapshot().sequenceNumber).toBe(20n);

    queues[0].close();

    await waitUntil(() => cache.health() === "recovering");
    await waitUntil(() => cache.snapshot().slot === 4n);
    expect(cache.snapshot().sequenceNumber).toBeUndefined();
    await waitUntil(() => queues.length === 2);

    queues[1].push(
      buildSnapshotMsg(30n, buildSnapshot(5n, 0, [{ symbol: "BTC-PERP" }]))
    );
    await waitUntil(() => cache.health() === "live");
    expect(cache.snapshot().sequenceNumber).toBe(30n);

    cache.close();
    await waitUntil(() => cache.health() === "closed");

    expect(healthReasons(events)).toEqual([
      "websocketSnapshot",
      "streamEnded",
      "websocketSnapshot",
      "closed",
    ]);
  });

  it("emits market update events after the snapshot has been committed", async () => {
    const initialSnapshot = buildSnapshot(1n, 0);
    const { queues, exchange } = createExchangePort();
    const cache = await createPhoenixExchangeCache({
      api: {
        getSnapshot: async () => initialSnapshot,
      },
      exchange,
    });

    let observedOpenInterestCap: bigint | undefined;
    let observedSequenceNumber: bigint | undefined;
    cache.onEvent((event) => {
      if (
        event.type !== "marketUpdated" ||
        event.change !== "openInterestCap"
      ) {
        return;
      }
      observedOpenInterestCap =
        cache.market("SOL-PERP")?.openInterestCapBaseLots;
      observedSequenceNumber = cache.snapshot().sequenceNumber;
    });

    await waitUntil(() => queues.length === 1);
    queues[0].push(buildSnapshotMsg(10n, buildSnapshot(2n, 1)));
    await waitUntil(() => cache.health() === "live");

    queues[0].push(buildOpenInterestCapDelta(11n, 3n, 2, 7_500n));

    await waitUntil(() => observedOpenInterestCap === 7_500n);
    expect(observedSequenceNumber).toBe(11n);

    cache.close();
  });

  it("replaces structured config blobs wholesale when applying deltas", async () => {
    const initialSnapshot = buildSnapshot(1n, 0);
    initialSnapshot.markets[0] = {
      ...initialSnapshot.markets[0],
      commodityMetadata: buildCommodityMetadata(),
    };

    const liveSnapshot = buildSnapshot(2n, 1);
    liveSnapshot.markets[0] = {
      ...liveSnapshot.markets[0],
      commodityMetadata: buildCommodityMetadata(),
    };

    const nextFundingConfig = {
      fundingIntervalSeconds: 7200,
      fundingPeriodSeconds: 172800,
      maxFundingRatePerInterval: 7500,
    };
    const nextMarkPriceParameters = {
      emaPeriodSlots: 450n,
      emaDiffRadius: 300n,
      bookPriceRadius: 650n,
      commoditiesAfterHoursRadius: 150n,
      adjustedExchangeSpotPriceWeight: 1800n,
      bookPriceWeight: 3200n,
      exchangePerpPriceWeight: 5000n,
      spotPriceStaleThreshold: 90n,
      bookPriceStaleThreshold: 25n,
      perpPriceStaleThreshold: 30n,
      riskActionPriceValidityRules:
        buildRiskActionPriceValidityRules("require"),
      oracleDivergenceRadius: 650,
      minOracleResponses: 2,
    };
    const nextCommodityMetadata = buildCommodityMetadata({
      isReopen: true,
      isAfterHours: false,
      status: "reopen",
      lastKnownIndexPrice: {
        value: 744100,
        decimals: 2,
        ui: "7441.00",
      },
      markPriceBand: {
        lower: {
          value: 742600,
          decimals: 2,
          ui: "7426.00",
        },
        upper: {
          value: 745600,
          decimals: 2,
          ui: "7456.00",
        },
      },
      executionPriceBand: {
        lower: {
          value: 743100,
          decimals: 2,
          ui: "7431.00",
        },
        upper: {
          value: 745100,
          decimals: 2,
          ui: "7451.00",
        },
      },
      lastIndexExpiryTimestamp: 1713203600,
    });

    const { queues, exchange } = createExchangePort();
    const cache = await createPhoenixExchangeCache({
      api: {
        getSnapshot: async () => initialSnapshot,
      },
      exchange,
    });

    await waitUntil(() => queues.length === 1);
    queues[0].push(buildSnapshotMsg(10n, liveSnapshot));
    await waitUntil(() => cache.health() === "live");

    queues[0].push(buildFundingParametersDelta(11n, 3n, 0, nextFundingConfig));
    await waitUntil(
      () =>
        cache.market("SOL-PERP")?.fundingConfig.maxFundingRatePerInterval ===
        nextFundingConfig.maxFundingRatePerInterval
    );
    expect(cache.market("SOL-PERP")?.fundingConfig).toEqual(nextFundingConfig);

    queues[0].push(
      buildMarkPriceParametersDelta(12n, 4n, 0, nextMarkPriceParameters)
    );
    await waitUntil(
      () =>
        cache.market("SOL-PERP")?.markPriceParameters.oracleDivergenceRadius ===
        nextMarkPriceParameters.oracleDivergenceRadius
    );
    expect(cache.market("SOL-PERP")?.markPriceParameters).toEqual(
      nextMarkPriceParameters
    );

    queues[0].push(
      buildCommodityMetadataDelta(
        13n,
        5n,
        0,
        buildCommodityMetadata(),
        nextCommodityMetadata
      )
    );
    await waitUntil(
      () => cache.market("SOL-PERP")?.commodityMetadata?.status === "reopen"
    );
    expect(cache.market("SOL-PERP")?.commodityMetadata).toEqual(
      nextCommodityMetadata
    );

    cache.close();
  });
});
