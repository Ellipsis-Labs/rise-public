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
