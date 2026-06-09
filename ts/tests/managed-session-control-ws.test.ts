import { afterEach, describe, expect, it, vi } from "vitest";
import {
  fromSnapshot,
  type AuthSession,
  type AuthSessionSnapshot,
} from "@/auth/session";
import { AuthSessionManager } from "@/auth/manager";
import { MemoryAuthSessionStorage } from "@/auth/storage";
import { createWsClient } from "@/ws/WsClient";

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

const waitUntil = async (
  predicate: () => boolean,
  message: string
): Promise<void> => {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error(message);
};

const flushMicrotasks = async (count = 5): Promise<void> => {
  for (let i = 0; i < count; i += 1) {
    await Promise.resolve();
  }
};

const flushScheduledAuthRefreshRetry = async (): Promise<void> => {
  await vi.runOnlyPendingTimersAsync();
  await flushMicrotasks();
  await vi.runOnlyPendingTimersAsync();
};

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

describe("rise managed websocket session handling", () => {
  it("refreshes an expired auto-auth session before connecting", async () => {
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const sessionManager = new AuthSessionManager(
      new MemoryAuthSessionStorage()
    );
    await sessionManager.importSnapshot(
      makeSnapshot({ expiresAt: Date.now() - 1_000 })
    );

    const freshSession: AuthSession = fromSnapshot(
      makeSnapshot({
        accessToken: makeJwt({
          sid: "sid-1",
          jti: "fresh-jti",
          exp: Math.floor((Date.now() + 60_000) / 1000),
        }),
        accessJti: "fresh-jti",
        expiresAt: Date.now() + 60_000,
      })
    );
    const refreshFn = vi.fn(async () => freshSession);

    const ws = createWsClient({
      url: "wss://example.com/v1/ws",
      sessionManager,
      refreshFn,
    });

    await waitUntil(
      () => refreshFn.mock.calls.length === 1,
      "websocket auth refresh was not attempted"
    );

    expect(refreshFn).toHaveBeenCalledTimes(1);
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].protocol).toEqual([
      "phoenix-jwt",
      freshSession.accessToken,
    ]);

    ws.close();
  });

  it("refreshes an expiring auto-auth session before connecting", async () => {
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const sessionManager = new AuthSessionManager(
      new MemoryAuthSessionStorage()
    );
    await sessionManager.importSnapshot(
      makeSnapshot({ expiresAt: Date.now() + 30_000 })
    );

    const freshSession: AuthSession = fromSnapshot(
      makeSnapshot({
        accessToken: makeJwt({
          sid: "sid-1",
          jti: "fresh-jti",
          exp: Math.floor((Date.now() + 120_000) / 1000),
        }),
        accessJti: "fresh-jti",
        expiresAt: Date.now() + 120_000,
      })
    );
    const refreshFn = vi.fn(async () => freshSession);

    const ws = createWsClient({
      url: "wss://example.com/v1/ws",
      sessionManager,
      refreshFn,
    });

    await waitUntil(
      () => refreshFn.mock.calls.length === 1,
      "websocket auth refresh was not attempted"
    );

    expect(refreshFn).toHaveBeenCalledTimes(1);
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].protocol).toEqual([
      "phoenix-jwt",
      freshSession.accessToken,
    ]);

    ws.close();
  });

  it("retries an expiring authenticated websocket refresh after retry-after", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const sessionManager = new AuthSessionManager(
      new MemoryAuthSessionStorage()
    );
    await sessionManager.importSnapshot(
      makeSnapshot({ expiresAt: Date.now() + 30_000 })
    );

    const freshSession: AuthSession = fromSnapshot(
      makeSnapshot({
        accessToken: makeJwt({
          sid: "sid-1",
          jti: "fresh-jti",
          exp: Math.floor((Date.now() + 120_000) / 1000),
        }),
        accessJti: "fresh-jti",
        expiresAt: Date.now() + 120_000,
      })
    );
    const refreshFn = vi
      .fn()
      .mockRejectedValueOnce(
        Object.assign(new Error("rate_limited"), {
          code: "rate_limited",
          retryAfterMs: 2_000,
        })
      )
      .mockResolvedValueOnce(freshSession);

    const ws = createWsClient({
      url: "wss://example.com/v1/ws",
      sessionManager,
      refreshFn,
      authMode: "authenticated",
    });

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(refreshFn).toHaveBeenCalledTimes(1);
    expect(MockWebSocket.instances).toHaveLength(0);

    await flushMicrotasks();
    expect(vi.getTimerCount()).toBeGreaterThan(0);

    await vi.advanceTimersByTimeAsync(1_999);
    expect(refreshFn).toHaveBeenCalledTimes(1);
    expect(MockWebSocket.instances).toHaveLength(0);

    await flushScheduledAuthRefreshRetry();

    expect(refreshFn).toHaveBeenCalledTimes(2);
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].protocol).toEqual([
      "phoenix-jwt",
      freshSession.accessToken,
    ]);

    ws.close();
  });

  it("preserves the managed session when websocket auth refresh fails transiently", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const sessionManager = new AuthSessionManager(
      new MemoryAuthSessionStorage()
    );
    const snapshot = makeSnapshot({ expiresAt: Date.now() + 120_000 });
    await sessionManager.importSnapshot(snapshot);

    const refreshFn = vi.fn(async () => {
      throw new Error("network_error");
    });

    const ws = createWsClient({
      url: "wss://example.com/v1/ws",
      sessionManager,
      refreshFn,
    });

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(MockWebSocket.instances).toHaveLength(1);

    MockWebSocket.instances[0].onclose?.({
      code: 4401,
      reason: "invalid_access_token",
    });
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(refreshFn).toHaveBeenCalledTimes(1);
    await expect(sessionManager.exportSnapshot()).resolves.toEqual({
      ...snapshot,
      expiresAt: Math.floor(snapshot.expiresAt / 1000) * 1000,
    });

    ws.close();
  });

  it("still clears the managed session when refresh fails terminally", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const sessionManager = new AuthSessionManager(
      new MemoryAuthSessionStorage()
    );
    await sessionManager.importSnapshot(
      makeSnapshot({ expiresAt: Date.now() + 120_000 })
    );

    const refreshFn = vi.fn(async () => {
      const error = new Error("session_missing") as Error & { code: string };
      error.code = "session_missing";
      throw error;
    });

    const ws = createWsClient({
      url: "wss://example.com/v1/ws",
      sessionManager,
      refreshFn,
    });

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    MockWebSocket.instances[0].onclose?.({
      code: 4401,
      reason: "invalid_access_token",
    });
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(refreshFn).toHaveBeenCalledTimes(1);
    await expect(sessionManager.exportSnapshot()).resolves.toBeNull();

    ws.close();
  });
});
