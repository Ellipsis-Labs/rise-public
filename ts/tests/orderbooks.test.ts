import { describe, expect, it } from "vitest";
import { createPhoenixOrderbookManager, type L2BookUpdate } from "@/index";

const createL2BookPort = () => {
  const queues: Array<{
    symbol: string;
    bypassExecutionBand?: boolean;
    push: (message: L2BookUpdate) => boolean;
    close: () => void;
  }> = [];

  return {
    queues,
    l2Book: (
      symbol: string,
      optionsOrSignal?: { bypassExecutionBand?: boolean } | AbortSignal,
      signal?: AbortSignal
    ) => {
      const options =
        optionsOrSignal instanceof AbortSignal ? undefined : optionsOrSignal;
      const resolvedSignal =
        optionsOrSignal instanceof AbortSignal ? optionsOrSignal : signal;
      const values: L2BookUpdate[] = [];
      const waiters: Array<(result: IteratorResult<L2BookUpdate>) => void> = [];
      let closed = false;

      const queue = {
        push(message: L2BookUpdate) {
          if (closed) return false;
          const waiter = waiters.shift();
          if (waiter) {
            waiter({ value: message, done: false });
            return true;
          }
          values.push(message);
          return true;
        },
        close() {
          if (closed) return;
          closed = true;
          while (waiters.length > 0) {
            waiters.shift()?.({ value: undefined, done: true });
          }
        },
        [Symbol.asyncIterator]() {
          return {
            next: () => {
              if (values.length > 0) {
                return Promise.resolve({
                  value: values.shift()!,
                  done: false,
                });
              }
              if (closed) {
                return Promise.resolve({ value: undefined, done: true });
              }
              return new Promise<IteratorResult<L2BookUpdate>>((resolve) => {
                waiters.push(resolve);
              });
            },
            return: async () => {
              queue.close();
              return { value: undefined, done: true };
            },
          };
        },
      };

      resolvedSignal?.addEventListener("abort", () => queue.close(), {
        once: true,
      });
      const entry = {
        symbol,
        bypassExecutionBand: options?.bypassExecutionBand,
        push: (message: L2BookUpdate) => queue.push(message),
        close: () => queue.close(),
      };
      queues.push(entry);
      return queue;
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

describe("orderbook manager", () => {
  it("bootstraps a resource from the HTTP snapshot and exposes derived state", async () => {
    const manager = createPhoenixOrderbookManager({
      api: {
        getOrderbook: async (symbol) => ({
          symbol,
          slot: 1,
          timestamp: 1_700_000_000,
          bids: [[100, 2]],
          asks: [[102, 3]],
          mid: 101,
        }),
      },
    });

    const resource = manager.resource("sol-perp");

    expect(resource.store.getState().status.health).toBe("cold");

    const book = await resource.ready();
    expect(book?.symbol).toBe("SOL-PERP");
    expect(book?.bypassExecutionBand).toBe(false);
    expect(book?.bestBid?.price).toBe(100);
    expect(book?.bestAsk?.price).toBe(102);
    expect(book?.mid).toBe(101);
    expect(book?.spread).toBe(2);

    const state = resource.store.getState();
    expect(state.bypassExecutionBand).toBe(false);
    expect(state.status.health).toBe("bootstrapped");
    expect(state.bids[0]?.size).toBe(2);
    expect(state.asks[0]?.size).toBe(3);

    manager.close();
  });

  it("streams live orderbook updates through retained resources", async () => {
    const { queues, l2Book } = createL2BookPort();
    const manager = createPhoenixOrderbookManager({
      api: {
        getOrderbook: async (symbol) => ({
          symbol,
          slot: 1,
          timestamp: 1_700_000_000,
          bids: [[100, 2]],
          asks: [[102, 3]],
          mid: 101,
        }),
      },
      l2Book,
    });

    const resource = manager.resource("SOL-PERP");
    const release = resource.retain();

    await resource.ready();
    await waitUntil(() => queues.length > 0);

    queues.at(-1)!.push({
      market: "SOL-PERP",
      ts: 1_700_000_500,
      slot: 2n,
      bids: [[101, 4]],
      asks: [[103, 5]],
    });

    await waitUntil(() => resource.store.getState().status.health === "live");

    const state = resource.store.getState();
    expect(state.status.isConnected).toBe(true);
    expect(state.bestBid?.price).toBe(101);
    expect(state.bestAsk?.price).toBe(103);
    expect(state.mid).toBe(102);
    expect(state.spread).toBe(2);

    release();
    expect(resource.store.getState().status.isConnected).toBe(false);

    manager.close();
  });

  it("keeps bypassExecutionBand resources distinct and threads the flag through bootstrap and stream", async () => {
    const apiCalls: Array<{
      symbol: string;
      bypassExecutionBand?: boolean;
    }> = [];
    const { queues, l2Book } = createL2BookPort();
    const manager = createPhoenixOrderbookManager({
      api: {
        getOrderbook: async (symbol, options) => {
          apiCalls.push({
            symbol,
            bypassExecutionBand: options?.bypassExecutionBand,
          });
          return {
            symbol,
            slot: 1,
            timestamp: 1_700_000_000,
            bids: options?.bypassExecutionBand ? [[100, 8]] : [[100, 2]],
            asks: options?.bypassExecutionBand ? [[102, 9]] : [[102, 3]],
            mid: 101,
          };
        },
      },
      l2Book,
    });

    const filtered = manager.resource("SOL-PERP");
    const bypassed = manager.resource("SOL-PERP", {
      bypassExecutionBand: true,
    });
    const release = bypassed.retain();

    expect(filtered).not.toBe(bypassed);
    expect(filtered.key).toBe("orderbook:SOL-PERP");
    expect(bypassed.key).toBe("orderbook:SOL-PERP:bypassExecutionBand");
    expect(manager.peek("SOL-PERP")).toBe(filtered);
    expect(manager.peek("SOL-PERP", { bypassExecutionBand: true })).toBe(
      bypassed
    );

    const book = await bypassed.ready();

    expect(apiCalls).toEqual([
      { symbol: "SOL-PERP", bypassExecutionBand: true },
    ]);
    expect(book?.bypassExecutionBand).toBe(true);
    expect(book?.bids[0]?.size).toBe(8);
    expect(bypassed.store.getState().bypassExecutionBand).toBe(true);

    await waitUntil(() => queues.length > 0);
    expect(queues.at(-1)?.bypassExecutionBand).toBe(true);

    queues.at(-1)!.push({
      market: "SOL-PERP",
      ts: 1_700_000_500,
      slot: 2n,
      bids: [[101, 10]],
      asks: [[103, 11]],
      bypassExecutionBand: true,
    });

    await waitUntil(() => bypassed.store.getState().status.health === "live");
    expect(bypassed.snapshot()?.bypassExecutionBand).toBe(true);
    expect(bypassed.store.getState().bestBid?.size).toBe(10);

    release();
    manager.close();
  });
});
