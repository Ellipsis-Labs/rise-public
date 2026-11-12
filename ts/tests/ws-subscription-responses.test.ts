import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPhoenixWsClient } from "@/index";
import { createWsClient } from "@/ws/WsClient";
import type {
  SubscriptionMessage,
  WsClient,
  WsClientOpts,
  WsServerErrorMessage,
  WsSubscriptionStatusMessage,
} from "@/ws/types";

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  readyState = 0;
  readonly sent: string[] = [];
  readonly closes: Array<{ code?: number; reason?: string }> = [];
  onopen: (() => void) | null = null;
  onmessage: ((evt: { data: string }) => void) | null = null;
  onclose: ((evt: { code: number; reason?: string }) => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(
    public url: string,
    public protocol?: string | string[]
  ) {
    MockWebSocket.instances.push(this);
  }

  open() {
    this.readyState = 1;
    this.onopen?.();
  }

  send(payload: string) {
    this.sent.push(payload);
  }

  close(code?: number, reason?: string) {
    this.closes.push({ code, reason });
    this.readyState = 3;
    this.onclose?.({ code: code ?? 1000, reason });
  }

  receive(frame: unknown) {
    this.onmessage?.({ data: JSON.stringify(frame) });
  }

  sentMessages(): SubscriptionMessage[] {
    return this.sent.map(
      (payload) => JSON.parse(payload) as SubscriptionMessage
    );
  }
}

const AUTHORITY_A = "AuthorityA1111111111111111111111111111111111";
const AUTHORITY_B = "AuthorityB1111111111111111111111111111111111";

const traderStateRequest = (authority: string) => ({
  channel: "traderState",
  authority,
  traderPdaIndex: 0,
});

const traderStateKey = (authority: string) => `traderState:${authority}:0`;

const traderStateData = (authority: string, slot: number) => ({
  channel: "traderState",
  authority,
  traderPdaIndex: 0,
  slot,
});

const status = (
  statusValue: "subscribed" | "unsubscribed",
  subscription: Record<string, unknown>
) => ({
  channel: "subscriptionStatus",
  status: statusValue,
  subscription,
  clientId: "client-1",
});

const error = (
  message: string,
  code: number,
  subscription?: Record<string, unknown>
) => ({
  channel: "error",
  error: message,
  code,
  ...(subscription ? { subscription } : {}),
});

type Event =
  | { kind: "status"; status: string; subscription: Record<string, unknown> }
  | {
      kind: "error";
      code: number;
      error: string;
      subscription?: Record<string, unknown>;
    };

type Stream = {
  log: Array<unknown>;
  errors: Array<{ code: number; error: string }>;
  closed: number;
};

const flush = async () => {
  await Promise.resolve();
  await Promise.resolve();
  await vi.advanceTimersByTimeAsync(0);
};

const setup = async (opts: Partial<WsClientOpts> = {}) => {
  const events: Event[] = [];
  const client = createWsClient({
    url: "wss://example.com/v1/ws",
    authMode: "anonymous",
    connectMode: "eager",
    backoff: { baseMs: 0, maxMs: 0 },
    heartbeat: false,
    onSubscriptionStatus: (message: WsSubscriptionStatusMessage) =>
      events.push({
        kind: "status",
        status: message.status,
        subscription: message.subscription,
      }),
    onServerError: (message: WsServerErrorMessage) =>
      events.push({
        kind: "error",
        code: message.code,
        error: message.error,
        ...(message.subscription ? { subscription: message.subscription } : {}),
      }),
    ...opts,
  });
  await flush();
  const socket = MockWebSocket.instances.at(-1)!;
  socket.open();
  return { client, socket, events };
};

const listen = (
  client: WsClient,
  key: string,
  subscription: Record<string, unknown>,
  routingKey = key
): Stream => {
  const stream: Stream = { log: [], errors: [], closed: 0 };
  client.subscribe(
    `${key}::listener`,
    { type: "subscribe", subscription },
    (data) => stream.log.push(data),
    {
      routingKey,
      onError: (err) => {
        stream.errors.push(err);
        stream.log.push({ error: err.code });
      },
      onClose: () => {
        stream.closed += 1;
        stream.log.push("closed");
      },
    }
  );
  return stream;
};

const unlisten = (
  client: WsClient,
  key: string,
  subscription: Record<string, unknown>
) => {
  client.unsubscribe(`${key}::listener`, {
    type: "unsubscribe",
    subscription,
  });
};

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  MockWebSocket.instances.length = 0;
});

describe("ws subscription status and error attribution", () => {
  it("accepts reversed mixed-channel acknowledgements", async () => {
    const { client, socket } = await setup();
    const request = traderStateRequest(AUTHORITY_A);
    const trader = listen(client, traderStateKey(AUTHORITY_A), request);
    const mids = listen(client, "allMids", { channel: "allMids" });
    socket.receive(status("subscribed", { channel: "allMids" }));
    socket.receive(status("subscribed", request));
    const update = traderStateData(AUTHORITY_A, 20);
    const midUpdate = {
      channel: "allMids",
      mids: { SOL: 123 },
      slot: 20,
      slotIndex: 0,
    };
    socket.receive(update);
    socket.receive(midUpdate);
    expect(trader.log).toEqual([update]);
    expect(mids.log).toEqual([midUpdate]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("correlates an echoed rejection overtaking another channel's acknowledgement", async () => {
    const { client, socket } = await setup();
    const request = traderStateRequest(AUTHORITY_A);
    const trader = listen(client, traderStateKey(AUTHORITY_A), request);
    const mids = listen(client, "allMids", { channel: "allMids" });
    socket.receive(error("unavailable", 503, { channel: "allMids" }));
    socket.receive(status("subscribed", request));
    const update = traderStateData(AUTHORITY_A, 21);
    socket.receive(update);
    expect(mids.log).toEqual([{ error: 503 }, "closed"]);
    expect(trader.log).toEqual([update]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("recovers when a quota rejection overtakes older same-subscription statuses", async () => {
    const { client, socket, events } = await setup();
    const request = traderStateRequest(AUTHORITY_A);
    const key = traderStateKey(AUTHORITY_A);
    listen(client, key, request);
    unlisten(client, key, request);
    const mids = listen(client, "allMids", { channel: "allMids" });
    const replacement = listen(client, key, request);
    expect(socket.sentMessages()).toEqual([
      { type: "subscribe", subscription: request },
      { type: "unsubscribe", subscription: request },
      { type: "subscribe", subscription: { channel: "allMids" } },
      { type: "subscribe", subscription: request },
    ]);

    socket.receive(status("subscribed", { channel: "allMids" }));
    socket.receive(error("subscription limit reached", 429, request));
    socket.receive(status("subscribed", request));
    socket.receive(status("unsubscribed", request));
    socket.receive(traderStateData(AUTHORITY_A, 21));
    expect(events.at(-1)).toEqual({
      kind: "error",
      code: 429,
      error: "subscription limit reached",
    });
    expect(socket.closes).toEqual([
      { code: 4001, reason: "subscription_response_mismatch" },
    ]);
    expect(replacement.log).toEqual([]);
    expect(mids.log).toEqual([]);

    await flush();
    const next = MockWebSocket.instances.at(-1)!;
    next.open();
    expect(next.sentMessages()).toEqual([
      { type: "subscribe", subscription: { channel: "allMids" } },
      { type: "subscribe", subscription: request },
    ]);
    next.receive(status("subscribed", { channel: "allMids" }));
    next.receive(status("subscribed", request));
    const update = traderStateData(AUTHORITY_A, 22);
    const midUpdate = {
      channel: "allMids",
      mids: { SOL: 123 },
      slot: 22,
      slotIndex: 0,
    };
    next.receive(update);
    next.receive(midUpdate);
    expect(replacement.log).toEqual([update]);
    expect(mids.log).toEqual([midUpdate]);
    expect(next.closes).toEqual([]);
    client.close();
  });

  it("keeps an ended subscription's echoed error context across another channel's status", async () => {
    const { client, socket } = await setup();
    const request = traderStateRequest(AUTHORITY_A);
    const trader = listen(client, traderStateKey(AUTHORITY_A), request);
    const mids = listen(client, "allMids", { channel: "allMids" });
    socket.receive(status("subscribed", request));
    socket.receive(status("subscribed", { channel: "allMids" }));
    socket.receive(status("unsubscribed", request));
    socket.receive(error("actor unavailable", 503, request));
    const update = {
      channel: "allMids",
      mids: { SOL: 123 },
      slot: 23,
      slotIndex: 0,
    };
    socket.receive(update);
    expect(trader.log).toEqual(["closed", { error: 503 }]);
    expect(mids.log).toEqual([update]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("reconnects without guessing which mixed-channel request a legacy error rejects", async () => {
    const { client, socket, events } = await setup();
    const request = traderStateRequest(AUTHORITY_A);
    const trader = listen(client, traderStateKey(AUTHORITY_A), request);
    const mids = listen(client, "allMids", { channel: "allMids" });
    socket.receive(error("unavailable", 503));
    expect(events.at(-1)).toEqual({
      kind: "error",
      code: 503,
      error: "unavailable",
    });
    expect(trader.log).toEqual([]);
    expect(mids.log).toEqual([]);
    expect(socket.closes).toEqual([
      { code: 4001, reason: "subscription_response_mismatch" },
    ]);
    await flush();
    const next = MockWebSocket.instances.at(-1)!;
    next.open();
    expect(next.sentMessages()).toEqual([
      { type: "subscribe", subscription: request },
      { type: "subscribe", subscription: { channel: "allMids" } },
    ]);
    client.close();
  });

  it.each(["echoed", "legacy"] as const)(
    "preserves an acknowledged exchange subscription after an %s refresh rejection",
    async (protocol) => {
      const { client, socket, events } = await setup();
      const request = { channel: "exchange" };
      listen(client, "exchange", request);
      socket.receive(status("subscribed", request));
      const refreshed = listen(client, "exchange", request);
      socket.receive(
        error(
          "refresh cooldown",
          429,
          protocol === "echoed" ? request : undefined
        )
      );
      const update = { channel: "exchange", messageType: "delta", slot: 22 };
      socket.receive(update);
      expect(refreshed.log).toEqual([{ error: 429 }, update]);
      expect(refreshed.closed).toBe(0);
      expect(events.at(-1)).toEqual({
        kind: "error",
        code: 429,
        error: "refresh cooldown",
        subscription: request,
      });
      expect(socket.closes).toEqual([]);
      socket.close(1006, "network");
      await flush();
      const next = MockWebSocket.instances.at(-1)!;
      next.open();
      expect(next.sentMessages()).toEqual([
        { type: "subscribe", subscription: request },
      ]);
      client.close();
    }
  );

  it("reports the real status of an unsubscribed frame", async () => {
    const { client, socket, events } = await setup();

    socket.receive(status("unsubscribed", { channel: "allMids" }));

    expect(events).toEqual([
      {
        kind: "status",
        status: "unsubscribed",
        subscription: { channel: "allMids" },
      },
    ]);
    client.close();
  });

  it("attributes a rejection to its subscription and closes only that listener", async () => {
    const { client, socket, events } = await setup();
    const mids = listen(client, "allMids", { channel: "allMids" });
    socket.receive(status("subscribed", { channel: "allMids" }));

    const trader = listen(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    socket.receive(error("Invalid authority", 400));
    socket.receive({ channel: "allMids", mids: {}, slot: 1, slotIndex: 0 });

    expect(events.at(-1)).toEqual({
      kind: "error",
      code: 400,
      error: "Invalid authority",
      subscription: traderStateRequest(AUTHORITY_A),
    });
    expect(trader.log).toEqual([{ error: 400 }, "closed"]);
    expect(mids.log).toHaveLength(1);
    expect(mids.closed).toBe(0);
    expect(mids.errors).toEqual([]);
    expect(socket.closes).toEqual([]);

    // The rejected subscription is not resent on reconnect.
    socket.close(1006, "network");
    await flush();
    const next = MockWebSocket.instances.at(-1)!;
    next.open();
    expect(next.sentMessages()).toEqual([
      { type: "subscribe", subscription: { channel: "allMids" } },
    ]);
    client.close();
  });

  it("closes a trader state subscription whose register fails after its ack", async () => {
    const { client, socket, events } = await setup();
    const mids = listen(client, "allMids", { channel: "allMids" });
    socket.receive(status("subscribed", { channel: "allMids" }));

    const a = listen(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    const b = listen(
      client,
      traderStateKey(AUTHORITY_B),
      traderStateRequest(AUTHORITY_B)
    );
    events.length = 0;

    socket.receive(status("subscribed", traderStateRequest(AUTHORITY_A)));
    socket.receive(traderStateData(AUTHORITY_A, 1));
    socket.receive(status("unsubscribed", traderStateRequest(AUTHORITY_A)));
    socket.receive(
      error(
        "trader state actor unavailable",
        500,
        traderStateRequest(AUTHORITY_A)
      )
    );
    socket.receive(status("subscribed", traderStateRequest(AUTHORITY_B)));
    socket.receive(traderStateData(AUTHORITY_B, 2));
    socket.receive({ channel: "allMids", mids: {}, slot: 3, slotIndex: 0 });

    expect(events).toEqual([
      {
        kind: "status",
        status: "subscribed",
        subscription: traderStateRequest(AUTHORITY_A),
      },
      {
        kind: "status",
        status: "unsubscribed",
        subscription: traderStateRequest(AUTHORITY_A),
      },
      {
        kind: "error",
        code: 500,
        error: "trader state actor unavailable",
        subscription: traderStateRequest(AUTHORITY_A),
      },
      {
        kind: "status",
        status: "subscribed",
        subscription: traderStateRequest(AUTHORITY_B),
      },
    ]);
    expect(a.log).toEqual([
      traderStateData(AUTHORITY_A, 1),
      "closed",
      { error: 500 },
    ]);
    expect(b.log).toEqual([traderStateData(AUTHORITY_B, 2)]);
    expect(mids.log).toHaveLength(1);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("closes exchange after a bootstrap failure and keeps later subscriptions", async () => {
    const { client, socket, events } = await setup();
    const exchange = listen(client, "exchange", { channel: "exchange" });
    const mids = listen(client, "allMids", { channel: "allMids" });

    const snapshot = { channel: "exchange", messageType: "snapshot", slot: 1 };
    socket.receive(snapshot);
    socket.receive(status("unsubscribed", { channel: "exchange" }));
    socket.receive(
      error("exchange bootstrap failed", 500, { channel: "exchange" })
    );
    socket.receive(status("subscribed", { channel: "allMids" }));
    socket.receive({ channel: "allMids", mids: {}, slot: 2, slotIndex: 0 });

    expect(exchange.log).toEqual([snapshot, "closed", { error: 500 }]);
    expect(events).toEqual([
      {
        kind: "status",
        status: "unsubscribed",
        subscription: { channel: "exchange" },
      },
      {
        kind: "error",
        code: 500,
        error: "exchange bootstrap failed",
        subscription: { channel: "exchange" },
      },
      {
        kind: "status",
        status: "subscribed",
        subscription: { channel: "allMids" },
      },
    ]);
    expect(mids.log).toHaveLength(1);
    expect(mids.closed).toBe(0);
    client.close();
  });

  it("attributes an error after an ack to the next unanswered frame", async () => {
    const { client, socket, events } = await setup();
    const a = listen(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    const b = listen(
      client,
      traderStateKey(AUTHORITY_B),
      traderStateRequest(AUTHORITY_B)
    );

    socket.receive(status("subscribed", traderStateRequest(AUTHORITY_A)));
    socket.receive(error("Invalid authority", 400));
    socket.receive(traderStateData(AUTHORITY_A, 1));

    expect(events.at(-1)).toMatchObject({
      kind: "error",
      subscription: traderStateRequest(AUTHORITY_B),
    });
    expect(b.log).toEqual([{ error: 400 }, "closed"]);
    expect(a.log).toEqual([traderStateData(AUTHORITY_A, 1)]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("matches canonicalized echoes to the requests that were sent", async () => {
    const { client, socket, events } = await setup();
    const book = listen(client, "orderbook:sol", {
      channel: "orderbook",
      symbol: "sol",
    });
    const trades = listen(client, "trades:sol", {
      channel: "trades",
      symbol: "sol",
    });

    socket.receive(
      status("subscribed", { channel: "orderbook", symbol: "SOL" })
    );
    socket.receive(
      status("subscribed", { channel: "trades", marketSymbol: "SOL" })
    );
    // Nothing is pending any more, so this error is unattributed.
    socket.receive(error("unrelated", 400));

    expect(events).toEqual([
      {
        kind: "status",
        status: "subscribed",
        subscription: { channel: "orderbook", symbol: "SOL" },
      },
      {
        kind: "status",
        status: "subscribed",
        subscription: { channel: "trades", marketSymbol: "SOL" },
      },
      { kind: "error", code: 400, error: "unrelated" },
    ]);
    expect(book.log).toEqual([]);
    expect(trades.log).toEqual([]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("treats the reply to a dropped subscription as a plain status", async () => {
    const { client, socket, events } = await setup();
    const a = listen(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    const mids = listen(client, "allMids", { channel: "allMids" });
    socket.receive(status("subscribed", traderStateRequest(AUTHORITY_A)));
    socket.receive(status("subscribed", { channel: "allMids" }));
    events.length = 0;

    unlisten(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    expect(socket.sentMessages().at(-1)).toEqual({
      type: "unsubscribe",
      subscription: traderStateRequest(AUTHORITY_A),
    });
    socket.receive(status("unsubscribed", traderStateRequest(AUTHORITY_A)));

    expect(events).toEqual([
      {
        kind: "status",
        status: "unsubscribed",
        subscription: traderStateRequest(AUTHORITY_A),
      },
    ]);
    expect(a.closed).toBe(0);
    expect(mids.closed).toBe(0);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("leaves an error with nothing pending unattributed", async () => {
    const { client, socket, events } = await setup();
    const mids = listen(client, "allMids", { channel: "allMids" });
    socket.receive(status("subscribed", { channel: "allMids" }));
    events.length = 0;

    socket.receive(error("Invalid JSON", 400));

    expect(events).toEqual([
      { kind: "error", code: 400, error: "Invalid JSON" },
    ]);
    expect(mids.closed).toBe(0);
    expect(mids.errors).toEqual([]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("fails closed on an ambiguous error and resubscribes on the next socket", async () => {
    const { client, socket, events } = await setup();
    listen(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    unlisten(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    const b = listen(
      client,
      traderStateKey(AUTHORITY_B),
      traderStateRequest(AUTHORITY_B)
    );

    socket.receive(status("subscribed", traderStateRequest(AUTHORITY_A)));
    socket.receive(status("unsubscribed", traderStateRequest(AUTHORITY_A)));
    socket.receive(error("trader state actor unavailable", 500));

    expect(events.at(-1)).toEqual({
      kind: "error",
      code: 500,
      error: "trader state actor unavailable",
    });
    expect(socket.closes).toEqual([
      { code: 4001, reason: "subscription_response_mismatch" },
    ]);
    expect(b.closed).toBe(0);

    await flush();
    const next = MockWebSocket.instances.at(-1)!;
    expect(next).not.toBe(socket);
    next.open();
    expect(next.sentMessages()).toEqual([
      { type: "subscribe", subscription: traderStateRequest(AUTHORITY_B) },
    ]);
    client.close();
  });

  it("attributes the same error by its echo without failing closed", async () => {
    const { client, socket, events } = await setup();
    listen(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    unlisten(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    const b = listen(
      client,
      traderStateKey(AUTHORITY_B),
      traderStateRequest(AUTHORITY_B)
    );

    socket.receive(status("subscribed", traderStateRequest(AUTHORITY_A)));
    socket.receive(status("unsubscribed", traderStateRequest(AUTHORITY_A)));
    socket.receive(
      error(
        "trader state actor unavailable",
        500,
        traderStateRequest(AUTHORITY_A)
      )
    );
    socket.receive(status("subscribed", traderStateRequest(AUTHORITY_B)));

    expect(events.slice(-2)).toEqual([
      {
        kind: "error",
        code: 500,
        error: "trader state actor unavailable",
        subscription: traderStateRequest(AUTHORITY_A),
      },
      {
        kind: "status",
        status: "subscribed",
        subscription: traderStateRequest(AUTHORITY_B),
      },
    ]);
    expect(socket.closes).toEqual([]);
    expect(b.log).toEqual([]);
    client.close();
  });

  it("closes the pending subscription an echoed error names", async () => {
    const { client, socket, events } = await setup();
    const a = listen(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );

    socket.receive(
      error(
        "insufficient_scope_trader_state_subscription",
        403,
        traderStateRequest(AUTHORITY_A)
      )
    );

    expect(events).toEqual([
      {
        kind: "error",
        code: 403,
        error: "insufficient_scope_trader_state_subscription",
        subscription: traderStateRequest(AUTHORITY_A),
      },
    ]);
    expect(a.log).toEqual([{ error: 403 }, "closed"]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  const recoverSubscription = async (
    client: WsClient,
    socket: MockWebSocket,
    request: Record<string, unknown>,
    stream: Stream
  ) => {
    expect(socket.closes).toEqual([
      { code: 4001, reason: "subscription_response_mismatch" },
    ]);
    expect(stream.log).toEqual([]);
    await flush();
    const next = MockWebSocket.instances.at(-1)!;
    next.open();
    expect(next.sentMessages()).toEqual([
      { type: "subscribe", subscription: request },
    ]);
    next.receive(status("subscribed", request));
    const update = {
      ...request,
      messageType: "snapshot",
      slot: 30,
    };
    next.receive(update);
    expect(stream.log).toEqual([update]);
    expect(next.closes).toEqual([]);
    client.close();
  };

  it("recovers when an echoed error could belong to an acknowledged unsubscribe or its replacement", async () => {
    const { client, socket, events } = await setup();
    const key = traderStateKey(AUTHORITY_A);
    const request = traderStateRequest(AUTHORITY_A);
    listen(client, key, request);
    socket.receive(status("subscribed", request));
    unlisten(client, key, request);
    socket.receive(status("unsubscribed", request));
    const replacement = listen(client, key, request);

    socket.receive(error("retry later", 429, request));
    expect(events.at(-1)).toEqual({
      kind: "error",
      code: 429,
      error: "retry later",
    });
    await recoverSubscription(client, socket, request, replacement);
  });

  it("recovers rather than guessing which pending generation an echoed rejection belongs to", async () => {
    const { client, socket } = await setup();
    const request = traderStateRequest(AUTHORITY_A);
    const key = traderStateKey(AUTHORITY_A);
    listen(client, key, request);
    unlisten(client, key, request);
    const replacement = listen(client, key, request);

    socket.receive(error("try again", 503, request));
    await recoverSubscription(client, socket, request, replacement);
  });

  it("recovers when two unanswered refresh attempts share an error echo", async () => {
    const { client, socket } = await setup();
    const request = traderStateRequest(AUTHORITY_A);
    const key = traderStateKey(AUTHORITY_A);
    listen(client, key, request);
    const refreshed = listen(client, key, request);

    socket.receive(error("try again", 503, request));
    await recoverSubscription(client, socket, request, refreshed);
  });

  it("recovers when an echoed error could belong to an ended bootstrap or queued replacement", async () => {
    const { client, socket } = await setup();
    const request = { channel: "exchange" };
    listen(client, "original", request, "exchange");
    unlisten(client, "original", request);
    const replacement = listen(client, "replacement", request, "exchange");

    socket.receive(status("unsubscribed", request));
    socket.receive(error("bootstrap failed", 500, request));
    await recoverSubscription(client, socket, request, replacement);
  });

  it("recovers when an ended stream's onClose retry shares its error echo", async () => {
    const { client, socket } = await setup();
    const request = { channel: "exchange" };
    const originalErrors: Array<{ code: number; error: string }> = [];
    let replacement: Stream | undefined;
    const onClose = vi.fn(() => {
      replacement = listen(client, "replacement", request, "exchange");
    });
    client.subscribe(
      "original",
      { type: "subscribe", subscription: request },
      vi.fn(),
      {
        routingKey: "exchange",
        onError: (err) => originalErrors.push(err),
        onClose,
      }
    );

    socket.receive(status("unsubscribed", request));
    expect(onClose).toHaveBeenCalledOnce();
    socket.receive(error("bootstrap failed", 500, request));
    expect(originalErrors).toEqual([]);
    expect(replacement).toBeDefined();
    await recoverSubscription(client, socket, request, replacement!);
  });

  it("matches stripped fields in rejection and acknowledgement echoes", async () => {
    const { client, socket } = await setup();
    const rejected = listen(client, "market:invalid", {
      channel: "market",
      symbol: "INVALID",
      clientTag: "ignored by the server",
    });
    socket.receive(
      error("unknown market", 400, { channel: "market", symbol: "INVALID" })
    );
    expect(rejected.log).toEqual([{ error: 400 }, "closed"]);

    const mids = listen(client, "allMids", {
      channel: "allMids",
      clientTag: "also ignored",
    });
    socket.receive(status("subscribed", { channel: "allMids" }));
    const trader = listen(
      client,
      traderStateKey(AUTHORITY_A),
      traderStateRequest(AUTHORITY_A)
    );
    socket.receive(error("invalid authority", 400));
    const update = {
      channel: "allMids",
      mids: { SOL: 123 },
      slot: 12,
      slotIndex: 0,
    };
    socket.receive(update);

    expect(trader.log).toEqual([{ error: 400 }, "closed"]);
    expect(mids.log).toEqual([update]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("resubscribes allMids after an unsubscribe when heartbeats are disabled", async () => {
    const { client, socket } = await setup();
    const mids = { channel: "allMids" };
    listen(client, "allMids", mids);
    socket.receive(status("subscribed", mids));

    // Resubscribe before the server answers the unsubscribe.
    unlisten(client, "allMids", mids);
    const replacement = listen(client, "allMids", mids);

    const subscribes = socket
      .sentMessages()
      .filter((message) => message.type === "subscribe");
    expect(subscribes).toEqual([
      { type: "subscribe", subscription: mids },
      { type: "subscribe", subscription: mids },
    ]);
    expect(replacement.closed).toBe(0);
    client.close();
  });

  it("ends a rejected adapter stream", async () => {
    const errors: WsServerErrorMessage[] = [];
    const streams = createPhoenixWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "eager",
      heartbeat: false,
      onServerError: (message) => errors.push(message),
    });
    await flush();
    const socket = MockWebSocket.instances.at(-1)!;
    socket.open();

    const iterator = streams
      .traderState(AUTHORITY_A, 0)
      [Symbol.asyncIterator]();
    const next = iterator.next();
    socket.receive(error("Invalid authority", 400));

    await expect(next).rejects.toMatchObject({
      code: 400,
      message: "Invalid authority",
    });
    expect(errors).toEqual([
      {
        channel: "error",
        error: "Invalid authority",
        code: 400,
        subscription: traderStateRequest(AUTHORITY_A),
      },
    ]);
    streams.close();
  });
});
