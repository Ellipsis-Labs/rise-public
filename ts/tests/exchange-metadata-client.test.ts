import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createPhoenixClient,
  createPhoenixExchangeMetadata,
  Direction,
  decodeGlobalConfiguration,
  flight,
  StopLossOrderKind,
} from "@/index";
import { MarginType } from "@/primitives";
import { baseLots, ticks } from "@/primitives/_numberTypes";
import { Side } from "@/primitives/Side";
import type { PerpAssetMetadata } from "@/accounts";
import type { ExchangeSnapshotView } from "@/api/exchange/types";
import type {
  ExchangeDeltaMsg,
  ExchangeMsg,
  ExchangeSnapshotMsg,
} from "@/ws/adapters/exchange";
import { createAsyncQueue, type AsyncQueue } from "@/ws";
import { address } from "@solana/kit";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as metadataShared from "@/exchange-cache/_metadataShared";
import * as accounts from "@/accounts";
import * as coreHelpers from "@/core/helpers";

const TESTS_DIR = dirname(fileURLToPath(import.meta.url));
const MOCKS_DIR = resolve(TESTS_DIR, "mocks");
const ORIGINAL_ENV = { ...process.env };

type FixtureFile = {
  account: {
    data: [string, string];
  };
};

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
  reason: "update",
  updateType: "marketParameterUpdate",
  symbol: "SOL-PERP",
  field: "openInterestCapBaseLots",
  value: newBaseLots,
});

const createExchangePort = () => {
  const queues: Array<{
    queue: AsyncQueue<ExchangeMsg>;
    signal?: AbortSignal;
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
        signal,
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

const loadMockBytes = (fileName: string): Uint8Array => {
  const fixture = JSON.parse(
    readFileSync(resolve(MOCKS_DIR, fileName), "utf8")
  ) as FixtureFile;
  return Buffer.from(fixture.account.data[0], "base64");
};

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  process.env = { ...ORIGINAL_ENV };
});

const stringifyWithBigints = (value: unknown): string =>
  JSON.stringify(value, (_, innerValue) =>
    typeof innerValue === "bigint" ? innerValue.toString() : innerValue
  );

const waitUntil = async (predicate: () => boolean): Promise<void> => {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error("condition not met before timeout");
};

describe("exchange metadata client integration", () => {
  it("keeps client exchange metadata lazy until used", () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    expect(fetchMock).not.toHaveBeenCalled();
    client.dispose();
  });

  it("de-duplicates concurrent ready calls behind a shared bootstrap promise", async () => {
    const fetchMock = vi.fn(async () => {
      await new Promise((resolve) => setTimeout(resolve, 5));
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      apiUrl: "https://example.com",
      ws: false,
    });

    const first = client.exchange.ready();
    const second = client.exchange.ready();

    expect(first).toBe(second);

    await Promise.all([first, second]);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    client.dispose();
  });

  it("arms rpc fallback for client exchange metadata when rpcUrl is provided", () => {
    const client = createPhoenixClient({
      apiUrl: "https://example.com",
      ws: false,
      rpcUrl: "https://rpc.example.com",
      exchangeMetadata: {
        priority: "api",
        rpc: {
          pollIntervalMs: 5_000,
          ttlMs: 30_000,
        },
      },
    });

    expect(client.exchange.store.getState().source).toEqual({
      active: "none",
      priority: "api",
      fallbackAvailable: true,
    });

    client.dispose();
  });

  it("arms rpc fallback from SOLANA_RPC_URL when rpcUrl is omitted", () => {
    process.env.SOLANA_RPC_URL = "https://rpc.example.com/";

    const client = createPhoenixClient({
      apiUrl: "https://example.com",
      ws: false,
      exchangeMetadata: {
        priority: "api",
      },
    });

    expect(client.exchange.store.getState().source).toEqual({
      active: "none",
      priority: "api",
      fallbackAvailable: true,
    });
    expect(client.rpc.available).toBe(true);

    client.dispose();
  });

  it("arms rpc metadata when priority is rpc", () => {
    const client = createPhoenixClient({
      apiUrl: "https://example.com",
      ws: false,
      rpcUrl: "https://rpc.example.com",
      exchangeMetadata: {
        priority: "rpc",
        rpc: {
          pollIntervalMs: 5_000,
          ttlMs: 30_000,
        },
      },
    });

    expect(client.exchange.store.getState().source).toEqual({
      active: "none",
      priority: "rpc",
      fallbackAvailable: true,
    });

    client.dispose();
  });

  it("bootstraps exchange metadata from the API snapshot on demand", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    const snapshot = await client.exchange.ready();
    expect(snapshot.exchange.globalConfig).toBe("global-config");
    expect(client.exchange.market("SOL-PERP")?.splinePubkey).toBe("sol-spline");
    expect(fetchMock).toHaveBeenCalledTimes(1);

    client.dispose();
  });

  it("falls back to RPC metadata bootstrap when the API snapshot is unavailable", async () => {
    const rpcSnapshot = buildSnapshot();
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "https://example.com/v1/exchange/snapshot") {
        return new Response("unavailable", { status: 503 });
      }
      return new Response("unexpected", { status: 500 });
    });
    vi.stubGlobal("fetch", fetchMock);
    const loadRpcSnapshotSpy = vi
      .spyOn(metadataShared, "loadRpcSnapshot")
      .mockResolvedValue(rpcSnapshot);

    const client = createPhoenixClient({
      apiUrl: "https://example.com",
      ws: false,
      rpcUrl: "https://rpc.example.com",
    });

    const snapshot = await client.exchange.ready();

    expect(snapshot).toEqual(rpcSnapshot);
    expect(client.exchange.source()).toEqual({
      active: "rpc",
      priority: "api",
      fallbackAvailable: true,
      lastSyncAt: expect.any(Number),
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(loadRpcSnapshotSpy).toHaveBeenCalledWith(
      "https://rpc.example.com",
      client.pda
    );

    client.dispose();
  });

  it("uses the RPC batch slot when bootstrapping exchange metadata from RPC", async () => {
    const globalConfigBytes = loadMockBytes("global_config.json");
    const perpAssetMapBytes = loadMockBytes("perp_asset_map.json");
    const fetchOrderbookHeaderSpy = vi
      .spyOn(accounts, "fetchOrderbookHeader")
      .mockResolvedValue({
        marketStatus: 1,
        defaultTakerFeeMicro: 500,
        defaultMakerFeeMicro: -100,
      } as Awaited<ReturnType<typeof accounts.fetchOrderbookHeader>>);
    const getGlobalTraderIndexAddressesSpy = vi
      .spyOn(coreHelpers, "getGlobalTraderIndexAddresses")
      .mockResolvedValue(["gti-0"]);
    const getActiveTraderBufferAddressesSpy = vi
      .spyOn(coreHelpers, "getActiveTraderBufferAddresses")
      .mockResolvedValue(["atb-0"]);
    const fetchMock = vi.fn(
      async (_input: RequestInfo | URL, init?: RequestInit) => {
        const payload = JSON.parse(String(init?.body)) as {
          method: string;
          params: unknown[];
        };

        if (payload.method === "getAccountInfo") {
          return new Response(
            JSON.stringify({
              result: {
                context: { slot: 415023824 },
                value: {
                  data: [
                    Buffer.from(globalConfigBytes).toString("base64"),
                    "base64",
                  ],
                },
              },
            }),
            { status: 200, headers: { "content-type": "application/json" } }
          );
        }

        if (payload.method === "getMultipleAccounts") {
          expect(payload.params[0]).toHaveLength(2);
          return new Response(
            JSON.stringify({
              result: {
                context: { slot: 415023825 },
                value: [
                  {
                    data: [
                      Buffer.from(globalConfigBytes).toString("base64"),
                      "base64",
                    ],
                  },
                  {
                    data: [
                      Buffer.from(perpAssetMapBytes).toString("base64"),
                      "base64",
                    ],
                  },
                ],
              },
            }),
            { status: 200, headers: { "content-type": "application/json" } }
          );
        }

        return new Response("unexpected", { status: 500 });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const snapshot = await metadataShared.loadRpcSnapshot(
      "https://rpc.example.com",
      {
        getGlobalConfigurationAddress: async () =>
          address("2zskx2iyCvb6Stg7RBZkt1f6MrF4dpYtMG3yMvKwqtUZ"),
        getProgramAddress: () =>
          address("EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"),
        getSplineCollectionAddress: async () =>
          address("8g4T9V6nVgZQ5h8fhQF9qvPbR6U1sAb2YtLr3Zu6zJvN"),
      } as never
    );

    expect(snapshot.slot).toBe(415023825n);
    expect(snapshot.slotIndex).toBe(0);
    expect(snapshot.markets.length).toBeGreaterThan(0);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchOrderbookHeaderSpy).toHaveBeenCalled();
    expect(getGlobalTraderIndexAddressesSpy).toHaveBeenCalledOnce();
    expect(getActiveTraderBufferAddressesSpy).toHaveBeenCalledOnce();
  });

  it("builds commodity metadata from RPC PerpAssetMap fields", () => {
    const commodityMetadata = metadataShared.toCommodityMetadata({
      metadata: {
        assetFlags: {
          bits: 0b101,
          isCommodity: true,
          isCommoditiesReopen: false,
          isCommoditiesAfterHours: true,
        },
        commoditiesAfterHoursRadius: 150n,
        lastKnownIndexPrice: 742500n,
        lastIndexExpiryTimestamp: 1713200000n,
      } as PerpAssetMetadata,
      tickSize: 1n,
      baseLotsDecimals: 3,
      quoteDecimals: 6,
    });

    expect(commodityMetadata).toEqual({
      isCommodity: true,
      isReopen: false,
      isAfterHours: true,
      status: "afterHours",
      afterHoursRadius: {
        value: 150,
        decimals: 3,
        ui: "0.150",
      },
      lastKnownIndexPrice: {
        value: 742500,
        decimals: 3,
        ui: "742.500",
      },
      markPriceBand: {
        lower: {
          value: 742350,
          decimals: 3,
          ui: "742.350",
        },
        upper: {
          value: 742650,
          decimals: 3,
          ui: "742.650",
        },
      },
      executionPriceBand: {
        lower: {
          value: 742350,
          decimals: 3,
          ui: "742.350",
        },
        upper: {
          value: 742650,
          decimals: 3,
          ui: "742.650",
        },
      },
      lastIndexExpiryTimestamp: 1713200000,
    });
  });

  it("exposes a reactive exchange store with derived market indexes", async () => {
    const snapshot = buildSnapshot();
    snapshot.markets.push({
      ...snapshot.markets[0]!,
      symbol: "BTC-PERP",
      assetId: 2,
      marketPubkey: "btc-market",
      splinePubkey: "btc-spline",
      marketStatus: "closed",
    });

    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(snapshot), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    expect(client.exchange.store.getState().health).toBe("cold");
    expect(client.exchange.store.getState().source.priority).toBe("api");

    await client.exchange.ready();

    const state = client.exchange.store.getState();
    expect(state.health).toBe("bootstrapped");
    expect(state.snapshot?.exchange.globalConfig).toBe("global-config");
    expect(state.marketsBySymbol["SOL-PERP"]?.splinePubkey).toBe("sol-spline");
    expect(state.marketsByAssetId[1]?.symbol).toBe("SOL-PERP");
    expect(state.marketsByPubkey["btc-market"]?.symbol).toBe("BTC-PERP");
    expect(state.marketSymbols).toEqual(["BTC-PERP", "SOL-PERP"]);
    expect(state.activeMarketSymbols).toEqual(["SOL-PERP"]);
    expect(state.closedMarketSymbols).toEqual(["BTC-PERP"]);
    expect(state.marketStatusBySymbol["BTC-PERP"]?.isClosed).toBe(true);

    client.dispose();
  });

  it("aborts the API exchange stream when the metadata cache is closed", async () => {
    const { queues, exchange } = createExchangePort();
    const metadata = createPhoenixExchangeMetadata({
      api: {
        api: {
          getSnapshot: async () => buildSnapshot(),
        },
        exchange,
      },
    });

    await metadata.ready();
    await waitUntil(() => queues.length === 1);

    metadata.close();

    await waitUntil(() => queues[0]?.signal?.aborted === true);
  });

  it("refreshes and stays live after a sequence gap without requiring RPC", async () => {
    let httpCalls = 0;
    const initialSnapshot = buildSnapshot();
    const recoveredSnapshot = {
      ...buildSnapshot(),
      slot: 5n,
    };
    const snapshots = [initialSnapshot, recoveredSnapshot];
    const { queues, exchange } = createExchangePort();

    const metadata = createPhoenixExchangeMetadata({
      api: {
        api: {
          getSnapshot: async () => {
            httpCalls += 1;
            return snapshots.shift() ?? recoveredSnapshot;
          },
        },
        exchange,
      },
    });

    await metadata.ready();
    await waitUntil(() => queues.length === 1);

    queues[0]!.push(
      buildSnapshotMsg(20n, {
        ...buildSnapshot(),
        slot: 2n,
        slotIndex: 1,
      })
    );

    await waitUntil(() => metadata.health() === "live");
    expect(metadata.snapshot().sequenceNumber).toBe(20n);

    queues[0]!.push(buildOpenInterestCapDelta(22n, 3n, 0, 9_000n));

    await waitUntil(() => queues[0]?.refreshCount === 1);
    expect(httpCalls).toBe(1);

    queues[0]!.push(
      buildSnapshotMsg(30n, {
        ...buildSnapshot(),
        slot: 6n,
        slotIndex: 0,
      })
    );

    await waitUntil(() => metadata.health() === "live");
    expect(metadata.snapshot().sequenceNumber).toBe(30n);
    expect(httpCalls).toBe(1);

    metadata.close();
  });

  it("reinstalls API snapshots with bigint market fields after a stream restart", async () => {
    let httpCalls = 0;
    const restartedSnapshot = {
      ...buildSnapshot(),
      slot: 3n,
      slotIndex: 0,
    };
    const { queues, exchange } = createExchangePort();
    const onBackgroundError = vi.fn();

    const metadata = createPhoenixExchangeMetadata({
      api: {
        api: {
          getSnapshot: async () => {
            httpCalls += 1;
            return httpCalls === 1 ? buildSnapshot() : restartedSnapshot;
          },
        },
        exchange,
      },
      onBackgroundError,
    });

    await metadata.ready();
    await waitUntil(() => queues.length === 1);

    queues[0]!.push(
      buildSnapshotMsg(20n, {
        ...buildSnapshot(),
        slot: 2n,
        slotIndex: 1,
      })
    );

    await waitUntil(() => metadata.health() === "live");
    queues[0]!.close();

    await waitUntil(() => httpCalls === 2);
    await waitUntil(() => metadata.snapshot().slot === 3n);

    expect(onBackgroundError).not.toHaveBeenCalled();

    metadata.close();
  });

  it("builds order packets from cached exchange metadata through the client", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    const limitPacket = await client.orderPackets.buildLimitOrderPacket({
      symbol: "SOL-PERP",
      side: Side.Bid,
      priceUsd: "135.87",
      baseUnits: "0.25",
    });

    const marketPacket = await client.orderPackets.buildMarketOrderPacket({
      symbol: "SOL-PERP",
      side: Side.Ask,
      baseUnits: "0.25",
      priceLimitUsd: "135.80",
    });

    expect(limitPacket.priceInTicks).toEqual(ticks(1358n));
    expect(limitPacket.numBaseLots).toEqual(baseLots(250n));
    expect(marketPacket.priceInTicks).toEqual(ticks(1358n));
    expect(marketPacket.numBaseLots).toEqual(baseLots(250n));
    expect(fetchMock).toHaveBeenCalledTimes(1);

    client.dispose();
  });

  it("builds place-limit ixs from cached exchange metadata through client.ixs", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    const ix = await client.ixs.placeLimitOrder({
      authority: address("11111111111111111111111111111111"),
      symbol: "SOL-PERP",
      orderPacket: {
        side: Side.Bid,
        priceInTicks: ticks(1000n),
        numBaseLots: baseLots(5n),
        selfTradeBehavior: 1,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: 0,
        cancelExisting: false,
      },
    });

    expect(ix.accounts[3]?.address).toBe("11111111111111111111111111111111");
    expect(ix.accounts.at(-2)?.address).toBe("sol-market");
    expect(ix.accounts.at(-1)?.address).toBe("sol-spline");
    expect(fetchMock).toHaveBeenCalledTimes(1);

    client.dispose();
  });

  it("builds cancel-all ixs from cached exchange metadata through client.ixs", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    const ix = await client.ixs.buildCancelAll({
      authority: address("11111111111111111111111111111111"),
      symbol: "SOL-PERP",
    });

    expect(ix.accounts.at(-2)?.address).toBe("sol-market");
    expect(ix.accounts.at(-1)?.address).toBe("sol-spline");
    expect(fetchMock).toHaveBeenCalledTimes(1);

    client.dispose();
  });

  it("builds register-trader ixs through client.ixs without requiring exchange bootstrap", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    const ix = await client.ixs.buildRegisterTrader({
      authority: address("11111111111111111111111111111111"),
      marginType: MarginType.Cross,
    });

    expect(ix.accounts[3]?.address).toBe("11111111111111111111111111111111");
    expect(fetchMock).not.toHaveBeenCalled();

    client.dispose();
  });

  it("builds conditional-orders-account ixs through client.ixs without requiring exchange bootstrap", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });
    const authority = address("11111111111111111111111111111111");
    const traderAccount = await client.pda.getTraderAddress({
      authority,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });
    const conditionalOrdersAddress =
      await client.pda.getConditionalOrdersAddress({
        traderAccount,
      });

    const ix = await client.ixs.buildCreateConditionalOrdersAccount({
      authority,
      capacity: 8,
    });

    expect(ix.accounts[3]?.address).toBe(authority);
    expect(ix.accounts[4]?.address).toBe(authority);
    expect(ix.accounts[5]?.address).toBe(traderAccount);
    expect(ix.accounts[6]?.address).toBe(conditionalOrdersAddress);
    expect(fetchMock).not.toHaveBeenCalled();

    client.dispose();
  });

  it("builds position conditional order ixs from cached exchange metadata through client.ixs", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });
    const authority = address("11111111111111111111111111111111");
    const traderAccount = await client.pda.getTraderAddress({
      authority,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });
    const conditionalOrdersAddress =
      await client.pda.getConditionalOrdersAddress({
        traderAccount,
      });

    const ix = await client.ixs.placePositionConditionalOrder({
      authority,
      symbol: "SOL-PERP",
      greaterTriggerOrder: {
        triggerDirection: Direction.GreaterThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(1100n),
        executionPrice: ticks(1099n),
      },
      sizePercent: 100,
    });

    expect(ix.programAddress).toBe(client.pda.getProgramAddress());
    expect(ix.accounts[3]?.address).toBe(authority);
    expect(ix.accounts[4]?.address).toBe(traderAccount);
    expect(ix.accounts[5]?.address).toBe("perp-asset-map");
    expect(ix.accounts[8]?.address).toBe("sol-market");
    expect(ix.accounts[9]?.address).toBe("sol-spline");
    expect(ix.accounts[10]?.address).toBe(authority);
    expect(ix.accounts[11]?.address).toBe(conditionalOrdersAddress);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    client.dispose();
  });

  it("routes supported order ixs through flight when the client is configured with a fee collector", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const authority = address("11111111111111111111111111111111");
    const builderAuthority = address(
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    );
    const feeCollectorTrader = address(
      "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
    ) as never;

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
      flight: {
        builderAuthority,
        builderPdaIndex: 4,
        builderSubaccountIndex: 7,
      },
    });
    const getTraderAddressSpy = vi
      .spyOn(client.pda, "getTraderAddress")
      .mockResolvedValue(feeCollectorTrader);

    const ix = await client.ixs.placeLimitOrder({
      authority,
      symbol: "SOL-PERP",
      orderPacket: {
        side: Side.Bid,
        priceInTicks: ticks(1000n),
        numBaseLots: baseLots(5n),
        selfTradeBehavior: 1,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: 0,
        cancelExisting: false,
      },
    });

    expect(ix.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(ix.accounts[2]?.address).toBe(builderAuthority);
    expect(ix.accounts[3]?.address).toBe(feeCollectorTrader);
    expect(ix.accounts[5]?.address).toBe(authority);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(getTraderAddressSpy).toHaveBeenCalledWith({
      authority: builderAuthority,
      traderPdaIndex: 4,
      subaccountIndex: 7,
      phoenixProgramAddress: client.pda.getProgramAddress(),
    });

    client.dispose();
  });

  it("routes position conditional order ixs through flight when the client is configured with a fee collector", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const authority = address("11111111111111111111111111111111");
    const builderAuthority = address(
      "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    );
    const feeCollectorTrader = address(
      "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
    ) as never;

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
      flight: {
        builderAuthority,
        builderPdaIndex: 4,
        builderSubaccountIndex: 7,
      },
    });
    const getTraderAddressSpy = vi
      .spyOn(client.pda, "getTraderAddress")
      .mockResolvedValue(feeCollectorTrader);

    const ix = await client.ixs.placePositionConditionalOrder({
      authority,
      symbol: "SOL-PERP",
      greaterTriggerOrder: {
        triggerDirection: Direction.GreaterThan,
        tradeSide: Side.Ask,
        orderKind: StopLossOrderKind.IOC,
        triggerPrice: ticks(1100n),
        executionPrice: ticks(1099n),
      },
      sizePercent: 100,
    });

    expect(ix.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(ix.accounts[2]?.address).toBe(builderAuthority);
    expect(ix.accounts[3]?.address).toBe(feeCollectorTrader);
    expect(ix.accounts[5]?.address).toBe(authority);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(getTraderAddressSpy).toHaveBeenCalledWith({
      authority: builderAuthority,
      traderPdaIndex: 4,
      subaccountIndex: 7,
      phoenixProgramAddress: client.pda.getProgramAddress(),
    });

    client.dispose();
  });

  it("prefers the registered Flight BuilderState trader over the configured PDA derivation", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const authority = address("11111111111111111111111111111111");
    const builderAuthority = address(
      "F952dz4aHVUu75YdxUrGhLejhCfzXCYVDysDw6yL6uT4"
    );
    const registeredBuilderTrader = address(
      "ADvFCTV83VpkDnu69ZW17Grav5xswkcrmYexYHytoVYm"
    ) as never;
    const derivedBuilderTrader = address(
      "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
    ) as never;

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      rpcUrl: "https://rpc.example.com",
      ws: false,
      flight: {
        builderAuthority,
        builderPdaIndex: 4,
        builderSubaccountIndex: 7,
      },
    });
    const getTraderAddressSpy = vi
      .spyOn(client.pda, "getTraderAddress")
      .mockResolvedValue(derivedBuilderTrader);
    vi.spyOn(client.rpc.accounts, "fetchAccount").mockResolvedValue({
      data: loadMockBytes("flight_builder_state.json"),
    });

    const ix = await client.ixs.placeLimitOrder({
      authority,
      symbol: "SOL-PERP",
      orderPacket: {
        side: Side.Bid,
        priceInTicks: ticks(1000n),
        numBaseLots: baseLots(5n),
        selfTradeBehavior: 1,
        matchLimit: null,
        clientOrderId: 0n,
        lastValidSlot: null,
        orderFlags: 0,
        cancelExisting: false,
      },
    });

    expect(ix.programAddress).toBe(flight.FLIGHT_PROGRAM_ADDRESS);
    expect(ix.accounts[2]?.address).toBe(builderAuthority);
    expect(ix.accounts[3]?.address).toBe(registeredBuilderTrader);
    expect(ix.accounts[3]?.address).not.toBe(derivedBuilderTrader);
    expect(getTraderAddressSpy).not.toHaveBeenCalledWith({
      authority: builderAuthority,
      traderPdaIndex: 4,
      subaccountIndex: 7,
      phoenixProgramAddress: client.pda.getProgramAddress(),
    });

    client.dispose();
  });

  it("builds deposit ix bundles from cached exchange metadata through client.ixs", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    const authority = address("11111111111111111111111111111111");
    const feePayer = address("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
    const result = await client.ixs.buildDepositIxs({
      authority,
      amount: 1_000_000n,
      feePayer,
    });

    expect(result.instructions).toHaveLength(3);
    expect(result.named.createPhoenixAta.accounts[0]?.address).toBe(feePayer);
    expect(result.named.createPhoenixAta.accounts[2]?.address).toBe(authority);
    expect(result.named.emberDeposit.accounts[2]?.address).toBe(
      "EPjFWdd5AufqSSqeM2qAEyFnjAu1pzsVbW8UVzA73NA"
    );
    expect(result.named.depositFunds.accounts[3]?.address).toBe(authority);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    client.dispose();
  });

  it("builds withdraw ix bundles from cached exchange metadata through client.ixs", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(stringifyWithBigints(buildSnapshot()), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
    });

    const authority = address("11111111111111111111111111111111");
    const feePayer = address("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
    const result = await client.ixs.buildWithdrawIxs({
      authority,
      amount: 1_000_000n,
      feePayer,
    });

    expect(result.instructions).toHaveLength(5);
    expect(result.named.createPhoenixAta.accounts[0]?.address).toBe(feePayer);
    expect(result.named.approveToken.accounts[1]?.address).toBe(
      result.named.emberWithdraw.accounts[1]?.address
    );
    expect(result.named.withdrawFunds.accounts[3]?.address).toBe(authority);
    expect(result.named.emberWithdraw.accounts[2]?.address).toBe(
      "So11111111111111111111111111111111111111112"
    );
    expect(result.named.emberWithdraw.accounts[3]?.address).toBe(
      "EPjFWdd5AufqSSqeM2qAEyFnjAu1pzsVbW8UVzA73NA"
    );
    expect(fetchMock).toHaveBeenCalledTimes(1);

    client.dispose();
  });

  it("exposes explicit rpc reads through client.rpc", async () => {
    const globalConfigurationBytes = loadMockBytes("global_config.json");
    const orderbookBytes = loadMockBytes("orderbook.json");
    const splineBytes = loadMockBytes("splines.json");
    const perpAssetMapBytes = loadMockBytes("perp_asset_map.json");
    const globalConfiguration = decodeGlobalConfiguration(
      globalConfigurationBytes
    );

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
      rpcUrl: "https://rpc.example.com",
    });
    const globalConfigurationAddress =
      await client.pda.getGlobalConfigurationAddress();

    const fetchMock = vi.fn(
      async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = String(input);
        if (url === "https://example.com/v1/exchange/snapshot") {
          return new Response(stringifyWithBigints(buildSnapshot()), {
            status: 200,
            headers: { "content-type": "application/json" },
          });
        }

        if (url === "https://rpc.example.com") {
          const body = JSON.parse(String(init?.body)) as {
            params: [string, unknown];
          };
          const target = body.params[0];
          const payloadFor = (bytes: Uint8Array) =>
            new Response(
              JSON.stringify({
                jsonrpc: "2.0",
                id: 1,
                result: {
                  value: {
                    data: [Buffer.from(bytes).toString("base64"), "base64"],
                  },
                },
              }),
              {
                status: 200,
                headers: { "content-type": "application/json" },
              }
            );

          if (target === globalConfigurationAddress) {
            return payloadFor(globalConfigurationBytes);
          }
          if (target === globalConfiguration.perpAssetMapKey) {
            return payloadFor(perpAssetMapBytes);
          }
          if (target === "sol-market") {
            return payloadFor(orderbookBytes);
          }
          if (target === "sol-spline") {
            return payloadFor(splineBytes);
          }
        }

        return new Response("not found", { status: 404 });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const loadedGlobalConfiguration =
      await client.rpc.exchange.getGlobalConfiguration();
    const loadedPerpAssetMap = await client.rpc.exchange.getPerpAssetMap();
    const loadedOrderbookHeader =
      await client.rpc.markets.getOrderbookHeader("SOL-PERP");
    const loadedOrderbook = await client.rpc.markets.getOrderbook("SOL-PERP");
    const loadedSplineCollection =
      await client.rpc.markets.getSplineCollection("SOL-PERP");

    expect(client.rpc.available).toBe(true);
    expect(loadedGlobalConfiguration.perpAssetMapKey).toBe(
      globalConfiguration.perpAssetMapKey
    );
    expect(loadedPerpAssetMap.metadata.entries.length).toBeGreaterThan(0);
    expect(loadedOrderbookHeader.baseLotsDecimals).toBeTypeOf("number");
    expect(loadedOrderbook.bids.length).toBeGreaterThanOrEqual(0);
    expect(loadedSplineCollection.numActive).toBeGreaterThanOrEqual(0);

    client.dispose();
  });

  it("falls back to Buffer decoding when atob is unavailable for rpc reads", async () => {
    const globalConfigurationBytes = loadMockBytes("global_config.json");
    const globalConfiguration = decodeGlobalConfiguration(
      globalConfigurationBytes
    );

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      ws: false,
      rpcUrl: "https://rpc.example.com",
    });
    const globalConfigurationAddress =
      await client.pda.getGlobalConfigurationAddress();

    const originalAtob = globalThis.atob;
    vi.stubGlobal("atob", undefined);
    vi.stubGlobal("Buffer", Buffer);

    const fetchMock = vi.fn(
      async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = String(input);
        if (url === "https://rpc.example.com") {
          const body = JSON.parse(String(init?.body)) as {
            params: [string, unknown];
          };
          const target = body.params[0];
          if (target === globalConfigurationAddress) {
            return new Response(
              JSON.stringify({
                jsonrpc: "2.0",
                id: 1,
                result: {
                  value: {
                    data: [
                      Buffer.from(globalConfigurationBytes).toString("base64"),
                      "base64",
                    ],
                  },
                },
              }),
              {
                status: 200,
                headers: { "content-type": "application/json" },
              }
            );
          }
        }

        return new Response("not found", { status: 404 });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const loadedGlobalConfiguration =
      await client.rpc.exchange.getGlobalConfiguration();

    expect(loadedGlobalConfiguration.perpAssetMapKey).toBe(
      globalConfiguration.perpAssetMapKey
    );
    expect(originalAtob).toBeTypeOf("function");

    client.dispose();
  });

  it("emits source and snapshot events from the metadata manager", async () => {
    const events: string[] = [];
    const exchange = createPhoenixExchangeMetadata({
      api: {
        api: {
          getSnapshot: async () => buildSnapshot(),
        },
        liveUpdates: "none",
      },
    });

    exchange.subscribe((event) => events.push(event.type));
    await exchange.ready();

    expect(events).toContain("sourceChanged");
    expect(events).toContain("snapshotApplied");
    exchange.close();
  });
});
