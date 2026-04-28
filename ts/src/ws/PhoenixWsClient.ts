import {
  createAllMidsAdapter,
  createCandlesAdapter,
  createExchangeAdapter,
  createExchangeStatusAdapter,
  createFillsAdapter,
  createFundingRateAdapter,
  createL2BookAdapter,
  createMarkPriceAdapter,
  createMarketAdapter,
  createMarketStatsAdapter,
  createNotificationsAdapter,
  createOrderbookAdapter,
  createTraderStateAdapter,
} from "./adapters";
import type { PublicAdapterOptions } from "./adapters/options";
import type { RiseSessionControl, RiseWsAuthMode } from "@/auth/types";
import type { AuthSessionManager } from "@/auth/manager";
import type { AuthSession } from "@/auth/session";
import type {
  CustomSubscriptionDefinitions,
  CustomSubscriptionDefinition,
  RegisteredSubscriptionAdapter,
  RegisteredSubscriptionAdapters,
} from "./registerSubscriptions";
import type {
  SubscriptionMessage,
  WsClient,
  WsClientOpts,
  WsServerErrorListener,
} from "./types";
import {
  registerSubscription as registerCustomSubscription,
  registerSubscriptions as registerCustomSubscriptions,
} from "./registerSubscriptions";
import { createWsClient } from "./WsClient";

export interface PhoenixWsClientConfig extends WsClientOpts {
  adapterOptions?: PublicAdapterOptions;
  strictMode?: boolean;
  sessionManager?: AuthSessionManager;
  refreshFn?: (refreshToken: string) => Promise<AuthSession>;
  sessionControl?: RiseSessionControl;
  authMode?: RiseWsAuthMode;
  onServerError?: WsServerErrorListener;
}

export type PhoenixWsTransportClient = Pick<
  WsClient,
  "subscribe" | "unsubscribe" | "registerChannel" | "close" | "onServerError"
>;

export interface PhoenixWsFacadeConfig {
  wsClient: PhoenixWsTransportClient;
  adapterOptions?: PublicAdapterOptions;
  strictMode?: boolean;
  ownsWsClient?: boolean;
}

export interface PhoenixWsClient {
  wsClient: PhoenixWsTransportClient;
  allMids: ReturnType<typeof createAllMidsAdapter>;
  candles: ReturnType<typeof createCandlesAdapter>;
  exchange: ReturnType<typeof createExchangeAdapter>;
  exchangeStatus: ReturnType<typeof createExchangeStatusAdapter>;
  fills: ReturnType<typeof createFillsAdapter>;
  fundingRate: ReturnType<typeof createFundingRateAdapter>;
  l2Book: ReturnType<typeof createL2BookAdapter>;
  markPrice: ReturnType<typeof createMarkPriceAdapter>;
  market: ReturnType<typeof createMarketAdapter>;
  marketStats: ReturnType<typeof createMarketStatsAdapter>;
  notifications: ReturnType<typeof createNotificationsAdapter>;
  orderbook: ReturnType<typeof createL2BookAdapter>;
  orderbookSnapshot: ReturnType<typeof createOrderbookAdapter>;
  registerSubscription: <TRaw, TUpdate, TParams extends unknown[]>(
    definition: CustomSubscriptionDefinition<TRaw, TUpdate, TParams>
  ) => RegisteredSubscriptionAdapter<TUpdate, TParams>;
  registerSubscriptions: <T extends CustomSubscriptionDefinitions>(
    definitions: T
  ) => RegisteredSubscriptionAdapters<T>;
  traderState: ReturnType<typeof createTraderStateAdapter>;
  onServerError: (listener: WsServerErrorListener) => () => void;
  close: () => void;
}

let nextPhoenixWsFacadeId = 0;
const DEFAULT_LAZY_IDLE_CLOSE_MS = 30_000;

const createScopedWsClient = (
  wsClient: PhoenixWsTransportClient,
  ownsWsClient: boolean
): PhoenixWsTransportClient => {
  const scopePrefix = `rise-facade:${nextPhoenixWsFacadeId++}:`;
  const scopedSubscriptions = new Map<
    string,
    {
      scopedKey: string;
      unsubMsg: SubscriptionMessage;
    }
  >();

  return {
    subscribe(key, subMsg, onMessage, options) {
      const scopedKey =
        scopedSubscriptions.get(key)?.scopedKey ?? `${scopePrefix}${key}`;
      scopedSubscriptions.set(key, {
        scopedKey,
        unsubMsg: {
          type: "unsubscribe",
          subscription: subMsg.subscription,
        },
      });
      wsClient.subscribe(scopedKey, subMsg, onMessage, options);
    },
    unsubscribe(key, _unsubMsg) {
      const entry = scopedSubscriptions.get(key);
      if (!entry) {
        return;
      }
      scopedSubscriptions.delete(key);
      wsClient.unsubscribe(entry.scopedKey, entry.unsubMsg);
    },
    registerChannel(channel, registration) {
      return wsClient.registerChannel(channel, registration);
    },
    onServerError(listener) {
      return wsClient.onServerError(listener);
    },
    close() {
      for (const { scopedKey, unsubMsg } of scopedSubscriptions.values()) {
        wsClient.unsubscribe(scopedKey, unsubMsg);
      }
      scopedSubscriptions.clear();
      if (ownsWsClient) {
        wsClient.close();
      }
    },
  };
};

export const createPhoenixWsFacade = ({
  wsClient,
  adapterOptions,
  strictMode,
  ownsWsClient = false,
}: PhoenixWsFacadeConfig): PhoenixWsClient => {
  const scopedWsClient = createScopedWsClient(wsClient, ownsWsClient);
  const allMids = createAllMidsAdapter(
    scopedWsClient,
    adapterOptions?.allMids,
    strictMode
  );
  const candles = createCandlesAdapter(
    scopedWsClient,
    adapterOptions?.candles,
    strictMode
  );
  const exchange = createExchangeAdapter(
    scopedWsClient,
    adapterOptions?.exchange,
    strictMode
  );
  const exchangeStatus = createExchangeStatusAdapter(
    scopedWsClient,
    adapterOptions?.exchangeStatus,
    strictMode
  );
  const fills = createFillsAdapter(
    scopedWsClient,
    adapterOptions?.fills,
    strictMode
  );
  const fundingRate = createFundingRateAdapter(
    scopedWsClient,
    adapterOptions?.fundingRate,
    strictMode
  );
  const l2Book = createL2BookAdapter(
    scopedWsClient,
    adapterOptions?.l2Book,
    strictMode
  );
  const markPrice = createMarkPriceAdapter(
    scopedWsClient,
    adapterOptions?.markPrice,
    strictMode
  );
  const market = createMarketAdapter(
    scopedWsClient,
    adapterOptions?.market,
    strictMode
  );
  const marketStats = createMarketStatsAdapter(
    scopedWsClient,
    adapterOptions?.marketStats,
    strictMode
  );
  const notifications = createNotificationsAdapter(
    scopedWsClient,
    adapterOptions?.notifications,
    strictMode
  );
  const orderbook = createL2BookAdapter(
    scopedWsClient,
    adapterOptions?.orderbook,
    strictMode
  );
  const orderbookSnapshot = createOrderbookAdapter(
    scopedWsClient,
    adapterOptions?.orderbookSnapshot,
    strictMode
  );
  const traderState = createTraderStateAdapter(
    scopedWsClient,
    adapterOptions?.traderState,
    strictMode
  );

  return {
    wsClient,
    allMids,
    candles,
    exchange,
    exchangeStatus,
    fills,
    fundingRate,
    l2Book,
    markPrice,
    market,
    marketStats,
    notifications,
    orderbook,
    orderbookSnapshot,
    registerSubscription: <TRaw, TUpdate, TParams extends unknown[]>(
      definition: CustomSubscriptionDefinition<TRaw, TUpdate, TParams>
    ): RegisteredSubscriptionAdapter<TUpdate, TParams> =>
      registerCustomSubscription(scopedWsClient, definition),
    registerSubscriptions: <T extends CustomSubscriptionDefinitions>(
      definitions: T
    ): RegisteredSubscriptionAdapters<T> =>
      registerCustomSubscriptions(scopedWsClient, definitions),
    traderState,
    onServerError: (listener: WsServerErrorListener) =>
      scopedWsClient.onServerError(listener),
    close: () => scopedWsClient.close(),
  };
};

export const createPhoenixWsClient = (
  config: PhoenixWsClientConfig
): PhoenixWsClient => {
  const connectMode = config.connectMode ?? "eager";
  const wsClient = createWsClient({
    url: config.url,
    protocol: config.protocol,
    backoff: config.backoff,
    connectMode,
    idleCloseMs:
      config.idleCloseMs ??
      (connectMode === "lazy" ? DEFAULT_LAZY_IDLE_CLOSE_MS : undefined),
    sessionManager: config.sessionManager,
    refreshFn: config.refreshFn,
    sessionControl: config.sessionControl,
    authMode: config.authMode,
    onServerError: config.onServerError,
  });

  return createPhoenixWsFacade({
    wsClient,
    adapterOptions: config.adapterOptions,
    strictMode: config.strictMode,
    ownsWsClient: true,
  });
};
