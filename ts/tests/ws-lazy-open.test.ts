import { createPhoenixWsClient } from "@/index";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createWsClient } from "@/ws/WsClient";

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((evt: { data: string }) => void) | null = null;
  onclose: ((evt: { code: number; reason?: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  closeCalls = 0;

  constructor(
    public url: string,
    public protocol?: string | string[]
  ) {
    MockWebSocket.instances.push(this);
    queueMicrotask(() => this.onopen?.());
  }

  send() {}

  close() {
    this.closeCalls += 1;
    this.onclose?.({ code: 1000, reason: "idle" });
  }
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  MockWebSocket.instances.length = 0;
});

describe("rise ws lazy connection mode", () => {
  it("keeps the subscription and socket open across unfamiliar messages", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket);
    const errors = vi.spyOn(console, "error").mockImplementation(() => {});
    const warnings = vi.spyOn(console, "warn").mockImplementation(() => {});
    const onServerError = vi.fn();
    const streams = createPhoenixWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "lazy",
      onServerError,
    });
    const abort = new AbortController();
    const updates = streams.traderState("authority", 0, abort.signal);
    const iterator = updates[Symbol.asyncIterator]();
    const next = iterator.next();
    await vi.advanceTimersByTimeAsync(0);
    const socket = MockWebSocket.instances[0]!;
    for (const message of [
      { channel: "futureChannel", messageType: "futureMessage" },
      { channel: "exchange", messageType: "futureMessage" },
      { channel: "traderState", messageType: "futureMessage" },
      {
        channel: "traderState",
        authority: "authority",
        traderPdaIndex: 0,
        slot: "11",
        messageType: "delta",
        deltas: [
          {
            subaccountIndex: 0,
            sequence: 2,
            collateral: "105",
            futureField: true,
            escrowRequests: [
              {
                sequenceNumber: "7",
                senderAuthority: "sender",
                senderPdaIndex: 0,
                senderSubaccountIndex: 0,
                expirationOffset: null,
                initialSlot: "10",
                expirationSlot: null,
                actions: [
                  { kind: "futureAction" },
                  { kind: "cash", amount: "5" },
                ],
              },
            ],
          },
        ],
      },
    ]) {
      socket.onmessage?.({ data: JSON.stringify(message) });
      await vi.advanceTimersByTimeAsync(0);
    }
    await expect(next).resolves.toMatchObject({
      done: false,
      value: {
        messageType: "delta",
        slot: 11n,
        deltas: [
          {
            sequence: 2,
            collateral: "105",
            escrowRequests: [
              { actions: [{ kind: "unknown" }, { kind: "cash", amount: "5" }] },
            ],
          },
        ],
      },
    });
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(socket.closeCalls).toBe(0);
    expect(onServerError).not.toHaveBeenCalled();
    expect(errors).not.toHaveBeenCalled();
    expect(warnings).not.toHaveBeenCalled();
    abort.abort();
    streams.close();
    errors.mockRestore();
    warnings.mockRestore();
  });

  it("opens on first subscription and closes after the idle timeout", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const ws = createWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "lazy",
      idleCloseMs: 25,
    });

    expect(MockWebSocket.instances).toHaveLength(0);

    ws.subscribe(
      "exchange",
      {
        type: "subscribe",
        subscription: { channel: "exchange" },
      },
      () => {}
    );

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(MockWebSocket.instances).toHaveLength(1);

    ws.unsubscribe("exchange", {
      type: "unsubscribe",
      subscription: { channel: "exchange" },
    });

    await vi.advanceTimersByTimeAsync(24);
    expect(MockWebSocket.instances[0]?.closeCalls).toBe(0);

    await vi.advanceTimersByTimeAsync(1);
    expect(MockWebSocket.instances[0]?.closeCalls).toBe(1);

    ws.close();
  });

  it("forwards lazy connection config through createPhoenixWsClient", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const streams = createPhoenixWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "lazy",
      idleCloseMs: 25,
    });

    expect(MockWebSocket.instances).toHaveLength(0);

    streams.wsClient.subscribe(
      "exchange",
      {
        type: "subscribe",
        subscription: { channel: "exchange" },
      },
      () => {}
    );

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(MockWebSocket.instances).toHaveLength(1);

    streams.wsClient.unsubscribe("exchange", {
      type: "unsubscribe",
      subscription: { channel: "exchange" },
    });

    await vi.advanceTimersByTimeAsync(25);
    expect(MockWebSocket.instances[0]?.closeCalls).toBe(1);

    streams.close();
  });

  it("applies the default lazy idle timeout through createPhoenixWsClient", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const streams = createPhoenixWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "lazy",
    });

    streams.wsClient.subscribe(
      "exchange",
      {
        type: "subscribe",
        subscription: { channel: "exchange" },
      },
      () => {}
    );

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(MockWebSocket.instances).toHaveLength(1);

    streams.wsClient.unsubscribe("exchange", {
      type: "unsubscribe",
      subscription: { channel: "exchange" },
    });

    await vi.advanceTimersByTimeAsync(29_999);
    expect(MockWebSocket.instances[0]?.closeCalls).toBe(0);

    await vi.advanceTimersByTimeAsync(1);
    expect(MockWebSocket.instances[0]?.closeCalls).toBe(1);

    streams.close();
  });

  it("ignores stale socket opens until the current socket is open", async () => {
    vi.useFakeTimers();

    class RaceWebSocket {
      static instances: RaceWebSocket[] = [];
      onopen: (() => void) | null = null;
      onmessage: ((evt: { data: string }) => void) | null = null;
      onclose: ((evt: { code: number; reason?: string }) => void) | null = null;
      onerror: (() => void) | null = null;
      readyState = 0;
      readonly sent: string[] = [];

      constructor(
        public url: string,
        public protocol?: string | string[]
      ) {
        RaceWebSocket.instances.push(this);
      }

      open() {
        this.readyState = 1;
        this.onopen?.();
      }

      send(payload: string) {
        if (this.readyState !== 1) {
          throw new Error("Still in CONNECTING state");
        }
        this.sent.push(payload);
      }

      close() {
        this.readyState = 3;
        this.onclose?.({ code: 1000, reason: "closed" });
      }
    }

    vi.stubGlobal("WebSocket", RaceWebSocket as unknown as typeof WebSocket);

    const ws = createWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "lazy",
      backoff: { baseMs: 0, maxMs: 0 },
    });

    const subscribeMessage = {
      type: "subscribe" as const,
      subscription: { channel: "exchange" as const },
    };

    ws.subscribe("exchange", subscribeMessage, () => {});

    await Promise.resolve();
    await Promise.resolve();

    expect(RaceWebSocket.instances).toHaveLength(1);
    const first = RaceWebSocket.instances[0]!;
    first.onclose?.({ code: 1006, reason: "disconnect" });

    await vi.runAllTimersAsync();
    await Promise.resolve();

    expect(RaceWebSocket.instances).toHaveLength(2);
    const second = RaceWebSocket.instances[1]!;

    expect(() => first.open()).not.toThrow();
    expect(second.sent).toEqual([]);

    second.open();
    expect(second.sent).toEqual([JSON.stringify(subscribeMessage)]);

    ws.close();
  });
});
