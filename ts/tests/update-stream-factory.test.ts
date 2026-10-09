import { describe, expect, it, vi } from "vitest";
import { z } from "zod";
import { createUpdateStream } from "@/ws/adapters/_utils/updateStreamFactory";
import { createFillsAdapter } from "@/ws/adapters/fills/adapter";
import type { WsClient } from "@/ws/types";

describe("createUpdateStream", () => {
  it("refresh re-subscribes in place without unsubscribing", () => {
    const subscribe = vi.fn<WsClient["subscribe"]>();
    const unsubscribe = vi.fn<WsClient["unsubscribe"]>();
    const ws: WsClient = {
      subscribe,
      unsubscribe,
      registerChannel: () => () => undefined,
      close: () => undefined,
      onServerError: () => () => undefined,
    };

    const streamFactory = createUpdateStream(ws, {
      channel: "exchange",
      schema: z.number(),
      buildKey: () => "exchange",
      buildSubParams: () => ({}),
      processMessage: (message) => message,
    });

    const stream = streamFactory();
    stream.refresh();

    expect(subscribe).toHaveBeenCalledTimes(2);
    expect(unsubscribe).not.toHaveBeenCalled();
  });

  it("uses unique listener keys while preserving the shared routing key", () => {
    const subscribe = vi.fn<WsClient["subscribe"]>();
    const unsubscribe = vi.fn<WsClient["unsubscribe"]>();
    const ws: WsClient = {
      subscribe,
      unsubscribe,
      registerChannel: () => () => undefined,
      close: () => undefined,
      onServerError: () => () => undefined,
    };

    const streamFactory = createUpdateStream(ws, {
      channel: "exchange",
      schema: z.number(),
      buildKey: () => "exchange",
      buildSubParams: () => ({}),
      processMessage: (message) => message,
    });

    streamFactory();
    streamFactory();

    expect(subscribe).toHaveBeenCalledTimes(2);

    const [firstKey, , , firstOptions] = subscribe.mock.calls[0]!;
    const [secondKey, , , secondOptions] = subscribe.mock.calls[1]!;

    expect(firstKey).not.toBe(secondKey);
    expect(firstOptions).toEqual({ routingKey: "exchange" });
    expect(secondOptions).toEqual({ routingKey: "exchange" });
  });

  it("keeps integer-string fill timestamps as bigint and ISO ones as ms", async () => {
    const subscribe = vi.fn<WsClient["subscribe"]>();
    const ws: WsClient = {
      subscribe,
      unsubscribe: () => undefined,
      registerChannel: () => () => undefined,
      close: () => undefined,
      onServerError: () => () => undefined,
    };
    const fill = {
      marketSymbol: "SOL",
      baseQty: "1",
      quoteQty: "100",
      price: "100",
      transactionSignature: "sig",
      instructionType: "swap",
    };

    const iterator = createFillsAdapter(ws)()[Symbol.asyncIterator]();
    subscribe.mock.calls[0]![2]({
      channel: "fills",
      symbol: "SOL",
      fills: [
        { ...fill, timestamp: "1700000000000" },
        { ...fill, timestamp: "2023-11-14T22:13:20.000Z" },
      ],
    });

    expect((await iterator.next()).value.fill.timestamp).toBe(1700000000000n);
    expect((await iterator.next()).value.fill.timestamp).toBe(1700000000000);
  });
});
