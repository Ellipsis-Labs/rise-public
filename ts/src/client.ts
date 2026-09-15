import type { HttpTransport, RequestOptions } from "./http/transport";
import { appendQueryParams } from "./http/transport";
import {
  isRateLimitRetryableMethod,
  rateLimitCooldownDelayMs,
  rateLimitDelayWithPositiveJitterMs,
  rateLimitRetryDecision,
  resolveRateLimitCooldownConfig,
  resolveRateLimitRetryConfig,
  setRateLimitRetryAttempts,
  type RateLimitCooldownConfig,
  type RateLimitRetryConfig,
  type ResolvedRateLimitCooldownConfig,
  type ResolvedRateLimitRetryConfig,
} from "./http/rateLimitRetry";
import { V1CandlesClient } from "./api/candles/client";
import { V1CollateralClient } from "./api/collateral/client";
import { V1ExchangeClient } from "./api/exchange/client";
import { V1FundingClient } from "./api/funding/client";
import { V1InviteClient } from "./api/invite/client";
import { V1MarketsClient } from "./api/markets/client";
import { V1NotificationsClient } from "./api/notifications/client";
import { V1OrderbookClient } from "./api/orderbook/client";
import { V1OrdersClient } from "./api/orders/client";
import { V1TradersClient } from "./api/traders/client";
import { V1TradesClient } from "./api/trades/client";
import { RiseAuthRuntime } from "./auth/runtime";
import type { AuthSessionManager } from "./auth/manager";
import type { PhoenixAuthClient } from "./auth/client";
import type { AuthSessionSnapshot } from "./auth/session";
import type { RiseAuthConfig } from "./auth/types";
import { getPhoenixClientHeader } from "./clientIdentity";
import { PhoenixAuthError } from "./errors";
import { debugRise, summarizeDebugError } from "./internal/_debug";
import {
  createPhoenixWsClient,
  createPhoenixWsFacade,
} from "./ws/PhoenixWsClient";
import { toWebSocketUrl } from "./ws/url";
import type {
  PhoenixWsClient,
  PhoenixWsClientConfig,
  PhoenixWsTransportClient,
} from "./ws/PhoenixWsClient";
import {
  createPhoenixPdaClient,
  type PhoenixPdaCacheConfig,
  type PhoenixPdaClient,
  type PhoenixPdaAddressOverrides,
  type PhoenixPdaClientConfig,
} from "./pdaClient";
import {
  createPhoenixExchangeMetadata,
  type PhoenixExchangeMetadata,
  type PhoenixExchangeMetadataApiConfig,
  type PhoenixExchangeMetadataConfig,
} from "./exchange-cache";
import { createPhoenixMarketData, type PhoenixMarketData } from "./market-data";
import {
  createPhoenixOrderbookManager,
  type PhoenixOrderbookManager,
} from "./orderbooks";
import {
  buildLimitOrderPacketFromMarketParams,
  buildMarketOrderPacketFromMarketParams,
  type PhoenixOrderPacketBuilders,
} from "./orderPackets";
import { resolvePhoenixApiUrl, type PhoenixApiUrlConfig } from "./apiUrl";
import { createPhoenixIxClient, type PhoenixIxClient } from "./ixs";
import { createPhoenixRpcClient, type PhoenixRpcClient } from "./rpc";
import { resolvePhoenixRpcUrl } from "./rpcUrl";
import type { PhoenixProgramAddress } from "./primitives";
import type { PhoenixFlightClientConfig } from "./flight/client";

const DEFAULT_TIMEOUT = 30_000;
const DEFAULT_CLIENT_WS_IDLE_CLOSE_MS = 30_000;
const AUTH_RETRYABLE_CODES = new Set([
  "missing_access_token",
  "invalid_access_token",
  "access_token_expired",
  "access_jti_mismatch",
  "session_missing",
]);

const summarizeRequestBody = (
  body: unknown
): Record<string, unknown> | undefined => {
  if (body === undefined) {
    return undefined;
  }

  if (body === null) {
    return { bodyType: "null" };
  }

  if (Array.isArray(body)) {
    return { bodyType: "array", bodyLength: body.length };
  }

  if (typeof body === "object") {
    return {
      bodyType: "object",
      bodyKeys: Object.keys(body as Record<string, unknown>).sort(),
    };
  }

  return {
    bodyType: typeof body,
  };
};

const parseErrorPayload = async (
  response: Response
): Promise<{
  code?: string;
  message?: string;
  body?: Record<string, unknown>;
}> => {
  const text = await response.text().catch(() => "");
  if (!text.trim()) return {};
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    return {
      code: typeof parsed.error === "string" ? parsed.error : undefined,
      message: typeof parsed.message === "string" ? parsed.message : undefined,
      body: parsed,
    };
  } catch {
    return {};
  }
};

export interface PhoenixHttpClientConfig extends PhoenixApiUrlConfig {
  /** Optional Phoenix env/address config used by the PDA helper surface. */
  phoenixEnv?: string | null;
  /** Optional Phoenix address overrides used by the PDA helper surface. */
  addresses?: PhoenixPdaAddressOverrides;
  /** Optional explicit Phoenix program override used by the PDA helper surface. */
  programAddress?: PhoenixProgramAddress;
  /**
   * Memoize PDA derivations on `client.pda`.
   *
   * Pass `false` to disable caching or `{ maxEntries }` to bound the shared
   * PDA cache with LRU eviction.
   */
  pdaCache?: boolean | PhoenixPdaCacheConfig;
  /** Per-attempt network timeout in ms (default: 30000). */
  timeout?: number;
  /** Optional additional headers resolved per request. */
  extraHeaders?:
    | (() =>
        | Promise<Record<string, string> | undefined>
        | Record<string, string>
        | undefined)
    | undefined;
  /** Enables shared auth/session handling for HTTP and unified websocket clients. */
  auth?: boolean;
  /** Optional auth/session overrides used when `auth: true`. */
  authConfig?: RiseAuthConfig;
  /** Automatic rate-limit retry behavior for idempotent HTTP requests. */
  rateLimitRetry?: RateLimitRetryConfig | false;
  /**
   * Shared client-local cooldown applied to later idempotent HTTP requests
   * after a 429 Retry-After response.
   */
  rateLimitCooldown?: RateLimitCooldownConfig | false;
  /** Optional flight routing defaults applied to supported order-building paths. */
  flight?: PhoenixFlightClientConfig;
}

/**
 * HTTP client for the public Phoenix API surface exposed by `rise`.
 *
 * @example
 * ```ts
 * const client = new PhoenixHttpClient();
 *
 * const exchange = await client.exchange().getExchange();
 * const trader = await client.traders().getTrader(traderPubkey);
 * ```
 */
export class PhoenixHttpClient implements HttpTransport {
  private readonly apiUrl: string;
  private readonly timeout: number;
  private readonly extraHeaders?: PhoenixHttpClientConfig["extraHeaders"];
  private readonly rateLimitRetry: ResolvedRateLimitRetryConfig | null;
  private readonly rateLimitCooldown: ResolvedRateLimitCooldownConfig | null;
  private rateLimitCooldownUntilMs = 0;
  private readonly authRuntime?: RiseAuthRuntime;
  readonly pda: PhoenixPdaClient;
  private readonly candlesClient: V1CandlesClient;
  private readonly collateralClient: V1CollateralClient;
  private readonly exchangeClient: V1ExchangeClient;
  private readonly fundingClient: V1FundingClient;
  private readonly inviteClient: V1InviteClient;
  private readonly marketsClient: V1MarketsClient;
  private readonly notificationsClient: V1NotificationsClient;
  private readonly orderbookClient: V1OrderbookClient;
  private readonly ordersClient: V1OrdersClient;
  private readonly tradersClient: V1TradersClient;
  private readonly tradesClient: V1TradesClient;

  constructor(config: PhoenixHttpClientConfig = {}) {
    this.apiUrl = resolvePhoenixApiUrl(config);
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;
    this.extraHeaders = config.extraHeaders;
    this.rateLimitRetry = resolveRateLimitRetryConfig(config.rateLimitRetry);
    this.rateLimitCooldown = resolveRateLimitCooldownConfig(
      config.rateLimitCooldown,
      this.rateLimitRetry
    );
    const pdaConfig: PhoenixPdaClientConfig = {
      phoenixEnv: config.phoenixEnv,
      addresses: config.addresses,
      programAddress: config.programAddress,
      cache: config.pdaCache,
    };
    this.pda = createPhoenixPdaClient(pdaConfig);
    if (config.authConfig && !config.auth) {
      throw new Error("authConfig requires auth: true");
    }
    this.authRuntime = config.auth
      ? new RiseAuthRuntime(this.apiUrl, this.timeout, config.authConfig ?? {})
      : undefined;
    this.candlesClient = new V1CandlesClient(this);
    this.collateralClient = new V1CollateralClient(this);
    this.exchangeClient = new V1ExchangeClient(this);
    this.fundingClient = new V1FundingClient(this);
    this.inviteClient = new V1InviteClient(this);
    this.marketsClient = new V1MarketsClient(this);
    this.notificationsClient = new V1NotificationsClient(this);
    this.orderbookClient = new V1OrderbookClient(this);
    const flightConfig = config.flight;
    this.ordersClient = new V1OrdersClient(
      this,
      flightConfig
        ? {
            defaultFlight: {
              config: flightConfig,
              resolveFeeCollectorTraderAddress: (builderAuthority) =>
                this.pda.getTraderAddress({
                  authority: builderAuthority ?? flightConfig.builderAuthority,
                  traderPdaIndex: flightConfig.builderPdaIndex ?? 0,
                  subaccountIndex: flightConfig.builderSubaccountIndex ?? 0,
                  phoenixProgramAddress: this.pda.getProgramAddress(),
                }),
            },
          }
        : undefined
    );
    this.tradersClient = new V1TradersClient(this);
    this.tradesClient = new V1TradesClient(this);
  }

  private async performFetch(
    url: URL,
    method: string,
    headers: Record<string, string>,
    body: string | undefined
  ): Promise<Response> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    try {
      return await globalThis.fetch(url.toString(), {
        method,
        headers,
        body,
        signal: controller.signal,
      });
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "unknown network error";
      throw new Error(
        `Failed to connect to the Phoenix HTTP API (${method} ${url.toString()}): ${message}`,
        { cause: error }
      );
    } finally {
      clearTimeout(timer);
    }
  }

  private async resolveExtraHeaders(): Promise<Record<string, string>> {
    if (!this.extraHeaders) return {};
    const headers = await this.extraHeaders();
    return headers ?? {};
  }

  private async waitForRateLimitCooldown(
    method: string,
    url: URL
  ): Promise<void> {
    if (
      this.rateLimitCooldown === null ||
      !isRateLimitRetryableMethod(method)
    ) {
      return;
    }

    while (true) {
      const waitMs = Math.max(0, this.rateLimitCooldownUntilMs - Date.now());
      if (waitMs === 0) {
        return;
      }

      const jitteredWaitMs = rateLimitDelayWithPositiveJitterMs(waitMs);

      debugRise("http", "request:rate-limit-cooldown-wait", {
        method: method.toUpperCase(),
        url: url.toString(),
        waitMs: jitteredWaitMs,
        releaseJitterMs: jitteredWaitMs - waitMs,
      });
      await new Promise((resolve) => setTimeout(resolve, jitteredWaitMs));
    }
  }

  private recordRateLimitCooldown(
    response: Response,
    method: string,
    url: URL
  ): void {
    if (
      this.rateLimitCooldown === null ||
      response.status !== 429 ||
      !isRateLimitRetryableMethod(method)
    ) {
      return;
    }

    const nowMs = Date.now();
    const waitMs = rateLimitCooldownDelayMs(
      response,
      this.rateLimitCooldown,
      nowMs
    );
    const previousUntilMs = this.rateLimitCooldownUntilMs;
    this.rateLimitCooldownUntilMs = Math.max(
      this.rateLimitCooldownUntilMs,
      nowMs + waitMs
    );

    debugRise("http", "request:rate-limit-cooldown-update", {
      method: method.toUpperCase(),
      url: url.toString(),
      waitMs,
      extended: this.rateLimitCooldownUntilMs > previousUntilMs,
    });
  }

  async fetch(
    method: string,
    endpoint: string,
    options?: RequestOptions,
    body?: unknown
  ): Promise<Response> {
    const url = new URL(`${this.apiUrl}${endpoint}`);
    if (options?.params) appendQueryParams(url, options.params);
    const startedAt = Date.now();

    const authMode = options?.auth ?? "optional";
    const authEnabled =
      authMode !== "disabled" && this.authRuntime !== undefined;
    const session = authEnabled
      ? await this.authRuntime?.maybeGetSession()
      : undefined;
    if (authEnabled && authMode === "required" && !session) {
      throw new PhoenixAuthError(
        "Authenticated request requires a valid auth session",
        "no_auth_session"
      );
    }
    const requestBody = body !== undefined ? JSON.stringify(body) : undefined;

    try {
      debugRise("http", "request:start", {
        method: method.toUpperCase(),
        url: url.toString(),
        auth: authMode,
        hasSession: Boolean(session),
        routeId: options?.routeId,
        params:
          options?.params instanceof URLSearchParams
            ? options.params.toString()
            : options?.params,
        ...summarizeRequestBody(body),
      });

      const baseHeaders: Record<string, string> = {
        ...getPhoenixClientHeader(),
        ...(await this.resolveExtraHeaders()),
        ...options?.headers,
      };
      if (requestBody !== undefined) {
        baseHeaders["content-type"] = "application/json";
      }

      const execute = async (authorization?: string): Promise<Response> => {
        const headers: Record<string, string> = {
          ...baseHeaders,
        };
        if (authorization) headers.Authorization = authorization;
        await this.waitForRateLimitCooldown(method, url);
        const response = await this.performFetch(
          url,
          method,
          headers,
          requestBody
        );
        this.recordRateLimitCooldown(response, method, url);
        return response;
      };

      let authorizedResponse = await execute(
        session ? `Bearer ${session.accessToken}` : undefined
      );

      debugRise("http", "request:response", {
        method: method.toUpperCase(),
        url: url.toString(),
        status: authorizedResponse.status,
        durationMs: Date.now() - startedAt,
      });

      let rateLimitRetries = 0;
      let totalRateLimitWaitMs = 0;
      while (
        authorizedResponse.status === 429 &&
        this.rateLimitRetry !== null &&
        isRateLimitRetryableMethod(method) &&
        rateLimitRetries < this.rateLimitRetry.maxRetries
      ) {
        const retry = rateLimitRetryDecision(
          authorizedResponse,
          this.rateLimitRetry,
          totalRateLimitWaitMs
        );
        if (retry.kind === "exhausted") {
          debugRise("http", "request:retry-after-exhausted", {
            method: method.toUpperCase(),
            url: url.toString(),
            status: authorizedResponse.status,
            waitMs: retry.waitMs,
            nextTotalWaitMs: retry.nextTotalWaitMs,
            totalWaitMs: totalRateLimitWaitMs,
            maxTotalWaitMs: this.rateLimitRetry.maxTotalWaitMs,
            retryAfterSeconds: retry.retryAfterSeconds,
          });
          break;
        }

        debugRise("http", "request:retry-after", {
          method: method.toUpperCase(),
          url: url.toString(),
          status: authorizedResponse.status,
          waitMs: retry.waitMs,
          attempt: rateLimitRetries + 2,
          retryAfterSeconds: retry.retryAfterSeconds,
        });
        await new Promise((resolve) => setTimeout(resolve, retry.waitMs));
        totalRateLimitWaitMs = retry.nextTotalWaitMs;
        rateLimitRetries += 1;
        authorizedResponse = await execute(
          session ? `Bearer ${session.accessToken}` : undefined
        );
        debugRise("http", "request:retry-response", {
          method: method.toUpperCase(),
          url: url.toString(),
          status: authorizedResponse.status,
          durationMs: Date.now() - startedAt,
          attempt: rateLimitRetries + 1,
        });
      }

      if (authorizedResponse.status === 429) {
        setRateLimitRetryAttempts(authorizedResponse, rateLimitRetries + 1);
      }

      if (
        authEnabled &&
        session &&
        (authorizedResponse.status === 401 ||
          authorizedResponse.status === 403 ||
          authorizedResponse.status === 4401 ||
          authorizedResponse.status === 4403)
      ) {
        const { code } = await parseErrorPayload(authorizedResponse.clone());
        if (code && AUTH_RETRYABLE_CODES.has(code)) {
          const recovered = await this.authRuntime?.recoverAfterUnauthorized();
          if (recovered) {
            debugRise("http", "request:auth-retry", {
              method: method.toUpperCase(),
              url: url.toString(),
              status: authorizedResponse.status,
              code,
            });
            return execute(`Bearer ${recovered.accessToken}`);
          }
        }
      }

      return authorizedResponse;
    } catch (error) {
      debugRise("http", "request:error", {
        method: method.toUpperCase(),
        url: url.toString(),
        durationMs: Date.now() - startedAt,
        ...summarizeDebugError(error),
      });
      throw error;
    }
  }

  candles(): V1CandlesClient {
    return this.candlesClient;
  }

  collateral(): V1CollateralClient {
    return this.collateralClient;
  }

  exchange(): V1ExchangeClient {
    return this.exchangeClient;
  }

  funding(): V1FundingClient {
    return this.fundingClient;
  }

  invite(): V1InviteClient {
    return this.inviteClient;
  }

  markets(): V1MarketsClient {
    return this.marketsClient;
  }

  notifications(): V1NotificationsClient {
    return this.notificationsClient;
  }

  orderbook(): V1OrderbookClient {
    return this.orderbookClient;
  }

  orders(): V1OrdersClient {
    return this.ordersClient;
  }

  traders(): V1TradersClient {
    return this.tradersClient;
  }

  trades(): V1TradesClient {
    return this.tradesClient;
  }

  auth(): PhoenixAuthClient | undefined {
    return this.authRuntime?.getAuthClient();
  }

  sessionManager(): AuthSessionManager | undefined {
    return this.authRuntime?.getSessionManager();
  }

  async exportAuthSnapshot(): Promise<AuthSessionSnapshot | null> {
    return this.authRuntime?.exportSnapshot() ?? null;
  }

  dispose(): void {
    this.authRuntime?.getSessionManager()?.dispose();
  }
}

export interface PhoenixClientConfig extends PhoenixHttpClientConfig {
  /**
   * Solana RPC URL used by `client.rpc` and exchange metadata fallback.
   *
   * When omitted, `NEXT_PUBLIC_SOLANA_RPC_URL` or `SOLANA_RPC_URL` will be
   * used if present.
   */
  rpcUrl?: string;
  ws?:
    | false
    | (Omit<
        PhoenixWsClientConfig,
        "authMode" | "refreshFn" | "sessionControl" | "sessionManager" | "url"
      > & {
        url?: string;
        wsClient?: PhoenixWsTransportClient;
      });
  exchangeMetadata?: PhoenixClientExchangeMetadataConfig;
}

export type PhoenixClientExchangeMetadataApiConfig = Omit<
  PhoenixExchangeMetadataApiConfig,
  "api" | "exchange"
>;

export interface PhoenixClientExchangeMetadataConfig extends Omit<
  PhoenixExchangeMetadataConfig,
  "api"
> {
  api?: PhoenixClientExchangeMetadataApiConfig;
  stream?: boolean;
}

export interface PhoenixClient {
  api: PhoenixHttpClient;
  pda: PhoenixPdaClient;
  auth?: PhoenixAuthClient;
  sessionManager?: AuthSessionManager;
  streams?: PhoenixWsClient;
  exchange: PhoenixExchangeMetadata;
  orderPackets: PhoenixOrderPacketBuilders;
  ixs: PhoenixIxClient;
  rpc: PhoenixRpcClient;
  marketData(): PhoenixMarketData;
  orderbooks(): PhoenixOrderbookManager;
  dispose(): void;
}

export const createPhoenixClient = (
  config: PhoenixClientConfig = {}
): PhoenixClient => {
  const apiUrl = resolvePhoenixApiUrl(config);
  const resolvedRpcUrl = resolvePhoenixRpcUrl({
    rpcUrl: config.rpcUrl,
  });
  const exchangeRpcConfig = config.exchangeMetadata?.rpc;
  const exchangeRpcUrl = resolvePhoenixRpcUrl({
    rpcUrl: exchangeRpcConfig?.url ?? resolvedRpcUrl,
  });
  const exchangeStreamSetting = config.exchangeMetadata?.stream;
  const exchangeLiveUpdatesSetting = config.exchangeMetadata?.api?.liveUpdates;
  if (
    exchangeStreamSetting !== undefined &&
    exchangeLiveUpdatesSetting !== undefined &&
    exchangeStreamSetting !== (exchangeLiveUpdatesSetting === "ws")
  ) {
    throw new Error(
      "exchangeMetadata.stream conflicts with exchangeMetadata.api.liveUpdates"
    );
  }
  const exchangeLiveUpdates =
    exchangeStreamSetting !== undefined
      ? exchangeStreamSetting
        ? "ws"
        : "none"
      : (exchangeLiveUpdatesSetting ?? "none");
  const api = new PhoenixHttpClient(config);
  const auth = api.auth();
  const sessionManager = api.sessionManager();
  const sessionControl = config.authConfig?.sessionControl;
  const wsSetting = config.ws === false ? false : (config.ws ?? {});
  const externalWsClient = wsSetting === false ? undefined : wsSetting.wsClient;
  const wsConfig =
    wsSetting === false
      ? undefined
      : (({ wsClient: _wsClient, ...rest }) => rest)(wsSetting);
  const streams =
    wsSetting !== false
      ? externalWsClient
        ? createPhoenixWsFacade({
            wsClient: externalWsClient,
            adapterOptions: wsSetting.adapterOptions,
            strictMode: wsSetting.strictMode,
          })
        : createPhoenixWsClient({
            ...wsConfig,
            connectMode: wsConfig?.connectMode ?? "lazy",
            idleCloseMs:
              wsConfig?.idleCloseMs ?? DEFAULT_CLIENT_WS_IDLE_CLOSE_MS,
            url: wsConfig?.url ?? toWebSocketUrl(apiUrl),
            sessionManager,
            sessionControl,
            refreshFn: auth
              ? (refreshToken: string) => auth.refresh(refreshToken)
              : undefined,
          })
      : undefined;
  const exchange = createPhoenixExchangeMetadata({
    ...config.exchangeMetadata,
    pda: api.pda,
    api: {
      api: api.exchange(),
      exchange: streams?.exchange,
      enabled: true,
      liveUpdates: streams && exchangeLiveUpdates === "ws" ? "ws" : "none",
      ...(config.exchangeMetadata?.api ?? {}),
    },
    rpc:
      exchangeRpcUrl !== undefined || exchangeRpcConfig !== undefined
        ? {
            ...exchangeRpcConfig,
            url: exchangeRpcUrl,
            enabled: exchangeRpcConfig?.enabled ?? exchangeRpcUrl !== undefined,
          }
        : undefined,
  });
  const orderPackets: PhoenixOrderPacketBuilders = {
    buildLimitOrderPacket: async ({ symbol, ...params }) => {
      await exchange.ready();
      const market = exchange.market(symbol);
      if (!market) {
        const availableSymbols = exchange
          .snapshot()
          .markets.map((innerMarket) => innerMarket.symbol)
          .join(", ");
        throw new Error(
          `Unknown market symbol '${symbol}'. Available symbols: ${availableSymbols}`
        );
      }
      return buildLimitOrderPacketFromMarketParams(params, {
        tickSize: market.tickSize,
        baseLotsDecimals: market.baseLotsDecimals,
      });
    },
    buildMarketOrderPacket: async ({ symbol, ...params }) => {
      await exchange.ready();
      const market = exchange.market(symbol);
      if (!market) {
        const availableSymbols = exchange
          .snapshot()
          .markets.map((innerMarket) => innerMarket.symbol)
          .join(", ");
        throw new Error(
          `Unknown market symbol '${symbol}'. Available symbols: ${availableSymbols}`
        );
      }
      return buildMarketOrderPacketFromMarketParams(params, {
        tickSize: market.tickSize,
        baseLotsDecimals: market.baseLotsDecimals,
      });
    },
  };
  const rpc = createPhoenixRpcClient({
    rpcUrl: resolvedRpcUrl,
    exchange,
    pda: api.pda,
  });
  const ixs = createPhoenixIxClient({
    exchange,
    pda: api.pda,
    orderPackets,
    rpc,
    flight: config.flight,
  });
  let marketDataSingleton: PhoenixMarketData | undefined;
  let orderbooksSingleton: PhoenixOrderbookManager | undefined;
  const marketData = (): PhoenixMarketData => {
    if (!marketDataSingleton) {
      marketDataSingleton = createPhoenixMarketData({
        exchange,
        marketStats: streams?.marketStats,
      });
    }
    return marketDataSingleton;
  };
  const orderbooks = (): PhoenixOrderbookManager => {
    if (!orderbooksSingleton) {
      orderbooksSingleton = createPhoenixOrderbookManager({
        api: {
          getOrderbook: (symbol, options) =>
            api.orderbook().getOrderbook(symbol, options),
        },
        l2Book: streams?.l2Book,
      });
    }
    return orderbooksSingleton;
  };

  return {
    api: api,
    pda: api.pda,
    auth: auth,
    sessionManager: sessionManager,
    streams: streams,
    exchange,
    marketData,
    orderbooks,
    orderPackets,
    ixs,
    rpc,
    dispose: () => {
      marketDataSingleton?.close();
      orderbooksSingleton?.close();
      exchange.close();
      streams?.close();
      api.dispose();
    },
  };
};
