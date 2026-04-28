import { afterEach, describe, expect, it, vi } from "vitest";

import { PhoenixHttpClient } from "@/client";
import type { AuthSessionSnapshot } from "@/auth/session";
import { auth } from "@/index";

type LocalStorageLike = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
};

const toBase64Url = (value: unknown): string =>
  Buffer.from(JSON.stringify(value)).toString("base64url");

const makeJwt = (payload: { sid: string; jti: string; exp: number }): string =>
  `${toBase64Url({ alg: "none", typ: "JWT" })}.${toBase64Url(payload)}.sig`;

const makeSnapshot = (
  overrides: Partial<AuthSessionSnapshot> = {}
): AuthSessionSnapshot => {
  const sid = overrides.sessionId ?? "sid-1";
  const accessJti = overrides.accessJti ?? "jti-1";
  const expiresAt = overrides.expiresAt ?? Date.now() + 5 * 60_000;

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
    refreshExpiresAt: overrides.refreshExpiresAt ?? Date.now() + 60 * 60_000,
    counter: overrides.counter ?? "0",
  };
};

const makeAuthResponse = (
  overrides: {
    sid?: string;
    jti?: string;
    expiresIn?: number;
    refreshToken?: string;
    refreshExpiresIn?: number;
    popKey?: string;
  } = {}
) => {
  const expiresIn = overrides.expiresIn ?? 15 * 60;

  return {
    token_type: "Bearer" as const,
    access_token: makeJwt({
      sid: overrides.sid ?? "sid-1",
      jti: overrides.jti ?? "jti-2",
      exp: Math.floor((Date.now() + expiresIn * 1000) / 1000),
    }),
    expires_in: expiresIn,
    refresh_token: overrides.refreshToken ?? "refresh-token-2",
    refresh_expires_in: overrides.refreshExpiresIn ?? 60 * 60,
    pop_key: overrides.popKey ?? Buffer.from("pop-key-2").toString("base64url"),
  };
};

const createLocalStorage = (): LocalStorageLike => {
  const store = new Map<string, string>();

  return {
    getItem: (key) => store.get(key) ?? null,
    setItem: (key, value) => {
      store.set(key, value);
    },
    removeItem: (key) => {
      store.delete(key);
    },
  };
};

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("rise auth config", () => {
  it("hydrates the default browser session from localStorage when auth is enabled", async () => {
    const localStorage = createLocalStorage();
    const snapshot = makeSnapshot();
    localStorage.setItem("phoenix-rise-auth", JSON.stringify(snapshot));

    vi.stubGlobal("window", {
      localStorage,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    });

    const client = new PhoenixHttpClient({
      apiUrl: "https://example.com",
      auth: true,
    });

    await expect(client.exportAuthSnapshot()).resolves.toEqual(snapshot);
  });

  it("refreshes on the first authenticated request when the access token is near expiry", async () => {
    const initialSnapshot = makeSnapshot({
      accessJti: "jti-old",
      expiresAt: Date.now() + 10_000,
    });
    const refreshed = makeAuthResponse({
      sid: initialSnapshot.sessionId,
      jti: "jti-new",
    });

    const fetchMock = vi.fn(
      async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = String(input);

        if (url.endsWith("/v1/auth/refresh")) {
          return new Response(JSON.stringify(refreshed), {
            status: 200,
            headers: { "content-type": "application/json" },
          });
        }

        return new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      auth: true,
      authConfig: {
        initialSession: initialSnapshot,
      },
    });

    await client.fetch("GET", "/v1/protected", { auth: "required" });

    expect(fetchMock).toHaveBeenCalledTimes(2);

    const refreshHeaders = fetchMock.mock.calls[0]?.[1]?.headers as
      | Record<string, string>
      | undefined;
    const protectedHeaders = fetchMock.mock.calls[1]?.[1]?.headers as
      | Record<string, string>
      | undefined;

    expect(String(fetchMock.mock.calls[0]?.[0])).toBe(
      "https://example.com/v1/auth/refresh"
    );
    expect(refreshHeaders?.Authorization).toBe(
      `Bearer ${initialSnapshot.accessToken}`
    );
    expect(protectedHeaders?.Authorization).toBe(
      `Bearer ${refreshed.access_token}`
    );
    await expect(client.exportAuthSnapshot()).resolves.toEqual(
      expect.objectContaining({
        accessToken: refreshed.access_token,
        refreshToken: refreshed.refresh_token,
        accessJti: "jti-new",
      })
    );
  });

  it("sends the request even when auth is requested but no auth runtime is configured", async () => {
    const fetchMock = vi.fn(async () => {
      return new Response(JSON.stringify({ error: "missing_access_token" }), {
        status: 401,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
    });

    const response = await client.fetch("GET", "/v1/protected", {
      auth: "required",
    });

    expect(response.status).toBe(401);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(String(fetchMock.mock.calls[0]?.[0])).toBe(
      "https://example.com/v1/protected"
    );
    const headers = fetchMock.mock.calls[0]?.[1]?.headers as
      | Record<string, string>
      | undefined;
    expect(headers?.Authorization).toBeUndefined();
  });

  it("returns the unauthorized response when refresh fails instead of retrying anonymously", async () => {
    const initialSnapshot = makeSnapshot({
      accessJti: "jti-old",
      expiresAt: Date.now() + 5 * 60_000,
    });

    const fetchMock = vi.fn(
      async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = String(input);

        if (url.endsWith("/v1/auth/refresh")) {
          return new Response(
            JSON.stringify({ error: "invalid_refresh_token" }),
            {
              status: 401,
              headers: { "content-type": "application/json" },
            }
          );
        }

        return new Response(JSON.stringify({ error: "invalid_access_token" }), {
          status: 401,
          headers: { "content-type": "application/json" },
        });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      auth: true,
      authConfig: {
        initialSession: initialSnapshot,
      },
    });

    const response = await client.fetch("GET", "/v1/protected", {
      auth: "required",
    });

    expect(response.status).toBe(401);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(String(fetchMock.mock.calls[0]?.[0])).toBe(
      "https://example.com/v1/protected"
    );
    expect(String(fetchMock.mock.calls[1]?.[0])).toBe(
      "https://example.com/v1/auth/refresh"
    );

    const protectedHeaders = fetchMock.mock.calls[0]?.[1]?.headers as
      | Record<string, string>
      | undefined;
    expect(protectedHeaders?.Authorization).toBe(
      `Bearer ${initialSnapshot.accessToken}`
    );
    await expect(client.exportAuthSnapshot()).resolves.toBeNull();
  });

  it("rejects authConfig when auth is not enabled", () => {
    expect(
      () =>
        new PhoenixHttpClient({
          baseUrl: "https://example.com",
          authConfig: {},
        })
    ).toThrow("authConfig requires auth: true");
  });

  it("rejects conflicting apiUrl and deprecated baseUrl values", () => {
    expect(
      () =>
        new PhoenixHttpClient({
          apiUrl: "https://example.com",
          baseUrl: "https://other.example.com",
        })
    ).toThrow(
      "apiUrl and deprecated baseUrl must match when both are provided"
    );
  });

  it("requires an explicit manager or storage for external session control", () => {
    expect(
      () =>
        new PhoenixHttpClient({
          baseUrl: "https://example.com",
          auth: true,
          authConfig: {
            sessionControl: "external",
          },
        })
    ).toThrow(
      'authConfig.sessionControl="external" requires authConfig.sessionManager or authConfig.storage'
    );
  });

  it("accepts managed session control for compatibility", () => {
    expect(
      () =>
        new PhoenixHttpClient({
          baseUrl: "https://example.com",
          auth: true,
          authConfig: {
            sessionControl: "managed",
          },
        })
    ).not.toThrow();
  });

  it("does not refresh near-expiry sessions when session control is external", async () => {
    const sessionManager = new auth.AuthSessionManager(
      new auth.MemoryAuthSessionStorage()
    );
    const snapshot = makeSnapshot({
      accessJti: "jti-old",
      expiresAt: Date.now() + 10_000,
    });
    await sessionManager.importSnapshot(snapshot);

    const fetchMock = vi.fn(async () => {
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      auth: true,
      authConfig: {
        sessionManager,
        sessionControl: "external",
      },
    });

    await client.fetch("GET", "/v1/protected", { auth: "required" });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(String(fetchMock.mock.calls[0]?.[0])).toBe(
      "https://example.com/v1/protected"
    );

    const protectedHeaders = fetchMock.mock.calls[0]?.[1]?.headers as
      | Record<string, string>
      | undefined;
    expect(protectedHeaders?.Authorization).toBe(
      `Bearer ${snapshot.accessToken}`
    );
    await expect(client.exportAuthSnapshot()).resolves.toEqual(snapshot);
  });

  it("does not clear externally managed sessions when the refresh token is expired", async () => {
    const sessionManager = new auth.AuthSessionManager(
      new auth.MemoryAuthSessionStorage()
    );
    const snapshot = makeSnapshot({
      expiresAt: Date.now() + 60_000,
      refreshExpiresAt: Date.now() - 1_000,
    });
    await sessionManager.importSnapshot(snapshot);

    const fetchMock = vi.fn(async () => {
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      auth: true,
      authConfig: {
        sessionManager,
        sessionControl: "external",
      },
    });

    await client.fetch("GET", "/v1/protected", { auth: "required" });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    await expect(client.exportAuthSnapshot()).resolves.toEqual(snapshot);
  });

  it("surfaces invalid_access_token without refreshing when session control is external", async () => {
    const sessionManager = new auth.AuthSessionManager(
      new auth.MemoryAuthSessionStorage()
    );
    const snapshot = makeSnapshot();
    await sessionManager.importSnapshot(snapshot);

    const fetchMock = vi.fn(async () => {
      return new Response(JSON.stringify({ error: "invalid_access_token" }), {
        status: 401,
        headers: { "content-type": "application/json" },
      });
    });
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      auth: true,
      authConfig: {
        sessionManager,
        sessionControl: "external",
      },
    });

    const response = await client.fetch("GET", "/v1/protected", {
      auth: "required",
    });

    expect(response.status).toBe(401);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(String(fetchMock.mock.calls[0]?.[0])).toBe(
      "https://example.com/v1/protected"
    );
    await expect(client.exportAuthSnapshot()).resolves.toEqual(snapshot);
  });

  it("falls back to memory storage when browser localStorage is not writable", async () => {
    const snapshot = makeSnapshot();

    vi.stubGlobal("window", {
      localStorage: {
        getItem: vi.fn(() => null),
        setItem: vi.fn(() => {
          throw new Error("quota exceeded");
        }),
        removeItem: vi.fn(),
      },
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    });

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      auth: true,
    });

    const sessionManager = client.sessionManager();
    expect(sessionManager).toBeDefined();

    await sessionManager!.importSnapshot(snapshot);
    await expect(client.exportAuthSnapshot()).resolves.toEqual(snapshot);
  });
});
