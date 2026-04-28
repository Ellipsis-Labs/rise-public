import { PhoenixAuthError } from "@/errors";
import { AuthBackoffManager } from "./backoff";
import { AuthSessionManager } from "./manager";
import {
  fromSnapshot,
  isAccessExpired,
  isAccessExpiringWithin,
  isRefreshExpired,
  type AuthSession,
  type AuthSessionSnapshot,
} from "./session";
import {
  LocalStorageAuthSessionStorage,
  MemoryAuthSessionStorage,
  type AuthSessionStorage,
} from "./storage";
import { PhoenixAuthClient } from "./client";
import { isExternalSessionControl, type RiseAuthConfig } from "./types";

const ACCESS_REFRESH_WINDOW_MS = 60_000;

const isSnapshot = (
  session: AuthSession | AuthSessionSnapshot
): session is AuthSessionSnapshot => "popKey" in session;

const isTerminalRefreshError = (error: unknown): boolean => {
  if (!error || typeof error !== "object") return false;
  const code = (error as { code?: unknown }).code;
  return (
    code === "invalid_refresh_token" ||
    code === "refresh_expired" ||
    code === "session_missing"
  );
};

export class RiseAuthRuntime {
  private readonly backoff = new AuthBackoffManager();
  private readonly sessionIsExternallyManaged: boolean;
  private readonly sessionManager?: AuthSessionManager;
  private readonly authClient?: PhoenixAuthClient;

  constructor(apiUrl: string, timeout: number, config?: RiseAuthConfig) {
    this.sessionIsExternallyManaged = isExternalSessionControl(
      config?.sessionControl
    );
    if (!config) return;

    const normalizedInitialSession = config.initialSession
      ? isSnapshot(config.initialSession)
        ? fromSnapshot(config.initialSession)
        : config.initialSession
      : undefined;

    const sessionManager =
      config.sessionManager ??
      new AuthSessionManager(resolveStorage(config), normalizedInitialSession);
    this.sessionManager = sessionManager;
    this.authClient = new PhoenixAuthClient({
      apiUrl,
      timeout,
      sessionManager,
      sessionControl: config.sessionControl,
    });

    if (config.sessionManager && normalizedInitialSession) {
      void sessionManager.updateSession(normalizedInitialSession);
    }
  }

  isEnabled(): boolean {
    return this.authClient !== undefined;
  }

  getSessionManager(): AuthSessionManager | undefined {
    return this.sessionManager;
  }

  getAuthClient(): PhoenixAuthClient | undefined {
    return this.authClient;
  }

  async exportSnapshot(): Promise<AuthSessionSnapshot | null> {
    return this.sessionManager?.exportSnapshot() ?? null;
  }

  async maybeGetSession(): Promise<AuthSession | null> {
    if (!this.sessionManager) {
      return null;
    }

    let session = await this.sessionManager.getSession();

    if (this.sessionIsExternallyManaged) {
      if (session) {
        return session;
      }
      return null;
    }

    if (session && isRefreshExpired(session)) {
      await this.sessionManager.clearSession();
      session = null;
    }

    if (session && isAccessExpiringWithin(session, ACCESS_REFRESH_WINDOW_MS)) {
      const refreshed = await this.tryRefresh();
      if (refreshed) session = refreshed;
    }

    if (session && !isAccessExpired(session)) {
      return session;
    }

    return null;
  }

  async recoverAfterUnauthorized(): Promise<AuthSession | null> {
    if (this.sessionIsExternallyManaged) {
      return null;
    }

    const refreshed = await this.tryRefresh();
    if (refreshed) return refreshed;

    return null;
  }

  private async tryRefresh(): Promise<AuthSession | null> {
    if (!this.sessionManager || !this.authClient) return null;
    if (this.backoff.isBlocked("refresh")) return null;

    const session = await this.sessionManager.getSession();
    if (!session || isRefreshExpired(session)) return null;

    try {
      const next = await this.sessionManager.refreshWith((refreshToken) =>
        this.authClient!.refresh(refreshToken)
      );
      this.backoff.reset("refresh");
      return next;
    } catch (error) {
      const retryAfterSeconds =
        error instanceof PhoenixAuthError && error.retryAfterMs !== undefined
          ? Math.ceil(error.retryAfterMs / 1000)
          : undefined;
      const code =
        error instanceof PhoenixAuthError ? error.code : "refresh_failed";
      this.backoff.markFailure("refresh", {
        retryAfterSeconds,
        errorCode: code,
      });
      if (isTerminalRefreshError(error)) {
        await this.sessionManager.clearSession();
      }
      return null;
    }
  }
}

const isBrowserLocalStorageAvailable = (): boolean => {
  try {
    if (typeof window === "undefined" || !window.localStorage) {
      return false;
    }
    const probeKey = "__phoenix_rise_auth_probe__";
    window.localStorage.setItem(probeKey, "1");
    window.localStorage.removeItem(probeKey);
    return true;
  } catch {
    return false;
  }
};

const resolveStorage = (config: RiseAuthConfig): AuthSessionStorage => {
  if (config.storage) {
    return config.storage;
  }
  if (isExternalSessionControl(config.sessionControl)) {
    throw new Error(
      'authConfig.sessionControl="external" requires authConfig.sessionManager or authConfig.storage'
    );
  }
  if (isBrowserLocalStorageAvailable()) {
    return new LocalStorageAuthSessionStorage();
  }
  return new MemoryAuthSessionStorage();
};
