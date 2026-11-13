import { describe, expect, it, vi } from "vitest";
import * as z from "zod";
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
      onSubscriptionStatus: () => () => undefined,
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
      onSubscriptionStatus: () => () => undefined,
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
    expect(firstOptions?.routingKey).toBe("exchange");
    expect(secondOptions?.routingKey).toBe("exchange");
  });

  it.each(["ended", "rejected"] as const)(
    "delivers a message still in flight before the subscription is %s",
    async (completion) => {
      const subscribe = vi.fn<WsClient["subscribe"]>();
      const ws: WsClient = {
        subscribe,
        unsubscribe: () => undefined,
        registerChannel: () => () => undefined,
        close: () => undefined,
        onServerError: () => () => undefined,
      };

      const gate = Promise.withResolvers<void>();
      const streamFactory = createUpdateStream(ws, {
        channel: "exchange",
        schema: z.number(),
        buildKey: () => "exchange",
        buildSubParams: () => ({}),
        processMessage: async (message) => {
          await gate.promise;
          return message;
        },
      });

      const stream = streamFactory();
      const iterator = stream[Symbol.asyncIterator]();
      const [, , onMessage, options] = subscribe.mock.calls[0]!;
      onMessage(7);
      const error = { code: 403, error: "subscription denied" };
      if (completion === "rejected") options?.onError?.(error);
      options?.onClose?.();
      stream.refresh();
      expect(subscribe).toHaveBeenCalledTimes(1);

      const delivered = iterator.next();
      gate.resolve();

      await expect(delivered).resolves.toEqual({ value: 7, done: false });
      if (completion === "rejected") {
        await expect(iterator.next()).rejects.toMatchObject({
          ...error,
          message: error.error,
        });
      } else {
        await expect(iterator.next()).resolves.toEqual({
          value: undefined,
          done: true,
        });
      }
    }
  );

  it("rejects a waiting consumer with the server code and reason", async () => {
    const subscribe = vi.fn<WsClient["subscribe"]>();
    const ws: WsClient = {
      subscribe,
      unsubscribe: () => undefined,
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

    const iterator = streamFactory()[Symbol.asyncIterator]();
    const rejected = expect(iterator.next()).rejects.toMatchObject({
      code: 400,
      error: "unknown market",
      message: "unknown market",
    });
    const options = subscribe.mock.calls[0]![3];
    options?.onError?.(
      { code: 400, error: "unknown market" },
      { terminal: true }
    );
    options?.onClose?.();

    await rejected;
  });

  it("drains buffered updates before rejecting with the server error", async () => {
    const subscribe = vi.fn<WsClient["subscribe"]>();
    const ws: WsClient = {
      subscribe,
      unsubscribe: () => undefined,
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

    const iterator = streamFactory()[Symbol.asyncIterator]();
    const [, , onMessage, options] = subscribe.mock.calls[0]!;
    onMessage(7);
    onMessage(8);
    // Let synchronous processing results reach the queue before rejection.
    await Promise.resolve();
    options?.onError?.({ code: 403, error: "subscription revoked" });
    options?.onClose?.();

    await expect(iterator.next()).resolves.toEqual({ value: 7, done: false });
    await expect(iterator.next()).resolves.toEqual({ value: 8, done: false });
    await expect(iterator.next()).rejects.toMatchObject({
      code: 403,
      message: "subscription revoked",
    });
  });

  it("keeps an acknowledged stream alive after a nonterminal refresh error", async () => {
    const subscribe = vi.fn<WsClient["subscribe"]>();
    const ws: WsClient = {
      subscribe,
      unsubscribe: () => undefined,
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
    const iterator = stream[Symbol.asyncIterator]();
    const [, , onMessage, options] = subscribe.mock.calls[0]!;
    stream.refresh();
    options?.onError?.(
      { code: 429, error: "refresh rate limited" },
      { terminal: false }
    );
    const delivered = iterator.next();
    onMessage(7);

    await expect(delivered).resolves.toEqual({ value: 7, done: false });
    stream.refresh();
    expect(subscribe).toHaveBeenCalledTimes(3);
    options?.onClose?.();
    await expect(iterator.next()).resolves.toEqual({
      value: undefined,
      done: true,
    });
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
