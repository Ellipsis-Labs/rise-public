import { afterEach, describe, expect, it, vi } from "vitest";
import { createPhoenixClient } from "@/index";
import { PHOENIX_CLIENT_HEADER_VALUE } from "@/clientIdentity";

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
  delete process.env.PHOENIX_DISABLE_CLIENT_IDENTITY_HEADER;
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
    const url = new URL(MockWebSocket.instances[0]!.url);
    expect(url.origin + url.pathname).toBe(
      "wss://perp-api.phoenix.trade/v1/ws"
    );
    expect(url.searchParams.get("phoenixClient")).toBe(
      PHOENIX_CLIENT_HEADER_VALUE
    );

    client.dispose();
  });

  it("preserves websocket query parameters and the client identity opt-out", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket);
    process.env.PHOENIX_DISABLE_CLIENT_IDENTITY_HEADER = "1";
    const client = createPhoenixClient({
      ws: { url: "wss://example.com/v1/ws?region=us" },
    });
    client.streams!.wsClient.subscribe(
      "exchange",
      {
        type: "subscribe",
        subscription: { channel: "exchange" },
      },
      () => {}
    );
    await vi.advanceTimersByTimeAsync(0);
    expect(MockWebSocket.instances[0]?.url).toBe(
      "wss://example.com/v1/ws?region=us"
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
