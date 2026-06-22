import { auth, createPhoenixClient } from "@/index";
import type { AuthSessionSnapshot } from "@/auth/session";
import { afterEach, describe, expect, it, vi } from "vitest";

const toBase64Url = (value: unknown): string =>
  Buffer.from(JSON.stringify(value)).toString("base64url");

const makeJwt = (payload: { sid: string; jti: string; exp: number }): string =>
  `${toBase64Url({ alg: "none", typ: "JWT" })}.${toBase64Url(payload)}.sig`;

const makeSnapshot = (
  overrides: Partial<AuthSessionSnapshot> = {}
): AuthSessionSnapshot => {
  const sid = overrides.sessionId ?? "sid-1";
  const accessJti = overrides.accessJti ?? "jti-1";
  const expiresAt = overrides.expiresAt ?? Date.now() + 60_000;

  return {
    sessionId: sid,
    accessToken:
      overrides.accessToken ??
      makeJwt({
        sid,
        jti: accessJti,
        exp: Math.floor(expiresAt / 1000),
      }),
    refreshToken: overrides.refreshToken ?? "refresh-token-1",
    popKey: overrides.popKey ?? Buffer.from("pop-key-1").toString("base64url"),
    accessJti,
    expiresAt,
    refreshExpiresAt: overrides.refreshExpiresAt ?? Date.now() + 120_000,
    counter: overrides.counter ?? "0",
  };
};

const makeNextSnapshot = () =>
  makeSnapshot({
    sessionId: "sid-2",
    accessJti: "jti-2",
    refreshToken: "refresh-token-2",
    popKey: Buffer.from(
      Uint8Array.from({ length: 32 }, (_, index) => index + 1)
    ).toString("base64url"),
    expiresAt: Date.now() + 120_000,
    refreshExpiresAt: Date.now() + 240_000,
  });

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  sent: string[] = [];
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

  send(payload: string) {
    this.sent.push(payload);
  }

  close() {
    queueMicrotask(() => this.onclose?.({ code: 1000 }));
  }
}

const stubWebSocket = () => {
  vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);
};

const stubSuccessfulFetch = () => {
  const fetchMock = vi.fn(async () => {
    return new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
};

const createSessionManager = () =>
  new auth.AuthSessionManager(new auth.MemoryAuthSessionStorage());

const createExternalSessionClient = (sessionManager = createSessionManager()) =>
  createPhoenixClient({
    baseUrl: "https://example.com",
    auth: true,
    authConfig: {
      sessionManager,
      sessionControl: "external",
    },
    ws: {
      url: "wss://example.com/v1/ws",
      connectMode: "eager",
    },
  });

const sentMessages = (socket: MockWebSocket | undefined): unknown[] =>
  (socket?.sent ?? []).map((payload): unknown => JSON.parse(payload));

const flushWs = async () => {
  await Promise.resolve();
  await Promise.resolve();
  await vi.advanceTimersByTimeAsync(0);
};

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  MockWebSocket.instances.length = 0;
});

describe("rise external session control websocket integration", () => {
  it("waits for an external session update before connecting with expired auth", async () => {
    vi.useFakeTimers();

    stubWebSocket();
    const fetchMock = stubSuccessfulFetch();

    const sessionManager = new auth.AuthSessionManager(
      new auth.MemoryAuthSessionStorage()
    );
    await sessionManager.importSnapshot(
      makeSnapshot({
        accessToken: makeJwt({
          sid: "sid-1",
          jti: "jti-1",
          exp: Math.floor((Date.now() - 1_000) / 1000),
        }),
        expiresAt: Date.now() - 1_000,
      })
    );

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      auth: true,
      authConfig: {
        sessionManager,
        sessionControl: "external",
      },
      ws: {
        url: "wss://example.com/v1/ws",
        connectMode: "eager",
      },
    });

    await flushWs();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(MockWebSocket.instances).toHaveLength(0);

    const nextSnapshot = makeNextSnapshot();
    await sessionManager.importSnapshot(nextSnapshot);
    await flushWs();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].protocol).toEqual([
      "phoenix-jwt",
      nextSnapshot.accessToken,
    ]);

    client.dispose();
  });

  it("waits for an external session update after auth close", async () => {
    vi.useFakeTimers();

    stubWebSocket();
    const fetchMock = stubSuccessfulFetch();

    const sessionManager = new auth.AuthSessionManager(
      new auth.MemoryAuthSessionStorage()
    );
    const initialSnapshot = makeSnapshot({
      accessToken: makeJwt({
        sid: "sid-1",
        jti: "jti-1",
        exp: Math.floor((Date.now() + 120_000) / 1000),
      }),
      expiresAt: Date.now() + 120_000,
    });
    await sessionManager.importSnapshot(initialSnapshot);

    const client = createPhoenixClient({
      baseUrl: "https://example.com",
      auth: true,
      authConfig: {
        sessionManager,
        sessionControl: "external",
      },
      ws: {
        url: "wss://example.com/v1/ws",
        connectMode: "eager",
      },
    });

    await flushWs();

    expect(MockWebSocket.instances).toHaveLength(1);

    MockWebSocket.instances[0].onclose?.({
      code: 4401,
      reason: "invalid_access_token",
    });
    await flushWs();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(MockWebSocket.instances).toHaveLength(1);

    const nextSnapshot = makeNextSnapshot();
    await sessionManager.importSnapshot(nextSnapshot);
    await flushWs();

    expect(MockWebSocket.instances).toHaveLength(2);
    expect(MockWebSocket.instances[1].protocol).toEqual([
      "phoenix-jwt",
      nextSnapshot.accessToken,
    ]);

    client.dispose();
  });

  it("does not send auth-required subscriptions over an anonymous socket", async () => {
    vi.useFakeTimers();

    stubWebSocket();
    const fetchMock = stubSuccessfulFetch();

    const client = createExternalSessionClient();

    await flushWs();

    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].protocol).toBeUndefined();

    client.streams?.notifications({ authority: "authority-1" });
    await flushWs();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(sentMessages(MockWebSocket.instances[0])).toEqual([]);

    client.dispose();
  });

  it("still sends public subscriptions over an anonymous socket", async () => {
    vi.useFakeTimers();

    stubWebSocket();
    const fetchMock = stubSuccessfulFetch();

    const client = createExternalSessionClient();

    await flushWs();

    client.streams?.allMids();
    await flushWs();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(sentMessages(MockWebSocket.instances[0])).toEqual([
      {
        type: "subscribe",
        subscription: {
          channel: "allMids",
        },
      },
    ]);

    client.dispose();
  });

  it("keeps public subscriptions on the anonymous socket while private subscriptions wait for external auth", async () => {
    vi.useFakeTimers();

    stubWebSocket();
    const fetchMock = stubSuccessfulFetch();

    const sessionManager = createSessionManager();
    const client = createExternalSessionClient(sessionManager);

    await flushWs();

    client.streams?.notifications({ authority: "authority-1" });
    await flushWs();

    client.streams?.allMids();
    await flushWs();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].protocol).toBeUndefined();
    expect(sentMessages(MockWebSocket.instances[0])).toEqual([
      {
        type: "subscribe",
        subscription: {
          channel: "allMids",
        },
      },
    ]);

    const nextSnapshot = makeNextSnapshot();
    await sessionManager.importSnapshot(nextSnapshot);
    await flushWs();

    expect(MockWebSocket.instances).toHaveLength(2);
    expect(MockWebSocket.instances[1].protocol).toEqual([
      "phoenix-jwt",
      nextSnapshot.accessToken,
    ]);
    expect(sentMessages(MockWebSocket.instances[1])).toEqual([
      {
        type: "subscribe",
        subscription: {
          channel: "notifications",
          authority: "authority-1",
        },
      },
      {
        type: "subscribe",
        subscription: {
          channel: "allMids",
        },
      },
    ]);

    client.dispose();
  });

  it("sends deferred auth-required subscriptions after an external session appears", async () => {
    vi.useFakeTimers();

    stubWebSocket();
    const fetchMock = stubSuccessfulFetch();

    const sessionManager = createSessionManager();
    const client = createExternalSessionClient(sessionManager);

    await flushWs();

    client.streams?.notifications({ authority: "authority-1" });
    await flushWs();

    expect(MockWebSocket.instances).toHaveLength(1);
    expect(sentMessages(MockWebSocket.instances[0])).toEqual([]);

    const nextSnapshot = makeNextSnapshot();
    await sessionManager.importSnapshot(nextSnapshot);
    await flushWs();

    expect(fetchMock).not.toHaveBeenCalled();
    expect(MockWebSocket.instances).toHaveLength(2);
    expect(MockWebSocket.instances[1].protocol).toEqual([
      "phoenix-jwt",
      nextSnapshot.accessToken,
    ]);
    expect(sentMessages(MockWebSocket.instances[1])).toEqual([
      {
        type: "subscribe",
        subscription: {
          channel: "notifications",
          authority: "authority-1",
        },
      },
    ]);

    client.dispose();
  });
});
