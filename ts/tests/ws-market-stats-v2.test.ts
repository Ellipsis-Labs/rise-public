import { createMarketStatsAdapter } from "@/ws/adapters/market-stats/adapter";
import { createMarketStatsV2Adapter } from "@/ws/adapters/market-stats-v2/adapter";
import { createMarketStatsV2Plugin } from "@/ws/adapters/market-stats-v2/plugin";
import { buildMarketStatsV2RoutingKey } from "@/ws/adapters/market-stats-v2/routing";
import type {
  SubscriptionMessage,
  SubscriptionOptions,
  WsClient,
} from "@/ws/types";
import { describe, expect, it, vi } from "vitest";

type CapturedSubscription = {
  subMsg: SubscriptionMessage;
  onMessage: (data: unknown) => void;
  options?: SubscriptionOptions;
};

const createMockWsClient = () => {
  const subscriptions: CapturedSubscription[] = [];
  const ws: WsClient = {
    subscribe(_key, subMsg, onMessage, options) {
      subscriptions.push({ subMsg, onMessage, options });
    },
    unsubscribe() {},
    registerChannel() {
      return () => {};
    },
    close() {},
    onServerError() {
      return () => {};
    },
  };
  return { ws, subscriptions };
};

const rawStats = (symbol: string) => ({
  symbol,
  timestamp: 1,
  openInterest: 2,
  markPrice: 3,
  midPrice: 3.5,
  oraclePrice: 4,
  prevDayMarkPrice: 5,
  dayVolumeUsd: 6,
  dayVolumeBase: 7,
  currentFundingRate: 8,
  eightHourFundingRate: 9,
  annualizedFundingRate: 10,
});

describe("marketStatsV2 websocket adapter", () => {
  it("backs the V1-compatible stream with V2 and flattens each batch", async () => {
    const { ws, subscriptions } = createMockWsClient();
    const marketStats = createMarketStatsAdapter(ws);
    const stream = marketStats();
    const iterator = stream[Symbol.asyncIterator]();
    const first = iterator.next();

    expect(subscriptions[0]!.subMsg).toEqual({
      type: "subscribe",
      subscription: { channel: "marketStatsV2" },
    });
    expect(subscriptions[0]!.options?.routingKey).toBe("marketStatsV2");

    subscriptions[0]!.onMessage({
      channel: "marketStatsV2",
      stats: [rawStats("BTC-PERP"), rawStats("SOL-PERP")],
    });

    await expect(first).resolves.toMatchObject({
      done: false,
      value: { symbol: "BTC-PERP" },
    });
    await expect(iterator.next()).resolves.toMatchObject({
      done: false,
      value: { symbol: "SOL-PERP" },
    });
  });

  it("supports all, one, and multiple market subscriptions", () => {
    const { ws, subscriptions } = createMockWsClient();
    const marketStatsV2 = createMarketStatsV2Adapter(ws);

    marketStatsV2();
    marketStatsV2("sol-perp");
    marketStatsV2(["SOL-PERP", "btc-perp", "SOL-PERP"]);

    expect(subscriptions.map(({ subMsg }) => subMsg)).toEqual([
      {
        type: "subscribe",
        subscription: { channel: "marketStatsV2" },
      },
      {
        type: "subscribe",
        subscription: { channel: "marketStatsV2", symbols: ["SOL-PERP"] },
      },
      {
        type: "subscribe",
        subscription: {
          channel: "marketStatsV2",
          symbols: ["BTC-PERP", "SOL-PERP"],
        },
      },
    ]);
    expect(subscriptions.map(({ options }) => options?.routingKey)).toEqual([
      "marketStatsV2",
      'marketStatsV2:["SOL-PERP"]',
      'marketStatsV2:["BTC-PERP","SOL-PERP"]',
    ]);
  });

  it("rejects an empty symbol list", () => {
    const { ws } = createMockWsClient();
    const marketStatsV2 = createMarketStatsV2Adapter(ws);

    expect(() => marketStatsV2([])).toThrow("symbols must not be empty");
    expect(() => marketStatsV2("   ")).toThrow(
      "symbols must not contain empty values"
    );
  });

  it("delivers a full batch as one update", async () => {
    const { ws, subscriptions } = createMockWsClient();
    const marketStatsV2 = createMarketStatsV2Adapter(ws);
    const stream = marketStatsV2(["SOL-PERP", "BTC-PERP"]);
    const next = stream[Symbol.asyncIterator]().next();

    subscriptions[0]!.onMessage({
      channel: "marketStatsV2",
      symbols: ["BTC-PERP", "SOL-PERP"],
      stats: [rawStats("BTC-PERP"), rawStats("SOL-PERP")],
    });

    await expect(next).resolves.toEqual({
      done: false,
      value: {
        symbols: ["BTC-PERP", "SOL-PERP"],
        stats: [
          {
            symbol: "BTC-PERP",
            stats: {
              timestamp: 1000n,
              openInterest: 2,
              markPrice: 3,
              midPrice: 3.5,
              oraclePrice: 4,
              prevDayMarkPrice: 5,
              dayVolumeUsd: 6,
              dayVolumeBase: 7,
              currentFundingRate: 8,
              eightHourFundingRate: 9,
              annualizedFundingRate: 10,
            },
          },
          {
            symbol: "SOL-PERP",
            stats: {
              timestamp: 1000n,
              openInterest: 2,
              markPrice: 3,
              midPrice: 3.5,
              oraclePrice: 4,
              prevDayMarkPrice: 5,
              dayVolumeUsd: 6,
              dayVolumeBase: 7,
              currentFundingRate: 8,
              eightHourFundingRate: 9,
              annualizedFundingRate: 10,
            },
          },
        ],
      },
    });
  });

  it("routes all and filtered batches independently", async () => {
    const plugin = createMarketStatsV2Plugin();
    const all = vi.fn();
    const filtered = vi.fn();
    const registry = new Map([
      ["marketStatsV2", { onMsg: all }],
      [buildMarketStatsV2RoutingKey(["SOL-PERP"]), { onMsg: filtered }],
    ]);

    await plugin.handle(
      { channel: "marketStatsV2", stats: [rawStats("SOL-PERP")] },
      registry
    );
    await plugin.handle(
      {
        channel: "marketStatsV2",
        symbols: ["SOL-PERP"],
        stats: [rawStats("SOL-PERP")],
      },
      registry
    );

    expect(all).toHaveBeenCalledTimes(1);
    expect(filtered).toHaveBeenCalledTimes(1);
  });

  it("rejects malformed routing filters without throwing", () => {
    const plugin = createMarketStatsV2Plugin();

    expect(
      plugin.validate({
        channel: "marketStatsV2",
        symbols: ["   "],
        stats: [],
      })
    ).toBe(false);
    expect(
      plugin.validate({
        channel: "marketStatsV2",
        symbols: [],
        stats: [],
      })
    ).toBe(false);
  });
});
