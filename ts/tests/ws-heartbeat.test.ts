import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createWsClient } from "@/ws/WsClient";
import type {
  SubscriptionMessage,
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

  // A half-open socket never completes the closing handshake: record the
  // close but never fire `onclose`.
  close(code?: number, reason?: string) {
    this.closes.push({ code, reason });
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

const ALL_MIDS_SUBSCRIBE: SubscriptionMessage = {
  type: "subscribe",
  subscription: { channel: "allMids" },
};
const EXCHANGE_SUBSCRIBE: SubscriptionMessage = {
  type: "subscribe",
  subscription: { channel: "exchange" },
};
const allMidsFrame = (slot: number) => ({
  channel: "allMids",
  mids: { SOL: 1 },
  slot,
  slotIndex: 0,
});

const flush = async () => {
  await Promise.resolve();
  await Promise.resolve();
  await vi.advanceTimersByTimeAsync(0);
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

describe("ws heartbeat", () => {
  it("closes a silent socket after the idle timeout and reconnects", async () => {
    const client = createWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "eager",
      backoff: { baseMs: 0, maxMs: 0 },
      heartbeat: { idleTimeoutMs: 1_000 },
    });
    client.subscribe("exchange", EXCHANGE_SUBSCRIBE, () => {});
    await flush();
    const socket = MockWebSocket.instances[0]!;
    socket.open();
    expect(socket.sentMessages()).toEqual([
      EXCHANGE_SUBSCRIBE,
      ALL_MIDS_SUBSCRIBE,
    ]);

    // Every inbound message resets the timer.
    await vi.advanceTimersByTimeAsync(999);
    socket.receive(allMidsFrame(1));
    await vi.advanceTimersByTimeAsync(999);
    expect(socket.closes).toEqual([]);
    expect(MockWebSocket.instances).toHaveLength(1);

    await vi.advanceTimersByTimeAsync(1);
    expect(socket.closes).toEqual([{ code: 4000, reason: "idle_timeout" }]);

    await vi.advanceTimersByTimeAsync(10);
    expect(MockWebSocket.instances).toHaveLength(2);
    const next = MockWebSocket.instances[1]!;
    next.open();
    expect(next.sentMessages()).toEqual([
      EXCHANGE_SUBSCRIBE,
      ALL_MIDS_SUBSCRIBE,
    ]);

    // The stale socket's frames are ignored.
    socket.receive(allMidsFrame(2));
    await vi.advanceTimersByTimeAsync(1_000);
    expect(next.closes).toEqual([{ code: 4000, reason: "idle_timeout" }]);
    client.close();
  });

  it("keeps today's behavior when disabled", async () => {
    const client = createWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "eager",
      heartbeat: false,
    });
    client.subscribe("exchange", EXCHANGE_SUBSCRIBE, () => {});
    await flush();
    const socket = MockWebSocket.instances[0]!;
    socket.open();

    await vi.advanceTimersByTimeAsync(10 * 60_000);
    expect(socket.sentMessages()).toEqual([EXCHANGE_SUBSCRIBE]);
    expect(socket.closes).toEqual([]);
    client.close();
  });

  it("hides the internal allMids subscription from callers", async () => {
    const statuses: WsSubscriptionStatusMessage[] = [];
    const client = createWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "eager",
      onSubscriptionStatus: (message) => statuses.push(message),
    });
    const exchangeFrames: unknown[] = [];
    client.subscribe("exchange", EXCHANGE_SUBSCRIBE, (data) =>
      exchangeFrames.push(data)
    );
    await flush();
    const socket = MockWebSocket.instances[0]!;
    socket.open();

    socket.receive({
      channel: "subscriptionStatus",
      status: "subscribed",
      subscription: { channel: "exchange" },
    });
    socket.receive({
      channel: "subscriptionStatus",
      status: "subscribed",
      subscription: { channel: "allMids" },
    });
    socket.receive(allMidsFrame(1));

    expect(statuses.map((message) => message.subscription)).toEqual([
      { channel: "exchange" },
    ]);
    expect(exchangeFrames).toEqual([]);
    client.close();
  });

  it("shares the server allMids subscription with a caller's", async () => {
    const statuses: WsSubscriptionStatusMessage[] = [];
    const client = createWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "eager",
      onSubscriptionStatus: (message) => statuses.push(message),
    });
    await flush();
    const socket = MockWebSocket.instances[0]!;
    socket.open();
    expect(socket.sentMessages()).toEqual([ALL_MIDS_SUBSCRIBE]);

    const frames: unknown[] = [];
    client.subscribe("allMids", ALL_MIDS_SUBSCRIBE, (data) =>
      frames.push(data)
    );
    socket.receive({
      channel: "subscriptionStatus",
      status: "subscribed",
      subscription: { channel: "allMids" },
    });
    socket.receive(allMidsFrame(1));
    expect(frames).toEqual([allMidsFrame(1)]);
    // The caller rides on the heartbeat's subscription: no duplicate frame.
    expect(socket.sentMessages()).toEqual([ALL_MIDS_SUBSCRIBE]);
    expect(statuses).toHaveLength(1);

    // Dropping the caller keeps the heartbeat's subscription on the server.
    client.unsubscribe("allMids", {
      type: "unsubscribe",
      subscription: { channel: "allMids" },
    });
    expect(socket.sentMessages()).toEqual([ALL_MIDS_SUBSCRIBE]);
    socket.receive(allMidsFrame(2));
    expect(frames).toEqual([allMidsFrame(1)]);

    // A caller that was subscribed before the socket opened is resent once.
    client.subscribe("allMids", ALL_MIDS_SUBSCRIBE, (data) =>
      frames.push(data)
    );
    socket.onclose?.({ code: 1006, reason: "network" });
    await vi.advanceTimersByTimeAsync(10_000);
    const next = MockWebSocket.instances.at(-1)!;
    expect(next).not.toBe(socket);
    next.open();
    expect(next.sentMessages()).toEqual([ALL_MIDS_SUBSCRIBE]);
    client.close();
  });

  it("does not keep a lazy socket open by itself", async () => {
    const client = createWsClient({
      url: "wss://example.com/v1/ws",
      authMode: "anonymous",
      connectMode: "lazy",
      idleCloseMs: 25,
      heartbeat: { idleTimeoutMs: 1_000 },
    });
    client.subscribe("exchange", EXCHANGE_SUBSCRIBE, () => {});
    await flush();
    const socket = MockWebSocket.instances[0]!;
    socket.open();
    expect(socket.sentMessages()).toEqual([
      EXCHANGE_SUBSCRIBE,
      ALL_MIDS_SUBSCRIBE,
    ]);

    client.unsubscribe("exchange", {
      type: "unsubscribe",
      subscription: { channel: "exchange" },
    });
    await vi.advanceTimersByTimeAsync(25);
    expect(socket.closes).toEqual([{ code: undefined, reason: undefined }]);

    // Once closed, the internal subscription alone does not reconnect a lazy
    // client, and the heartbeat timer no longer fires.
    socket.onclose?.({ code: 1000, reason: "" });
    await vi.advanceTimersByTimeAsync(10_000);
    expect(MockWebSocket.instances).toHaveLength(1);
    client.close();
  });
});
