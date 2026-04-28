import { describe, expect, it, vi } from "vitest";
import { z } from "zod";
import { createUpdateStream } from "@/ws/adapters/_utils/updateStreamFactory";
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
});
