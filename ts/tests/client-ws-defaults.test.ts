import { afterEach, describe, expect, it, vi } from "vitest";
import { createPhoenixClient } from "@/index";

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((evt: { data: string }) => void) | null = null;
  onclose: ((evt: { code: number; reason?: string }) => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(
    public url: string,
    public protocol?: string | string[]
  ) {
    MockWebSocket.instances.push(this);
    queueMicrotask(() => this.onopen?.());
  }

  send() {}

  close() {}
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  MockWebSocket.instances.length = 0;
});

describe("createPhoenixClient websocket defaults", () => {
  it("enables a lazy websocket transport by default", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const client = createPhoenixClient();

    expect(client.streams).toBeDefined();
    expect(MockWebSocket.instances).toHaveLength(0);

    client.streams!.wsClient.subscribe(
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
    expect(MockWebSocket.instances[0]?.url).toBe(
      "wss://perp-api.phoenix.trade/v1/ws"
    );

    client.dispose();
  });

  it("disables client.streams when ws is false", () => {
    const client = createPhoenixClient({
      ws: false,
    });

    expect(client.streams).toBeUndefined();

    client.dispose();
  });
});
