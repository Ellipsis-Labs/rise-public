import { afterEach, describe, expect, it, vi } from "vitest";
import { auth, PhoenixHttpClient } from "@/index";
import type { AuthSessionSnapshot } from "@/auth/session";

const makeJwt = (payload: Record<string, unknown>): string =>
  `${Buffer.from(JSON.stringify({ alg: "none" })).toString("base64url")}.${Buffer.from(JSON.stringify(payload)).toString("base64url")}.sig`;

const makeSnapshot = (): AuthSessionSnapshot => {
  const expiresAt = Math.floor((Date.now() + 300_000) / 1000) * 1000;
  return {
    sessionId: "managed-session",
    accessToken: makeJwt({
      sid: "managed-session",
      jti: "jti",
      exp: expiresAt / 1000,
    }),
    refreshToken: "refresh-token",
    popKey: "",
    accessJti: "jti",
    expiresAt,
    refreshExpiresAt: expiresAt + 300_000,
    counter: "0",
  };
};

const externalSnapshot = (token: string): AuthSessionSnapshot => ({
  ...makeSnapshot(),
  sessionId: undefined,
  accessToken: token,
  refreshToken: "",
  accessJti: "",
});

const installBrowserStorage = () => {
  const values = new Map<string, string>();
  const listeners = new Set<(event: { key: string | null }) => void>();
  const localStorage = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
    removeItem: (key: string) => {
      values.delete(key);
    },
  };
  vi.stubGlobal("window", {
    localStorage,
    addEventListener: (
      _type: string,
      listener: (event: { key: string | null }) => void
    ) => {
      listeners.add(listener);
    },
    removeEventListener: (
      _type: string,
      listener: (event: { key: string | null }) => void
    ) => {
      listeners.delete(listener);
    },
  });
  return {
    localStorage,
    dispatchStorage: (key: string | null) => {
      for (const listener of listeners) listener({ key });
    },
  };
};

afterEach(() => {
  vi.unstubAllGlobals();
});

const externalTokens = [
  { name: "opaque", token: "opaque-external-token" },
  {
    name: "JWT without sid/jti",
    token: makeJwt({ exp: Math.floor((Date.now() + 300_000) / 1000) }),
  },
  { name: "JWT without sid/jti/exp", token: makeJwt({}) },
];

describe("managed snapshot restoration boundaries", () => {
  it.each([
    {
      name: "opaque",
      token: "opaque",
      sessionId: "snapshot-sid",
      error: "Invalid JWT format",
    },
    {
      name: "missing sid",
      token: makeJwt({ exp: 4_000_000_000 }),
      sessionId: undefined,
      error: "Missing sid in access token snapshot",
    },
    {
      name: "empty sid",
      token: makeJwt({ sid: "", exp: 4_000_000_000 }),
      sessionId: "",
      error: "Missing sid in access token snapshot",
    },
    {
      name: "missing JWT exp",
      token: makeJwt({ sid: "sid" }),
      sessionId: "snapshot-sid",
      error: "Missing exp in access token snapshot",
    },
  ])(
    "rejects $name in managed parsers, imports and initial restoration",
    async ({ token, sessionId, error }) => {
      const snapshot = { ...makeSnapshot(), accessToken: token, sessionId };
      expect(() => auth.fromSnapshot(snapshot)).toThrow(error);
      for (const sessionControl of [undefined, "managed"] as const) {
        const manager = new auth.AuthSessionManager(
          new auth.MemoryAuthSessionStorage(),
          undefined,
          { sessionControl }
        );
        const previous = makeSnapshot();
        await manager.importSnapshot(previous);
        await expect(manager.importSnapshot(snapshot)).rejects.toThrow(error);
        await expect(manager.exportSnapshot()).resolves.toEqual(previous);
        expect(
          () =>
            new PhoenixHttpClient({
              apiUrl: "https://example.com",
              auth: true,
              authConfig: { initialSession: snapshot, sessionControl },
            })
        ).toThrow(error);
        manager.dispose();
      }
    }
  );

  it("retains managed snapshot session-id fallback and JWT expiry precedence", () => {
    const snapshot = makeSnapshot();
    const token = makeJwt({ exp: snapshot.expiresAt / 1000 });
    expect(
      auth.fromSnapshot({ ...snapshot, accessToken: token, expiresAt: 0 })
    ).toMatchObject({
      sessionId: snapshot.sessionId,
      expiresAt: snapshot.expiresAt,
    });
    expect(
      auth.fromSnapshot({ ...snapshot, sessionId: undefined }).sessionId
    ).toBe("managed-session");
  });
});

describe("external snapshot restoration boundaries", () => {
  it.each(externalTokens)(
    "imports $name with explicitly external managers",
    async ({ token }) => {
      const manager = new auth.AuthSessionManager(
        new auth.MemoryAuthSessionStorage(),
        undefined,
        { sessionControl: "external" }
      );
      const snapshot = externalSnapshot(token);
      await manager.importSnapshot(snapshot);
      await expect(manager.getSession()).resolves.toEqual(
        auth.fromExternalSnapshot(snapshot)
      );
      await expect(
        manager.importSnapshot({ ...snapshot, accessToken: "" })
      ).rejects.toThrow("Access token is empty");
      await expect(manager.getSession()).resolves.toEqual(
        auth.fromExternalSnapshot(snapshot)
      );
      manager.dispose();
    }
  );

  it.each(externalTokens)(
    "restores initial $name snapshots and configures runtime-owned imports",
    async ({ token }) => {
      const snapshot = externalSnapshot(token);
      const fetchMock = vi.fn<typeof fetch>(
        async () => new Response("{}", { status: 200 })
      );
      vi.stubGlobal("fetch", fetchMock);
      const client = new PhoenixHttpClient({
        apiUrl: "https://example.com",
        auth: true,
        authConfig: {
          sessionControl: "external",
          storage: new auth.MemoryAuthSessionStorage(),
          initialSession: snapshot,
        },
      });
      await client.fetch("GET", "/v1/protected", { auth: "required" });
      expect(
        new Headers(fetchMock.mock.calls[0]?.[1]?.headers).get("Authorization")
      ).toBe(`Bearer ${token}`);
      const manager = client.sessionManager();
      if (!manager) throw new Error("Expected runtime-owned session manager");
      await manager.importSnapshot(
        externalSnapshot("replacement-opaque-token")
      );
      await client.fetch("GET", "/v1/protected", { auth: "required" });
      expect(
        new Headers(fetchMock.mock.calls[1]?.[1]?.headers).get("Authorization")
      ).toBe("Bearer replacement-opaque-token");
      expect(fetchMock).toHaveBeenCalledTimes(2);
      manager.dispose();
    }
  );
});

describe("local storage snapshot restoration", () => {
  it.each(externalTokens)(
    "hydrates external $name tokens for authenticated requests",
    async ({ token }) => {
      const { localStorage } = installBrowserStorage();
      const snapshot = externalSnapshot(token);
      localStorage.setItem(
        "phoenix-rise-auth-external",
        JSON.stringify(snapshot)
      );
      const fetchMock = vi.fn<typeof fetch>(
        async () => new Response("{}", { status: 200 })
      );
      vi.stubGlobal("fetch", fetchMock);
      const client = new PhoenixHttpClient({
        apiUrl: "https://example.com",
        auth: true,
        authConfig: {
          sessionControl: "external",
          storage: new auth.LocalStorageAuthSessionStorage(undefined, {
            sessionControl: "external",
          }),
        },
      });
      await client.fetch("GET", "/v1/protected", { auth: "required" });
      expect(
        new Headers(fetchMock.mock.calls[0]?.[1]?.headers).get("Authorization")
      ).toBe(`Bearer ${token}`);
      expect(fetchMock).toHaveBeenCalledTimes(1);
      client.sessionManager()?.dispose();
    }
  );

  it.each([undefined, "managed", "external"] as const)(
    "applies identical get/event parsing and cleanup in %s mode",
    async (sessionControl) => {
      const { localStorage, dispatchStorage } = installBrowserStorage();
      const key = "custom-auth-key";
      const storage = new auth.LocalStorageAuthSessionStorage(key, {
        sessionControl,
      });
      const listener = vi.fn();
      const unsubscribe = storage.subscribe(listener);
      for (const snapshot of [
        makeSnapshot(),
        ...externalTokens.map(({ token }) => externalSnapshot(token)),
      ]) {
        const serialized = JSON.stringify(snapshot);
        localStorage.setItem(key, serialized);
        const fromGet = await storage.get();
        const retainedFromGet = localStorage.getItem(key);
        localStorage.setItem(key, serialized);
        dispatchStorage("unrelated-key");
        expect(listener).not.toHaveBeenCalled();
        dispatchStorage(key);
        expect(listener).toHaveBeenLastCalledWith(fromGet);
        expect(localStorage.getItem(key)).toBe(retainedFromGet);
        if (sessionControl !== "external" && snapshot.sessionId === undefined) {
          expect(fromGet).toBeNull();
        } else {
          expect(fromGet?.accessToken).toBe(snapshot.accessToken);
        }
        listener.mockClear();
      }
      for (const serialized of [
        "{",
        JSON.stringify({ ...externalSnapshot("opaque"), accessToken: "" }),
      ]) {
        localStorage.setItem(key, serialized);
        await expect(storage.get()).resolves.toBeNull();
        expect(localStorage.getItem(key)).toBeNull();
        localStorage.setItem(key, serialized);
        dispatchStorage(key);
        expect(listener).toHaveBeenLastCalledWith(null);
        expect(localStorage.getItem(key)).toBeNull();
      }
      localStorage.removeItem(key);
      dispatchStorage(null);
      expect(listener).toHaveBeenLastCalledWith(null);
      unsubscribe();
      listener.mockClear();
      dispatchStorage(key);
      expect(listener).not.toHaveBeenCalled();
    }
  );

  it("isolates default external persistence from managed reads, writes, events and clears", async () => {
    const { localStorage, dispatchStorage } = installBrowserStorage();
    const managed = new auth.LocalStorageAuthSessionStorage();
    const external = new auth.LocalStorageAuthSessionStorage(undefined, {
      sessionControl: "external",
    });
    const managedSnapshot = makeSnapshot();
    const externalSession = auth.fromExternalSnapshot(
      externalSnapshot("opaque")
    );
    await managed.set(auth.fromSnapshot(managedSnapshot));
    await external.set(externalSession);
    await expect(managed.get()).resolves.toEqual(
      auth.fromSnapshot(managedSnapshot)
    );
    await expect(external.get()).resolves.toEqual(externalSession);
    const managedListener = vi.fn();
    const externalListener = vi.fn();
    const stopManaged = managed.subscribe(managedListener);
    const stopExternal = external.subscribe(externalListener);
    dispatchStorage("phoenix-rise-auth-external");
    expect(managedListener).not.toHaveBeenCalled();
    expect(externalListener).toHaveBeenLastCalledWith(externalSession);
    localStorage.setItem(
      "phoenix-rise-auth",
      JSON.stringify(externalSnapshot("opaque"))
    );
    await expect(managed.get()).resolves.toBeNull();
    await expect(external.get()).resolves.toEqual(externalSession);
    await managed.clear();
    await expect(external.get()).resolves.toEqual(externalSession);
    stopManaged();
    stopExternal();
  });

  it.each(externalTokens)(
    "discards external $name snapshots before managed requests or refreshes",
    async ({ token }) => {
      const { localStorage } = installBrowserStorage();
      const key = "phoenix-rise-auth";
      localStorage.setItem(key, JSON.stringify(externalSnapshot(token)));
      const fetchMock = vi.fn<typeof fetch>();
      vi.stubGlobal("fetch", fetchMock);
      const client = new PhoenixHttpClient({
        apiUrl: "https://example.com",
        auth: true,
      });

      await expect(
        client.fetch("GET", "/v1/protected", { auth: "required" })
      ).rejects.toMatchObject({
        name: "PhoenixAuthError",
        code: "no_auth_session",
      });
      expect(fetchMock).not.toHaveBeenCalled();
      expect(localStorage.getItem(key)).toBeNull();
      await expect(client.exportAuthSnapshot()).resolves.toBeNull();
      client.sessionManager()?.dispose();
    }
  );

  it.each([
    { name: "opaque", token: "opaque-external-token" },
    { name: "JWT without exp", token: makeJwt({ sid: "external-session" }) },
  ])(
    "invalidates cached managed sessions when cross-tab storage changes to $name",
    async ({ token }) => {
      const { localStorage, dispatchStorage } = installBrowserStorage();
      const key = "phoenix-rise-auth";
      const snapshot = makeSnapshot();
      localStorage.setItem(key, JSON.stringify(snapshot));
      const fetchMock = vi.fn<typeof fetch>();
      vi.stubGlobal("fetch", fetchMock);
      const client = new PhoenixHttpClient({
        apiUrl: "https://example.com",
        auth: true,
      });
      await expect(client.exportAuthSnapshot()).resolves.toEqual(snapshot);
      const manager = client.sessionManager();
      if (!manager) throw new Error("Expected runtime-owned session manager");
      const listener = vi.fn();
      manager.onChange(listener);

      localStorage.setItem(
        key,
        JSON.stringify({
          ...externalSnapshot(token),
          sessionId: "external-session",
        })
      );
      dispatchStorage(key);

      expect(listener).toHaveBeenCalledExactlyOnceWith(null);
      expect(localStorage.getItem(key)).toBeNull();
      await expect(client.exportAuthSnapshot()).resolves.toBeNull();
      await expect(
        client.fetch("GET", "/v1/protected", { auth: "required" })
      ).rejects.toMatchObject({
        name: "PhoenixAuthError",
        code: "no_auth_session",
      });
      expect(fetchMock).not.toHaveBeenCalled();
      manager.dispose();
    }
  );
});
