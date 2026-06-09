import type { AuthSession, AuthSessionSnapshot } from "./session";
import { fromSnapshot, isRefreshExpired, toSnapshot } from "./session";
import type {
  AuthSessionRefreshLock,
  AuthSessionStorage,
  AuthSessionStorageSubscriber,
} from "./storage";
import { shouldClearSessionOnRefreshFailure } from "./refreshErrors";

type StorageWithLock = {
  withLock<T>(fn: () => Promise<T>): Promise<T>;
};

const hasStorageLock = (
  storage: AuthSessionStorage
): storage is AuthSessionStorage & StorageWithLock => {
  const candidate = storage as Partial<StorageWithLock>;
  return typeof candidate.withLock === "function";
};

const hasRefreshLock = (
  storage: AuthSessionStorage
): storage is AuthSessionStorage & AuthSessionRefreshLock => {
  const candidate = storage as Partial<AuthSessionRefreshLock>;
  return typeof candidate.withRefreshLock === "function";
};

const hasStorageSubscription = (
  storage: AuthSessionStorage
): storage is AuthSessionStorage & AuthSessionStorageSubscriber => {
  const candidate = storage as Partial<AuthSessionStorageSubscriber>;
  return typeof candidate.subscribe === "function";
};

class AsyncMutex {
  private current: Promise<void> = Promise.resolve();

  async runExclusive<T>(fn: () => Promise<T>): Promise<T> {
    let release: () => void = () => {};
    const next = new Promise<void>((resolve) => {
      release = resolve;
    });
    const prev = this.current;
    this.current = prev.then(() => next);
    await prev;
    try {
      return await fn();
    } finally {
      release();
    }
  }
}

export class AuthSessionManager {
  private readonly mutex = new AsyncMutex();
  private refreshInFlight: Promise<AuthSession> | null = null;
  private cachedSession: AuthSession | null | undefined;
  private readonly listeners = new Set<(session: AuthSession | null) => void>();
  private storageUnsubscribe?: () => void;

  constructor(
    protected readonly storage: AuthSessionStorage,
    initialSession?: AuthSession
  ) {
    this.cachedSession = initialSession;
    if (initialSession) {
      void this.storage.set(initialSession);
    }
    if (hasStorageSubscription(storage)) {
      this.storageUnsubscribe = storage.subscribe((session) => {
        if (sessionsEqual(this.cachedSession ?? null, session)) return;
        this.cachedSession = session;
        this.notify(session);
      });
    }
  }

  dispose(): void {
    this.storageUnsubscribe?.();
    this.storageUnsubscribe = undefined;
    this.listeners.clear();
  }

  protected async withStorageLock<T>(fn: () => Promise<T>): Promise<T> {
    if (hasStorageLock(this.storage)) {
      return this.storage.withLock(fn);
    }
    return fn();
  }

  protected async withRefreshLock<T>(fn: () => Promise<T>): Promise<T> {
    if (hasRefreshLock(this.storage)) {
      return this.storage.withRefreshLock(fn);
    }
    return this.withStorageLock(fn);
  }

  protected async readSessionFromStorage(): Promise<AuthSession | null> {
    return this.storage.get();
  }

  protected setCachedSession(session: AuthSession | null): AuthSession | null {
    this.cachedSession = session;
    return session;
  }

  async getSession(): Promise<AuthSession | null> {
    if (this.cachedSession !== undefined) {
      return this.cachedSession;
    }
    return this.setCachedSession(await this.storage.get());
  }

  async updateSession(session: AuthSession): Promise<void> {
    await this.withStorageLock(async () => {
      await this.storage.set(session);
      this.cachedSession = session;
    });
    this.notify(session);
  }

  async clearSession(): Promise<void> {
    await this.withStorageLock(async () => {
      await this.storage.clear();
      this.cachedSession = null;
    });
    this.notify(null);
  }

  async withLock<T>(fn: () => Promise<T>): Promise<T> {
    return this.mutex.runExclusive(fn);
  }

  async refreshWith(
    refreshFn: (refreshToken: string) => Promise<AuthSession>
  ): Promise<AuthSession> {
    if (this.refreshInFlight) {
      return this.refreshInFlight;
    }

    const refreshPromise = (async () => {
      const baseline = await this.readSessionFromStorage();
      if (!baseline) {
        const error = new Error("No session to refresh") as Error & {
          code: string;
        };
        error.code = "no_session";
        throw error;
      }

      return this.withRefreshLock(async () => {
        const current = await this.readSessionFromStorage();
        if (!current) {
          const error = new Error("No session to refresh") as Error & {
            code: string;
          };
          error.code = "no_session";
          throw error;
        }

        if (!sessionsEqual(current, baseline) && !isAccessExpired(current)) {
          this.cachedSession = current;
          return current;
        }

        if (isRefreshExpired(current)) {
          const error = new Error("Refresh token expired") as Error & {
            code: string;
          };
          error.code = "refresh_expired";
          await this.clearSession();
          throw error;
        }

        try {
          const next = await refreshFn(current.refreshToken);
          await this.updateSession(next);
          return next;
        } catch (error) {
          if (shouldClearSessionOnRefreshFailure(error)) {
            await this.clearSession();
          }
          throw error;
        }
      });
    })();

    this.refreshInFlight = refreshPromise;

    try {
      return await refreshPromise;
    } finally {
      this.refreshInFlight = null;
    }
  }

  async exportSnapshot(): Promise<AuthSessionSnapshot | null> {
    const session = await this.getSession();
    return session ? toSnapshot(session) : null;
  }

  async importSnapshot(snapshot: AuthSessionSnapshot): Promise<void> {
    await this.updateSession(fromSnapshot(snapshot));
  }

  onChange(listener: (session: AuthSession | null) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  protected notify(session: AuthSession | null): void {
    for (const listener of this.listeners) {
      try {
        listener(session);
      } catch {
        // Ignore listener errors so auth flows stay resilient.
      }
    }
  }
}

const isAccessExpired = (session: AuthSession): boolean =>
  session.expiresAt <= Date.now();

const sessionsEqual = (
  left: AuthSession | null | undefined,
  right: AuthSession | null | undefined
): boolean => {
  if ((left ?? null) === (right ?? null)) return true;
  if (!left || !right) return false;
  if (left.sessionId !== right.sessionId) return false;
  if (left.accessToken !== right.accessToken) return false;
  if (left.refreshToken !== right.refreshToken) return false;
  if (left.accessJti !== right.accessJti) return false;
  if (left.expiresAt !== right.expiresAt) return false;
  if (left.refreshExpiresAt !== right.refreshExpiresAt) return false;
  if (left.counter !== right.counter) return false;
  if (left.popKeyBytes.length !== right.popKeyBytes.length) return false;
  for (let i = 0; i < left.popKeyBytes.length; i += 1) {
    if (left.popKeyBytes[i] !== right.popKeyBytes[i]) return false;
  }
  return true;
};
