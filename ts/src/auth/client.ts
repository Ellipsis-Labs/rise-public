import { getPhoenixClientHeader } from "@/clientIdentity";
import { PhoenixAuthError } from "@/errors";
import type { AuthSessionManager } from "./manager";
import { resetFromAuthResponse, type AuthSession } from "./session";
import { resolvePhoenixApiUrl, type PhoenixApiUrlConfig } from "@/apiUrl";
import {
  AuthChallengeResponseSchema,
  AuthResponseSchema,
  JsonWebKeySetSchema,
  PrivyWalletChallengeResponseSchema,
  WalletNonceResponseSchema,
  WalletTransactionChallengeResponseSchema,
  type RiseSessionControl,
  type AuthChallengeResponse,
  type JsonWebKeySet,
  type PrivyLoginRequest,
  type PrivyWalletChallengeResponse,
  type PrivyWalletProof,
  type ServiceChallengeRequest,
  type ServiceLoginRequest,
  type WalletNonceResponse,
  type WalletTransactionChallengeResponse,
  type WalletTransactionLoginRequest,
} from "./types";

export interface PhoenixAuthClientConfig extends PhoenixApiUrlConfig {
  timeout?: number;
  sessionManager?: AuthSessionManager;
  sessionControl?: RiseSessionControl;
  clientHeaderProvider?: () => Record<string, string>;
}

const DEFAULT_TIMEOUT = 30_000;
const DEFAULT_RATE_LIMIT_RETRY_SECONDS = 1;
const MAX_RATE_LIMIT_RETRY_SECONDS = 30;

const parseRetryAfterSeconds = (value: string | null): number | undefined => {
  if (!value) return undefined;
  const trimmed = value.trim();
  const parsedSeconds = Number(trimmed);
  if (Number.isFinite(parsedSeconds) && parsedSeconds >= 0) {
    return Math.ceil(parsedSeconds);
  }

  const retryDate = Date.parse(trimmed);
  if (Number.isNaN(retryDate)) return undefined;
  const deltaMs = retryDate - Date.now();
  if (deltaMs <= 0) return 0;
  return Math.ceil(deltaMs / 1000);
};

const effectiveRetryAfterSeconds = (retryAfterSeconds?: number): number => {
  const value = retryAfterSeconds ?? DEFAULT_RATE_LIMIT_RETRY_SECONDS;
  return Math.min(
    MAX_RATE_LIMIT_RETRY_SECONDS,
    Math.max(DEFAULT_RATE_LIMIT_RETRY_SECONDS, Math.ceil(value))
  );
};

const extractTimestampFromMessage = (message: string): string | undefined => {
  const match = message.match(/^timestamp:(.+)$/m);
  return match ? match[1].trim() : undefined;
};

export class PhoenixAuthClient {
  private readonly apiUrl: string;
  private readonly timeout: number;
  private readonly sessionManager?: AuthSessionManager;
  private readonly sessionControl?: RiseSessionControl;
  private readonly clientHeaderProvider?: () => Record<string, string>;

  constructor(config: PhoenixAuthClientConfig = {}) {
    this.apiUrl = resolvePhoenixApiUrl(config);
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;
    this.sessionManager = config.sessionManager;
    this.sessionControl = config.sessionControl;
    this.clientHeaderProvider = config.clientHeaderProvider;
  }

  private shouldPersistSessionState(): boolean {
    return this.sessionControl !== "external";
  }

  private async requestJson<T>(
    method: string,
    endpoint: string,
    schema: { parse: (data: unknown) => T },
    body?: unknown,
    params?: Record<string, string>,
    extraHeaders?: Record<string, string>
  ): Promise<T> {
    const url = new URL(`${this.apiUrl}${endpoint}`);
    if (params) {
      url.search = new URLSearchParams(params).toString();
    }

    const execute = async (): Promise<T> => {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), this.timeout);
      try {
        const response = await fetch(url.toString(), {
          method,
          headers: {
            ...(this.clientHeaderProvider?.() ?? getPhoenixClientHeader()),
            "Content-Type": "application/json",
            ...(extraHeaders ?? {}),
          },
          body: body === undefined ? undefined : JSON.stringify(body),
          signal: controller.signal,
        });

        if (!response.ok) {
          const errorText = await response.text();
          let message = errorText || response.statusText;
          let code = "auth_request_failed";
          const retryAfterSeconds = parseRetryAfterSeconds(
            response.headers.get("retry-after")
          );

          try {
            const parsed = JSON.parse(errorText) as Record<string, unknown>;
            const errorValue = parsed.error;
            const messageValue = parsed.message;
            if (typeof errorValue === "string") {
              code = errorValue;
              message = errorValue;
            }
            if (typeof messageValue === "string") {
              message = messageValue;
            }
          } catch {
            // Non-JSON error body.
          }

          throw new PhoenixAuthError(message, code, {
            status: response.status,
            retryAfterSeconds,
            retryAt:
              retryAfterSeconds !== undefined
                ? Date.now() + retryAfterSeconds * 1000
                : null,
          });
        }

        return schema.parse(await response.json());
      } catch (error) {
        if (error instanceof PhoenixAuthError) throw error;
        if (error instanceof Error && error.name === "AbortError") {
          throw new PhoenixAuthError(
            `Request timeout after ${this.timeout}ms`,
            "timeout",
            { cause: error }
          );
        }
        throw new PhoenixAuthError(
          error instanceof Error ? error.message : "Auth request failed",
          "network_error",
          { cause: error }
        );
      } finally {
        clearTimeout(timeoutId);
      }
    };

    try {
      return await execute();
    } catch (error) {
      if (error instanceof PhoenixAuthError && error.code === "rate_limited") {
        const retryAfterSeconds = effectiveRetryAfterSeconds(
          error.retryAfterMs !== undefined
            ? Math.ceil(error.retryAfterMs / 1000)
            : undefined
        );
        await new Promise((resolve) =>
          setTimeout(resolve, retryAfterSeconds * 1000)
        );
        return execute();
      }
      throw error;
    }
  }

  private async handleAuthResponse(response: {
    access_token: string;
    refresh_token: string;
    refresh_expires_in: number;
    pop_key: string;
    token_type: "Bearer";
    expires_in: number;
  }): Promise<AuthSession> {
    const session = resetFromAuthResponse(response);
    if (this.sessionManager && this.shouldPersistSessionState()) {
      await this.sessionManager.updateSession(session);
    }
    return session;
  }

  async loginWithPrivyToken(
    privyToken: string,
    walletPubkey?: string,
    walletProof?: PrivyWalletProof
  ): Promise<AuthSession> {
    const body: PrivyLoginRequest = {
      privy_token: privyToken,
      ...(walletPubkey ? { wallet_pubkey: walletPubkey } : {}),
      ...(walletProof ? { wallet_proof: walletProof } : {}),
    };
    const response = await this.requestJson(
      "POST",
      "/v1/auth/login/privy",
      AuthResponseSchema,
      body
    );
    return this.handleAuthResponse(response);
  }

  /**
   * Fetches a Solana memo transaction the wallet must sign to prove ownership
   * during a Privy-authenticated login. Designed for Ledger users (and any
   * other wallet that can sign transactions but not arbitrary off-chain
   * messages). Returns the base64-encoded unsigned transaction; sign it with
   * the wallet, then pass `{ nonce_id, signed_transaction }` as `walletProof`
   * to `loginWithPrivyToken`.
   */
  async getPrivyWalletChallenge(
    privyToken: string,
    walletPubkey: string
  ): Promise<PrivyWalletChallengeResponse> {
    return this.requestJson(
      "POST",
      "/v1/auth/privy/wallet-challenge",
      PrivyWalletChallengeResponseSchema,
      {
        privy_token: privyToken,
        wallet_pubkey: walletPubkey,
      }
    );
  }

  async getWalletNonce(walletPubkey: string): Promise<WalletNonceResponse> {
    return this.requestJson(
      "GET",
      "/v1/auth/nonce",
      WalletNonceResponseSchema,
      undefined,
      { wallet_pubkey: walletPubkey }
    );
  }

  async loginWithWalletSignature(
    walletPubkey: string,
    signature: string,
    nonceId: string
  ): Promise<AuthSession> {
    const response = await this.requestJson(
      "POST",
      "/v1/auth/login/wallet",
      AuthResponseSchema,
      {
        wallet_pubkey: walletPubkey,
        signature,
        nonce_id: nonceId,
      }
    );
    return this.handleAuthResponse(response);
  }

  /**
   * Fetches a Solana memo transaction the wallet must sign to authenticate
   * with Phoenix directly. Designed for wallets that refuse off-chain
   * `signMessage` payloads (canonically: Ledger's Solana app). The Phoenix
   * session minted from `loginWithWalletTransaction` is interchangeable with
   * one minted from `loginWithWalletSignature`; downstream
   * `linkWithCustomJwt` (Privy) continues to work the same way.
   */
  async getWalletTransactionChallenge(
    walletPubkey: string
  ): Promise<WalletTransactionChallengeResponse> {
    return this.requestJson(
      "POST",
      "/v1/auth/wallet/transaction-challenge",
      WalletTransactionChallengeResponseSchema,
      { wallet_pubkey: walletPubkey }
    );
  }

  async loginWithWalletTransaction(
    walletPubkey: string,
    signedTransaction: string,
    nonceId: string
  ): Promise<AuthSession> {
    const body: WalletTransactionLoginRequest = {
      wallet_pubkey: walletPubkey,
      nonce_id: nonceId,
      signed_transaction: signedTransaction,
    };
    const response = await this.requestJson(
      "POST",
      "/v1/auth/login/wallet/transaction",
      AuthResponseSchema,
      body
    );
    return this.handleAuthResponse(response);
  }

  async serviceChallenge(
    request: ServiceChallengeRequest
  ): Promise<AuthChallengeResponse> {
    const response = await this.requestJson(
      "POST",
      "/v1/auth/login/service/challenge",
      AuthChallengeResponseSchema,
      request
    );
    if (!response.timestamp) {
      const parsed = extractTimestampFromMessage(response.message);
      if (parsed) return { ...response, timestamp: parsed };
    }
    return response;
  }

  async loginWithServiceSignature(
    request: ServiceLoginRequest
  ): Promise<AuthSession> {
    const response = await this.requestJson(
      "POST",
      "/v1/auth/login/service",
      AuthResponseSchema,
      request
    );
    return this.handleAuthResponse(response);
  }

  async refresh(refreshToken: string): Promise<AuthSession> {
    try {
      const session = await this.sessionManager?.getSession();
      const extraHeaders =
        session?.accessToken !== undefined
          ? { Authorization: `Bearer ${session.accessToken}` }
          : undefined;

      const response = await this.requestJson(
        "POST",
        "/v1/auth/refresh",
        AuthResponseSchema,
        { refresh_token: refreshToken },
        undefined,
        extraHeaders
      );
      return await this.handleAuthResponse(response);
    } catch (error) {
      if (
        error instanceof PhoenixAuthError &&
        (error.code === "invalid_refresh_token" ||
          error.code === "refresh_expired" ||
          error.code === "session_missing")
      ) {
        if (this.shouldPersistSessionState()) {
          await this.sessionManager?.clearSession();
        }
      }
      throw error;
    }
  }

  async logout(): Promise<void> {
    const session = await this.sessionManager?.getSession();
    const extraHeaders =
      session?.accessToken !== undefined
        ? { Authorization: `Bearer ${session.accessToken}` }
        : undefined;
    await this.requestJson(
      "POST",
      "/v1/auth/logout",
      {
        parse: () => undefined,
      },
      undefined,
      undefined,
      extraHeaders
    );
    if (this.shouldPersistSessionState()) {
      await this.sessionManager?.clearSession();
    }
  }

  async getJwks(): Promise<JsonWebKeySet> {
    return this.requestJson("GET", "/v1/auth/jwks", JsonWebKeySetSchema);
  }
}
