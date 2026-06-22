import { handleError, initializeErrorSystem } from "./errorHandling";
import { createConnectionError } from "./errorHandling/errors";
import { createMessageHandler } from "./messageHandler";
import { createDefaultPlugins, createPluginRegistry } from "./plugins";
import { debugWs } from "./debug";
import { AuthWsLifecycleController } from "./authLifecycleMachine";
import {
  isAccessExpired,
  isAccessExpiringWithin,
  isRefreshExpired,
} from "@/auth/session";
import {
  getRefreshRetryAfterMs,
  isTerminalRefreshError,
} from "@/auth/refreshErrors";
import { isExternalSessionControl } from "@/auth/types";
import type {
  Subscription,
  SubscriptionOptions,
  SubscriptionMessage,
  WsChannelRegistration,
  WsClient,
  WsClientConfig,
  WsClientOpts,
  WsServerErrorListener,
} from "./types";

const ACCESS_REFRESH_WINDOW_MS = 60_000;
const WS_OPEN = 1;
const DEFER_CONNECT_FOR_SESSION_OWNER = Symbol(
  "defer-connect-for-session-owner"
);
const AUTH_REQUIRED_SUBSCRIPTION_CHANNELS: ReadonlySet<string> = new Set([
  "events",
  "notifications",
  "trader",
  "traderVolume",
  "transaction",
]);

export const createWsClient = (opts: WsClientOpts): WsClient => {
  type DeferredSessionOwnerConnect = typeof DEFER_CONNECT_FOR_SESSION_OWNER;
  type ResolvedProtocol = WsClientConfig["protocol"];
  let ws: WebSocket | undefined;
  let connectInFlight: Promise<void> | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let idleCloseTimer: ReturnType<typeof setTimeout> | null = null;
  let authRefreshRetryTimer: ReturnType<typeof setTimeout> | null = null;
  let deferredScheduleImmediate = false;
  let suppressAuthChangeDuringProtocolRefresh = false;

  const authMode =
    opts.authMode ?? (opts.sessionManager ? "auto" : "anonymous");
  const sessionIsExternallyManaged = isExternalSessionControl(
    opts.sessionControl
  );

  const config: WsClientConfig = {
    ...opts,
    backoff: opts.backoff ?? { baseMs: 500, maxMs: 10_000 },
    connectMode: opts.connectMode ?? "eager",
  };
  const lifecycle = new AuthWsLifecycleController(config.backoff, authMode);

  debugWs("ws client initialized", {
    url: config.url,
    authMode,
    hasSessionManager: Boolean(opts.sessionManager),
  });

  initializeErrorSystem();

  const subscriptionRegistry = new Map<string, Subscription>();
  const subscriptionListeners = new Map<
    string,
    {
      routingKey: string;
      subMsg: SubscriptionMessage;
      onMessage: (data: unknown) => void;
    }
  >();
  const listenerKeysByRoutingKey = new Map<string, Set<string>>();
  const serverErrorListeners = new Set<WsServerErrorListener>();
  if (opts.onServerError) {
    serverErrorListeners.add(opts.onServerError);
  }

  const emitServerError = (message: {
    channel: "error";
    error: string;
    code: number;
  }) => {
    for (const listener of serverErrorListeners) {
      try {
        listener(message);
      } catch {
        // Listener errors must not disrupt websocket processing.
      }
    }
  };

  const pluginRegistry = createPluginRegistry(
    createDefaultPlugins({
      onServerErrorFrame: emitServerError,
    })
  );
  const customChannelRegistry = new Map<string, WsChannelRegistration>();
  const handleMessage = createMessageHandler(pluginRegistry);

  const getListenersForRoutingKey = (routingKey: string) => {
    const listenerKeys = listenerKeysByRoutingKey.get(routingKey);
    if (!listenerKeys) {
      return [];
    }

    const listeners: Array<{
      routingKey: string;
      subMsg: SubscriptionMessage;
      onMessage: (data: unknown) => void;
    }> = [];
    for (const listenerKey of listenerKeys) {
      const listener = subscriptionListeners.get(listenerKey);
      if (listener) {
        listeners.push(listener);
      }
    }
    return listeners;
  };

  const syncRoutingSubscription = (routingKey: string) => {
    const listeners = getListenersForRoutingKey(routingKey);
    if (listeners.length === 0) {
      subscriptionRegistry.delete(routingKey);
      return;
    }

    subscriptionRegistry.set(routingKey, {
      onMsg: (data) => {
        for (const listener of getListenersForRoutingKey(routingKey)) {
          listener.onMessage(data);
        }
      },
      onResub: () => {
        const [listener] = getListenersForRoutingKey(routingKey);
        if (!listener) return;
        sendSubscriptionMessage(listener.subMsg);
      },
    });
  };

  const hasActiveSubscriptions = () => listenerKeysByRoutingKey.size > 0;

  const shouldMaintainConnection = () =>
    config.connectMode === "eager" || hasActiveSubscriptions();

  const wantsAuthenticatedConnection = () => {
    if (authMode === "authenticated") return true;
    if (authMode === "anonymous") return false;
    return lifecycle.knownAuthToken !== null;
  };

  const isOpenSocket = (socket: WebSocket | undefined): socket is WebSocket =>
    socket !== undefined &&
    (typeof socket.readyState !== "number" || socket.readyState === WS_OPEN);

  const canUseAnonymousSocketWhileAwaitingAuth = () =>
    authMode === "auto" &&
    lifecycle.isAwaitingAuth &&
    lifecycle.connectionAuth === "anonymous" &&
    !wantsAuthenticatedConnection();

  const clearReconnectTimer = () => {
    if (!reconnectTimer) return;
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  };

  const clearIdleCloseTimer = () => {
    if (!idleCloseTimer) return;
    clearTimeout(idleCloseTimer);
    idleCloseTimer = null;
  };

  const clearAuthRefreshRetryTimer = () => {
    if (!authRefreshRetryTimer) return;
    clearTimeout(authRefreshRetryTimer);
    authRefreshRetryTimer = null;
  };

  const scheduleIdleClose = () => {
    clearIdleCloseTimer();
    if (config.connectMode !== "lazy" || shouldMaintainConnection()) {
      return;
    }
    if (config.idleCloseMs === undefined || config.idleCloseMs < 0) {
      return;
    }
    idleCloseTimer = setTimeout(() => {
      idleCloseTimer = null;
      if (shouldMaintainConnection()) {
        return;
      }
      clearReconnectTimer();
      try {
        ws?.close();
      } catch {
        // Best-effort close.
      }
    }, config.idleCloseMs);
  };

  const ensureConnection = () => {
    clearIdleCloseTimer();
    if (!shouldMaintainConnection()) {
      return;
    }
    if (canUseAnonymousSocketWhileAwaitingAuth() && isOpenSocket(ws)) {
      return;
    }
    if (lifecycle.isConnected || connectInFlight || reconnectTimer) {
      return;
    }
    void connect();
  };

  const applyEffect = (
    effect:
      | { type: "close_socket" }
      | {
          type: "schedule_connect";
          waitMs: number;
          mode: "backoff" | "immediate";
        }
      | { type: "handle_auth_close_policy" }
  ) => {
    switch (effect.type) {
      case "close_socket": {
        ws?.close();
        return;
      }
      case "schedule_connect": {
        if (!shouldMaintainConnection()) {
          return;
        }
        if (lifecycle.isClosing) return;
        if (connectInFlight) {
          deferredScheduleImmediate ||= effect.mode === "immediate";
          return;
        }
        clearReconnectTimer();
        reconnectTimer = setTimeout(() => {
          reconnectTimer = null;
          void connect();
        }, effect.waitMs);
        return;
      }
      case "handle_auth_close_policy": {
        void handleAuthClosePolicy();
        return;
      }
    }
  };

  const flushEffects = () => {
    const effects = lifecycle.drainEffects();
    for (const effect of effects) {
      applyEffect(effect);
    }
  };

  const requestReconnect = (mode: "backoff" | "immediate" = "backoff") => {
    if (!shouldMaintainConnection()) return;
    if (lifecycle.isClosing) return;
    lifecycle.send({ type: "REQUEST_RECONNECT", mode, hasSocket: Boolean(ws) });
    flushEffects();
  };

  const scheduleAuthRefreshRetry = (
    error: unknown,
    retry: () => void
  ): boolean => {
    if (isTerminalRefreshError(error)) {
      return false;
    }
    clearReconnectTimer();
    clearAuthRefreshRetryTimer();
    const waitMs = getRefreshRetryAfterMs(error) ?? config.backoff.baseMs;
    authRefreshRetryTimer = setTimeout(() => {
      authRefreshRetryTimer = null;
      if (lifecycle.isClosing) return;
      retry();
    }, waitMs);
    return true;
  };

  const awaitExternalSessionChange = () => {
    clearReconnectTimer();
    lifecycle.send({ type: "AWAIT_AUTH" });
  };

  const subscriptionRequiresAuth = (message: SubscriptionMessage): boolean => {
    const channel = message.subscription.channel;
    return (
      typeof channel === "string" &&
      AUTH_REQUIRED_SUBSCRIPTION_CHANNELS.has(channel)
    );
  };

  const isCurrentSocket = (socket: WebSocket): boolean => ws === socket;

  const canUseCurrentSocketForMessage = (
    message: SubscriptionMessage
  ): boolean => {
    if (lifecycle.isConnected) return true;
    return (
      canUseAnonymousSocketWhileAwaitingAuth() &&
      !subscriptionRequiresAuth(message)
    );
  };

  const requestAuthenticatedTransport = () => {
    if (authMode === "anonymous") return;
    if (sessionIsExternallyManaged && !lifecycle.knownAuthToken) {
      awaitExternalSessionChange();
      return;
    }
    requestReconnect("immediate");
  };

  const canSendSubscriptionMessage = (
    message: SubscriptionMessage
  ): boolean => {
    if (
      authMode !== "anonymous" &&
      lifecycle.connectionAuth === "anonymous" &&
      subscriptionRequiresAuth(message)
    ) {
      requestAuthenticatedTransport();
      return false;
    }

    if (
      lifecycle.connectionAuth === "anonymous" &&
      wantsAuthenticatedConnection()
    ) {
      requestReconnect("immediate");
      return false;
    }

    return true;
  };

  const sendSerialized = (payload: string) => {
    const socket = ws;
    if (!socket || !isOpenSocket(socket)) return;
    if (
      lifecycle.connectionAuth === "anonymous" &&
      wantsAuthenticatedConnection()
    ) {
      requestReconnect("immediate");
      return;
    }
    socket.send(payload);
  };

  const sendSubscriptionMessage = (message: SubscriptionMessage) => {
    if (!canSendSubscriptionMessage(message)) return;
    sendSerialized(JSON.stringify(message));
  };

  const onClose = async (socket: WebSocket, evt?: CloseEvent) => {
    if (!isCurrentSocket(socket)) return;
    lifecycle.send({
      type: "SOCKET_CLOSE",
      code: evt?.code,
      reason: evt?.reason,
    });
    ws = undefined;
    flushEffects();
  };

  const handleAuthClosePolicy = async (): Promise<void> => {
    if (lifecycle.isClosing) return;
    const sessionManager = opts.sessionManager;
    if (!sessionManager) return;

    if (sessionIsExternallyManaged) {
      awaitExternalSessionChange();
      return;
    }

    if (!opts.refreshFn) return;

    try {
      await sessionManager.refreshWith(opts.refreshFn);
      requestReconnect("immediate");
    } catch (error) {
      // `refreshWith` already clears terminal refresh failures. Preserve the
      // current session on transient errors so websocket auth issues do not
      // force a full logout.
      scheduleAuthRefreshRetry(error, () => {
        void handleAuthClosePolicy();
      });
    }
  };

  const onError = async (socket: WebSocket) => {
    if (!isCurrentSocket(socket)) return;
    lifecycle.send({
      type: "SOCKET_ERROR",
      message: "WebSocket connection error",
    });
    const activeSocket = ws;
    ws = undefined;
    try {
      activeSocket?.close();
    } catch {
      // Best-effort close.
    }
    flushEffects();

    const error = createConnectionError("WebSocket connection error", {
      operation: "connection",
    });
    await handleError(error);
  };

  const mergeProtocolToken = (
    protocol: string | string[] | undefined,
    token: string | null
  ): string | string[] | undefined => {
    if (!token) return protocol;
    if (!protocol) return ["phoenix-jwt", token];

    if (Array.isArray(protocol)) {
      const next = [...protocol];
      let injected = false;
      for (let i = 0; i < next.length; i += 1) {
        const current = next[i];
        const lower = current.toLowerCase();
        if (lower === "phoenix-jwt") {
          if (i + 1 < next.length) {
            next[i + 1] = token;
          } else {
            next.splice(i + 1, 0, token);
          }
          injected = true;
          break;
        }
        if (lower.startsWith("phoenix-jwt=")) {
          next[i] = `phoenix-jwt=${token}`;
          injected = true;
          break;
        }
        if (lower.startsWith("phoenix-jwt:")) {
          next[i] = `phoenix-jwt:${token}`;
          injected = true;
          break;
        }
      }
      if (!injected) {
        next.push("phoenix-jwt", token);
      }
      return next;
    }

    const trimmed = protocol.trim();
    const lower = trimmed.toLowerCase();
    if (lower === "phoenix-jwt") return ["phoenix-jwt", token];
    if (lower.startsWith("phoenix-jwt=")) return `phoenix-jwt=${token}`;
    if (lower.startsWith("phoenix-jwt:")) return `phoenix-jwt:${token}`;
    if (trimmed.includes(",")) {
      const parts = trimmed
        .split(",")
        .map((part) => part.trim())
        .filter(Boolean);
      return mergeProtocolToken(parts, token);
    }
    return [protocol, "phoenix-jwt", token];
  };

  const extractProtocolToken = (
    protocol?: string | string[]
  ): string | null => {
    if (!protocol) return null;
    const protocols = Array.isArray(protocol)
      ? protocol
      : protocol.split(",").map((part) => part.trim());
    for (let i = 0; i < protocols.length; i += 1) {
      const current = protocols[i];
      const lower = current.toLowerCase();
      if (lower === "phoenix-jwt") {
        const next = protocols[i + 1];
        if (next) return next;
      }
      if (lower.startsWith("phoenix-jwt=")) {
        return current.slice("phoenix-jwt=".length);
      }
      if (lower.startsWith("phoenix-jwt:")) {
        return current.slice("phoenix-jwt:".length);
      }
    }
    return null;
  };

  const getSessionForProtocol = async (): Promise<
    { accessToken: string } | null | DeferredSessionOwnerConnect
  > => {
    const sessionManager = opts.sessionManager;
    if (!sessionManager || authMode === "anonymous") return null;

    const session = await sessionManager.getSession();
    if (!session) return null;

    if (!isAccessExpiringWithin(session, ACCESS_REFRESH_WINDOW_MS)) {
      return session;
    }

    if (sessionIsExternallyManaged) {
      awaitExternalSessionChange();
      return DEFER_CONNECT_FOR_SESSION_OWNER;
    }

    if (!opts.refreshFn || isRefreshExpired(session)) {
      return isAccessExpired(session) ? null : session;
    }

    try {
      suppressAuthChangeDuringProtocolRefresh = true;
      return await sessionManager.refreshWith(opts.refreshFn);
    } catch (error) {
      const willRetry = scheduleAuthRefreshRetry(error, () => {
        requestReconnect("immediate");
      });
      if (willRetry) {
        return DEFER_CONNECT_FOR_SESSION_OWNER;
      }
      return isAccessExpired(session) ? null : session;
    } finally {
      suppressAuthChangeDuringProtocolRefresh = false;
    }
  };

  const resolveProtocol = async (): Promise<
    ResolvedProtocol | DeferredSessionOwnerConnect
  > => {
    if (authMode === "anonymous") return config.protocol;
    const session = await getSessionForProtocol();
    if (session === DEFER_CONNECT_FOR_SESSION_OWNER) {
      return DEFER_CONNECT_FOR_SESSION_OWNER;
    }
    if (!session) return config.protocol;
    return mergeProtocolToken(config.protocol, session.accessToken);
  };

  const handleAuthChange = (session: { accessToken: string } | null) => {
    if (suppressAuthChangeDuringProtocolRefresh) return;
    lifecycle.send({
      type: "AUTH_SESSION_CHANGED",
      token: session?.accessToken ?? null,
      hasSocket: Boolean(ws),
    });
    flushEffects();
  };

  const sessionUnsub = opts.sessionManager?.onChange(handleAuthChange);

  const connect = async () => {
    if (!shouldMaintainConnection()) return;
    if (lifecycle.isClosing || connectInFlight) return;
    clearReconnectTimer();
    clearIdleCloseTimer();

    connectInFlight = (async () => {
      try {
        const protocol = await resolveProtocol();
        if (protocol === DEFER_CONNECT_FOR_SESSION_OWNER) {
          return;
        }
        const protocolToken = extractProtocolToken(protocol);
        lifecycle.send({ type: "AUTH_TOKEN_OBSERVED", token: protocolToken });

        if (authMode === "authenticated" && !protocolToken) {
          lifecycle.send({ type: "AWAIT_AUTH" });
          return;
        }

        const pendingAuth =
          protocolToken !== null ? "authenticated" : "anonymous";
        lifecycle.send({ type: "CONNECT_START", pendingAuth });

        const websocket = new WebSocket(config.url, protocol);
        ws = websocket;

        websocket.onopen = () => {
          if (!isCurrentSocket(websocket)) {
            try {
              websocket.close();
            } catch {
              // Best-effort close for stale sockets.
            }
            return;
          }
          lifecycle.send({ type: "SOCKET_OPEN" });
          for (const sub of subscriptionRegistry.values()) {
            sub.onResub?.();
          }
        };

        websocket.onmessage = (evt) => {
          if (!isCurrentSocket(websocket)) return;
          if (typeof evt.data !== "string") return;
          void handleMessage(evt.data, subscriptionRegistry);
        };
        websocket.onclose = (evt) => void onClose(websocket, evt);
        websocket.onerror = () => {
          void onError(websocket);
        };
      } catch (error) {
        const wsError = createConnectionError(
          "WebSocket connection failed",
          {
            operation: "connection",
            attempt: lifecycle.reconnectAttempt + 1,
            maxAttempts: 10,
          },
          error instanceof Error ? error : new Error(String(error))
        );
        await handleError(wsError);
        requestReconnect("backoff");
      }
    })().finally(() => {
      connectInFlight = null;
      flushEffects();
      if (!shouldMaintainConnection()) {
        scheduleIdleClose();
      }
      if (!lifecycle.isClosing && deferredScheduleImmediate) {
        deferredScheduleImmediate = false;
        requestReconnect("immediate");
      }
    });
  };

  if (config.connectMode === "eager") {
    void connect();
  }

  return {
    subscribe(
      key: string,
      subMsg: SubscriptionMessage,
      onMessage: (data: unknown) => void,
      options?: SubscriptionOptions
    ) {
      const routingKey = options?.routingKey ?? key;
      const existingListener = subscriptionListeners.get(key);
      if (existingListener && existingListener.routingKey !== routingKey) {
        throw new Error(
          `WebSocket listener '${key}' already registered for '${existingListener.routingKey}'`
        );
      }

      const listenerKeys =
        listenerKeysByRoutingKey.get(routingKey) ?? new Set<string>();
      const isFirstRoutingListener = listenerKeys.size === 0;
      listenerKeys.add(key);
      listenerKeysByRoutingKey.set(routingKey, listenerKeys);
      subscriptionListeners.set(key, {
        routingKey,
        subMsg,
        onMessage,
      });
      syncRoutingSubscription(routingKey);
      ensureConnection();

      if (canUseCurrentSocketForMessage(subMsg)) {
        if (isFirstRoutingListener || existingListener) {
          sendSubscriptionMessage(subMsg);
        }
      }
    },

    unsubscribe(key: string, unsubMsg: SubscriptionMessage) {
      const listener = subscriptionListeners.get(key);
      if (!listener) {
        return;
      }

      subscriptionListeners.delete(key);
      const listenerKeys = listenerKeysByRoutingKey.get(listener.routingKey);
      listenerKeys?.delete(key);
      const isLastRoutingListener = !listenerKeys || listenerKeys.size === 0;
      if (listenerKeys && listenerKeys.size === 0) {
        listenerKeysByRoutingKey.delete(listener.routingKey);
      }
      syncRoutingSubscription(listener.routingKey);
      if (!hasActiveSubscriptions()) {
        clearReconnectTimer();
        scheduleIdleClose();
      }

      if (canUseCurrentSocketForMessage(unsubMsg)) {
        if (isLastRoutingListener) {
          sendSubscriptionMessage(unsubMsg);
        }
      }
    },

    registerChannel(channel: string, registration: WsChannelRegistration) {
      const existingCustom = customChannelRegistry.get(channel);
      if (existingCustom) {
        if (
          existingCustom.validate === registration.validate &&
          existingCustom.getKey === registration.getKey
        ) {
          return () => {};
        }
        throw new Error(`WebSocket channel already registered: ${channel}`);
      }

      if (pluginRegistry.getHandler(channel)) {
        throw new Error(`WebSocket channel already registered: ${channel}`);
      }

      customChannelRegistry.set(channel, registration);
      pluginRegistry.register(() => ({
        channel,
        validate: registration.validate,
        getKey: registration.getKey,
        handle: async (
          message: unknown,
          registry: Map<string, Subscription>
        ): Promise<void> => {
          const sub = registry.get(registration.getKey(message));
          sub?.onMsg(message);
        },
      }));

      return () => {
        customChannelRegistry.delete(channel);
        pluginRegistry.unregister(channel);
      };
    },

    close() {
      lifecycle.send({ type: "CLOSE_CLIENT" });
      clearReconnectTimer();
      clearIdleCloseTimer();
      clearAuthRefreshRetryTimer();
      sessionUnsub?.();
      ws?.close();
      subscriptionRegistry.clear();
      subscriptionListeners.clear();
      listenerKeysByRoutingKey.clear();
      customChannelRegistry.clear();
      lifecycle.stop();
    },

    onServerError(listener: WsServerErrorListener) {
      serverErrorListeners.add(listener);
      return () => {
        serverErrorListeners.delete(listener);
      };
    },
  };
};
