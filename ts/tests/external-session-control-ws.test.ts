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

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe("rise external session control websocket integration", () => {
  it("waits for an external session update after auth close", async () => {
    vi.useFakeTimers();

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

    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);
    const fetchMock = vi.fn(async () => {
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const sessionManager = new auth.AuthSessionManager(
      new auth.MemoryAuthSessionStorage()
    );
    const initialSnapshot = makeSnapshot({
      accessToken: makeJwt({
        sid: "sid-1",
        jti: "jti-1",
        exp: Math.floor((Date.now() + 60_000) / 1000),
      }),
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

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(MockWebSocket.instances).toHaveLength(1);

    MockWebSocket.instances[0].onclose?.({
      code: 4401,
      reason: "invalid_access_token",
    });
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(fetchMock).not.toHaveBeenCalled();
    expect(MockWebSocket.instances).toHaveLength(1);

    const nextSnapshot = makeSnapshot({
      sessionId: "sid-2",
      accessJti: "jti-2",
      refreshToken: "refresh-token-2",
      popKey: Buffer.from(
        Uint8Array.from({ length: 32 }, (_, index) => index + 1)
      ).toString("base64url"),
      expiresAt: Date.now() + 120_000,
      refreshExpiresAt: Date.now() + 240_000,
    });
    await sessionManager.importSnapshot(nextSnapshot);
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(0);

    expect(MockWebSocket.instances).toHaveLength(2);
    expect(MockWebSocket.instances[1].protocol).toEqual([
      "phoenix-jwt",
      nextSnapshot.accessToken,
    ]);

    client.dispose();
  });
});
