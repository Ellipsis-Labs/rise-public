import type { AuthSession } from "./session";
import { fromSnapshot, toSnapshot, type AuthSessionSnapshot } from "./session";

export interface AuthSessionStorage {
  get(): Promise<AuthSession | null>;
  set(session: AuthSession): Promise<void>;
  clear(): Promise<void>;
}

export interface AuthSessionStorageSubscriber {
  subscribe(listener: (session: AuthSession | null) => void): () => void;
}

export interface AuthSessionRefreshLock {
  withRefreshLock<T>(fn: () => Promise<T>): Promise<T>;
}

const DEFAULT_STORAGE_LOCK_TTL_MS = 2_000;
const DEFAULT_REFRESH_LOCK_TTL_MS = 35_000;

export class MemoryAuthSessionStorage implements AuthSessionStorage {
  private session: AuthSession | null = null;

  async get(): Promise<AuthSession | null> {
    return this.session;
  }

  async set(session: AuthSession): Promise<void> {
    this.session = session;
  }

  async clear(): Promise<void> {
    this.session = null;
  }
}

export class LocalStorageAuthSessionStorage
  implements AuthSessionStorage, AuthSessionStorageSubscriber
{
  private readonly storageKey: string;
  private readonly lockKey: string;
  private readonly refreshLockKey: string;

  constructor(storageKey = "phoenix-rise-auth") {
    this.storageKey = storageKey;
    this.lockKey = `${storageKey}:lock`;
    this.refreshLockKey = `${storageKey}:refresh-lock`;
    if (typeof window === "undefined" || !window.localStorage) {
      throw new Error("LocalStorageAuthSessionStorage requires a browser");
    }
  }

  private async withLease<T>(
    key: string,
    ttlMs: number,
    fn: () => Promise<T>
  ): Promise<T> {
    const owner = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const deadline = Date.now() + ttlMs;
    const sleep = (ms: number) =>
      new Promise<void>((resolve) => setTimeout(resolve, ms));

    const tryAcquire = (): boolean => {
      try {
        const raw = window.localStorage.getItem(key);
        if (raw) {
          const parsed = JSON.parse(raw) as {
            owner?: string;
            expiresAt?: number;
          };
          if (
            typeof parsed.expiresAt === "number" &&
            parsed.expiresAt > Date.now()
          ) {
            return false;
          }
        }

        const payload = JSON.stringify({
          owner,
          expiresAt: Date.now() + ttlMs,
        });
        window.localStorage.setItem(key, payload);
        const confirm = window.localStorage.getItem(key);
        if (!confirm) return false;
        const confirmed = JSON.parse(confirm) as { owner?: string };
        return confirmed.owner === owner;
      } catch {
        return false;
      }
    };

    while (!tryAcquire()) {
      if (Date.now() > deadline) {
        throw new Error("Auth session lock timeout");
      }
      await sleep(25 + Math.floor(Math.random() * 25));
    }

    try {
      return await fn();
    } finally {
      try {
        const raw = window.localStorage.getItem(key);
        if (raw) {
          const parsed = JSON.parse(raw) as { owner?: string };
          if (parsed.owner === owner) {
            window.localStorage.removeItem(key);
          }
        }
      } catch {
        // Ignore lock cleanup failures.
      }
    }
  }

  async withLock<T>(fn: () => Promise<T>): Promise<T> {
    if (typeof window === "undefined" || !window.localStorage) {
      return fn();
    }
    return this.withLease(this.lockKey, DEFAULT_STORAGE_LOCK_TTL_MS, fn);
  }

  async withRefreshLock<T>(fn: () => Promise<T>): Promise<T> {
    if (typeof window === "undefined" || !window.localStorage) {
      return fn();
    }
    return this.withLease(this.refreshLockKey, DEFAULT_REFRESH_LOCK_TTL_MS, fn);
  }

  async get(): Promise<AuthSession | null> {
    const raw = window.localStorage.getItem(this.storageKey);
    if (!raw) return null;
    try {
      return fromSnapshot(JSON.parse(raw) as AuthSessionSnapshot);
    } catch {
      try {
        window.localStorage.removeItem(this.storageKey);
      } catch {
        // Ignore localStorage removal failures.
      }
      return null;
    }
  }

  async set(session: AuthSession): Promise<void> {
    window.localStorage.setItem(
      this.storageKey,
      JSON.stringify(toSnapshot(session))
    );
  }

  async clear(): Promise<void> {
    window.localStorage.removeItem(this.storageKey);
  }

  subscribe(listener: (session: AuthSession | null) => void): () => void {
    if (
      typeof window === "undefined" ||
      typeof window.addEventListener !== "function" ||
      typeof window.removeEventListener !== "function"
    ) {
      return () => undefined;
    }

    const handleStorage = (event: StorageEvent) => {
      if (event.key !== this.storageKey && event.key !== null) return;
      const serialized = window.localStorage.getItem(this.storageKey);
      if (!serialized) {
        listener(null);
        return;
      }

      try {
        listener(fromSnapshot(JSON.parse(serialized) as AuthSessionSnapshot));
      } catch {
        listener(null);
      }
    };

    window.addEventListener("storage", handleStorage);
    return () => {
      window.removeEventListener("storage", handleStorage);
    };
  }
}
