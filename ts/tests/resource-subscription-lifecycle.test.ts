import { describe, expect, it, vi } from "vitest";
import {
  createPhoenixMarketData,
  createPhoenixOrderbookManager,
  createPhoenixTraderStateManager,
  type TraderStateSnapshotResponse,
} from "@/index";

const flush = async () => {
  for (let n = 0; n < 30; n++) await Promise.resolve();
};
const probeStream = () => {
  const signals: AbortSignal[] = [];
  const finishers: Array<() => void> = [];
  const stream = async function* (signal?: AbortSignal) {
    if (!signal) throw new Error("Probe needs an abort signal");
    signals.push(signal);
    await new Promise<void>((resolve) => {
      finishers.push(resolve);
      if (signal.aborted) resolve();
      else signal.addEventListener("abort", () => resolve(), { once: true });
    });
  };
  return {
    signals,
    stream,
    finish: () => finishers.forEach((finish) => finish()),
  };
};
const capability = { immediate: true, viaColdActivation: false };
const traderSnapshot: TraderStateSnapshotResponse = {
  authority: "authority-1",
  traderPdaIndex: 0,
  slot: 1,
  slotIndex: 0,
  snapshot: {
    version: 1,
    makerFeeOverrideMultiplier: 1,
    takerFeeOverrideMultiplier: 1,
    capabilities: {
      flags: 1,
      state: "active",
      capabilities: {
        placeLimitOrder: capability,
        placeMarketOrder: capability,
        riskIncreasingTrade: capability,
        riskReducingTrade: capability,
        depositCollateral: capability,
        withdrawCollateral: capability,
      },
    },
    subaccounts: [],
  },
};

describe("resource subscription lifecycle", () => {
  it("restarts trader subscriptions after immediate release/re-retain", async () => {
    const port = probeStream();
    const manager = createPhoenixTraderStateManager({
      api: { getTraderStateSnapshot: async () => traderSnapshot },
      traderState: (_authority, _index, signal) => port.stream(signal),
    });
    const resource = manager.resource({
      authority: "authority-1",
      traderPdaIndex: 0,
    });
    const release = resource.retain();
    await resource.ready();
    await flush();
    expect(port.signals).toHaveLength(1);
    release();
    const releaseAgain = resource.retain();
    await resource.ready();
    await flush();
    expect(port.signals).toHaveLength(2);
    expect(port.signals.filter((signal) => !signal.aborted)).toHaveLength(1);
    releaseAgain();
    manager.close();
    port.finish();
  });
  it("restarts cached orderbooks after immediate release/re-retain", async () => {
    const port = probeStream();
    const manager = createPhoenixOrderbookManager({
      api: {
        getOrderbook: async () => ({
          symbol: "kBONK",
          slot: 1,
          bids: [],
          asks: [],
        }),
      },
      l2Book: (_symbol, _options, signal) => port.stream(signal),
    });
    const resource = manager.resource("KBONK");
    const release = resource.retain();
    await resource.ready();
    await flush();
    expect(port.signals).toHaveLength(1);
    release();
    const releaseAgain = resource.retain();
    await resource.ready();
    await flush();
    expect(port.signals).toHaveLength(2);
    expect(port.signals.filter((signal) => !signal.aborted)).toHaveLength(1);
    releaseAgain();
    manager.close();
    port.finish();
  });
  it("keeps ready feeds active after releasing an explicit retain", async () => {
    const port = probeStream();
    const marketData = createPhoenixMarketData({ allMids: port.stream });
    const release = marketData.retain();
    await marketData.ready();
    await flush();
    release();
    await flush();
    expect(port.signals).toHaveLength(1);
    expect(port.signals[0].aborted).toBe(false);
    marketData.close();
    expect(port.signals[0].aborted).toBe(true);
    port.finish();
  });
  it.each(["allMids", "marketStats", "markPrice"] as const)(
    "preserves replacement %s stream ownership during reconnect",
    async (feed) => {
      const port = probeStream();
      const marketData = createPhoenixMarketData({
        allMids: feed === "allMids" ? port.stream : undefined,
        marketStats:
          feed === "marketStats"
            ? (_symbol, signal) => port.stream(signal)
            : undefined,
        markPrice:
          feed === "markPrice"
            ? (_symbol, signal) => port.stream(signal)
            : undefined,
      });
      marketData.store.setState({ symbols: ["kBONK"] });
      const release = marketData.retain();
      await flush();
      expect(port.signals).toHaveLength(1);
      marketData.reconnect();
      await flush();
      expect(port.signals).toHaveLength(2);
      expect(port.signals[0].aborted).toBe(true);
      release();
      marketData.close();
      await flush();
      expect(port.signals[1].aborted).toBe(true);
      port.finish();
    }
  );
});

it.each(["store", "resource"] as const)(
  "%s ready starts feeds without retain and keeps one stream across repeated calls",
  async (target) => {
    const port = probeStream();
    const marketData = createPhoenixMarketData({ allMids: port.stream });
    const resource =
      target === "store" ? marketData : marketData.resource("KBONK");
    await resource.ready();
    await resource.ready();
    expect(port.signals).toHaveLength(1);
    expect(port.signals[0].aborted).toBe(false);
    marketData.reconnect();
    await flush();
    expect(port.signals).toHaveLength(2);
    expect(port.signals[0].aborted).toBe(true);
    expect(port.signals[1].aborted).toBe(false);
    marketData.close();
    expect(port.signals[1].aborted).toBe(true);
    await expect(resource.ready()).rejects.toThrow("closed");
  }
);

it("reconnect starts feeds without retain and close stops them", async () => {
  const port = probeStream();
  const marketData = createPhoenixMarketData({ allMids: port.stream });
  marketData.reconnect();
  expect(port.signals).toHaveLength(1);
  expect(port.signals[0].aborted).toBe(false);
  marketData.close();
  expect(port.signals[0].aborted).toBe(true);
});

it.each(["allMids", "marketStats", "markPrice"] as const)(
  "does not revive retired %s retries after reconnect",
  async (feed) => {
    vi.useFakeTimers();
    const port = probeStream();
    const marketData = createPhoenixMarketData({
      allMids: feed === "allMids" ? port.stream : undefined,
      marketStats:
        feed === "marketStats"
          ? (_symbol, signal) => port.stream(signal)
          : undefined,
      markPrice:
        feed === "markPrice"
          ? (_symbol, signal) => port.stream(signal)
          : undefined,
      resyncBackoffMs: 100,
    });
    try {
      marketData.store.setState({ symbols: ["kBONK"] });
      const release = marketData.retain();
      port.finish();
      await flush();
      marketData.reconnect();
      expect(port.signals).toHaveLength(2);
      await vi.advanceTimersByTimeAsync(100);
      expect(port.signals).toHaveLength(2);
      release();
      expect(port.signals.every((signal) => signal.aborted)).toBe(true);
    } finally {
      marketData.close();
      port.finish();
      vi.useRealTimers();
    }
  }
);
