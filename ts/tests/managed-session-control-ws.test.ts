import { afterEach, describe, expect, it, vi } from "vitest";
import type { AuthSessionSnapshot } from "@/auth/session";
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
  it("preserves the managed session when websocket auth refresh fails transiently", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const sessionManager = new AuthSessionManager(
      new MemoryAuthSessionStorage()
    );
    const snapshot = makeSnapshot();
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
    await expect(sessionManager.exportSnapshot()).resolves.toEqual(snapshot);

    ws.close();
  });

  it("still clears the managed session when refresh fails terminally", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);

    const sessionManager = new AuthSessionManager(
      new MemoryAuthSessionStorage()
    );
    await sessionManager.importSnapshot(makeSnapshot());

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
