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
import { subscriptionIdentity } from "./subscriptionIdentity";
import {
  getPhoenixClientHeader,
  PHOENIX_CLIENT_HEADER_NAME,
} from "@/clientIdentity";
import type {
  Subscription,
  SubscriptionOptions,
  SubscriptionMessage,
  WsChannelRegistration,
  WsClient,
  WsClientConfig,
  WsClientOpts,
  WsServerErrorListener,
  WsServerErrorMessage,
  WsSubscriptionStatusListener,
  WsSubscriptionStatusMessage,
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
const DEFAULT_HEARTBEAT_IDLE_TIMEOUT_MS = 90_000;
// The server broadcasts `allMids` every few seconds while it has subscribers,
// and browsers do not expose ping frames, so an internal `allMids`
// subscription is the heartbeat. It shares the `allMids` adapter routing key.
const HEARTBEAT_ROUTING_KEY = "allMids";
const HEARTBEAT_SUBSCRIBE_MESSAGE: SubscriptionMessage = {
  type: "subscribe",
  subscription: { channel: "allMids" },
};
const HEARTBEAT_IDENTITY = subscriptionIdentity(
  HEARTBEAT_SUBSCRIBE_MESSAGE.subscription
);
const IDLE_TIMEOUT_CLOSE = { code: 4000, reason: "idle_timeout" } as const;
const RESPONSE_MISMATCH_CLOSE = {
  code: 4001,
  reason: "subscription_response_mismatch",
} as const;

type SubscriptionListener = {
  routingKey: string;
  subMsg: SubscriptionMessage;
  onMessage: (data: unknown) => void;
  onError?: SubscriptionOptions["onError"];
  onClose?: SubscriptionOptions["onClose"];
};

/** A subscribe or unsubscribe frame sent on the current socket, unanswered. */
type PendingFrame = {
  op: SubscriptionMessage["type"];
  identity: string | null;
  routingKey: string;
  subscription: Record<string, unknown>;
};

/**
 * The previous response can identify a server-ended subscription's trailing
 * error. Other subscriptions' responses may arrive independently, so it must
 * not be used to impose a global ordering on pending requests.
 */
type PreviousResponse =
  | { type: "none" }
  | { type: "traderStateAck"; identity: string }
  | {
      type: "serverEnded" | "maybeEnded";
      identity: string;
      subscription: Record<string, unknown>;
      listeners: SubscriptionListener[];
    };

const NO_PREVIOUS_RESPONSE: PreviousResponse = { type: "none" };

const notifyListeners = (
  listeners: readonly SubscriptionListener[],
  notify: (listener: SubscriptionListener) => void
) => {
  for (const listener of listeners) {
    try {
      notify(listener);
    } catch {
      // Listener errors must not disrupt websocket processing.
    }
  }
};

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

  const heartbeat =
    opts.heartbeat === false
      ? null
      : (opts.heartbeat ?? {
          idleTimeoutMs: DEFAULT_HEARTBEAT_IDLE_TIMEOUT_MS,
        });
  let heartbeatTimer: ReturnType<typeof setTimeout> | null = null;
  // Whether an `allMids` subscription (internal or a caller's) was sent on the
  // current socket and not ended by the server.
  let heartbeatSubscribed = false;

  const subscriptionRegistry = new Map<string, Subscription>();
  const subscriptionListeners = new Map<string, SubscriptionListener>();
  const listenerKeysByRoutingKey = new Map<string, Set<string>>();
  const serverErrorListeners = new Set<WsServerErrorListener>();
  if (opts.onServerError) {
    serverErrorListeners.add(opts.onServerError);
  }
  const subscriptionStatusListeners = new Set<WsSubscriptionStatusListener>();
  if (opts.onSubscriptionStatus) {
    subscriptionStatusListeners.add(opts.onSubscriptionStatus);
  }

  // Per-socket response tracking; reset whenever a socket opens or closes.
  let pendingFrames: PendingFrame[] = [];
  const acknowledgedIdentities = new Set<string>();
  const previousResponses = new Map<string, PreviousResponse>();

  const resetResponseTracking = () => {
    pendingFrames = [];
    acknowledgedIdentities.clear();
    previousResponses.clear();
    heartbeatSubscribed = false;
  };

  const emitServerError = (message: WsServerErrorMessage) => {
    for (const listener of serverErrorListeners) {
      try {
        listener(message);
      } catch {
        // Listener errors must not disrupt websocket processing.
      }
    }
  };

  const getListenersForRoutingKey = (routingKey: string) => {
    const listenerKeys = listenerKeysByRoutingKey.get(routingKey);
    if (!listenerKeys) {
      return [];
    }

    const listeners: SubscriptionListener[] = [];
    for (const listenerKey of listenerKeys) {
      const listener = subscriptionListeners.get(listenerKey);
      if (listener) {
        listeners.push(listener);
      }
    }
    return listeners;
  };

  const emitSubscriptionStatus = (
    message: WsSubscriptionStatusMessage,
    identity: string | null
  ) => {
    if (
      heartbeat &&
      identity !== null &&
      identity === HEARTBEAT_IDENTITY &&
      routingKeysForIdentity(identity).length === 0
    ) {
      // Status of the internal heartbeat subscription only.
      return;
    }
    for (const listener of subscriptionStatusListeners) {
      try {
        listener(message);
      } catch {
        // Listener errors must not disrupt websocket processing.
      }
    }
  };

  const routingKeysForIdentity = (identity: string): string[] => {
    const routingKeys = new Set<string>();
    for (const listener of subscriptionListeners.values()) {
      if (subscriptionIdentity(listener.subMsg.subscription) === identity) {
        routingKeys.add(listener.routingKey);
      }
    }
    return [...routingKeys];
  };

  /**
   * Drops listeners the server rejected or ended, unless a newer subscribe
   * for this routing key is still awaiting its own response.
   */
  const dropRoutingKey = (routingKey: string): SubscriptionListener[] => {
    if (
      pendingFrames.some(
        (frame) => frame.op === "subscribe" && frame.routingKey === routingKey
      )
    ) {
      return [];
    }
    const listeners = getListenersForRoutingKey(routingKey);
    for (const listenerKey of listenerKeysByRoutingKey.get(routingKey) ?? []) {
      subscriptionListeners.delete(listenerKey);
    }
    listenerKeysByRoutingKey.delete(routingKey);
    syncRoutingSubscription(routingKey);
    if (listeners.length > 0 && !hasActiveSubscriptions()) {
      clearReconnectTimer();
      scheduleIdleClose();
    }
    return listeners;
  };

  const dropIdentity = (identity: string): SubscriptionListener[] =>
    routingKeysForIdentity(identity).flatMap(dropRoutingKey);

  const failClosed = (detail: string, errorFrame?: WsServerErrorMessage) => {
    debugWs("subscription response mismatch", { detail });
    if (errorFrame) {
      emitServerError({
        channel: "error",
        error: errorFrame.error,
        code: errorFrame.code,
      });
    }
    const socket = ws;
    if (socket) {
      closeSocketForReconnect(socket, RESPONSE_MISMATCH_CLOSE);
    }
  };

  const endSubscription = (
    listeners: SubscriptionListener[],
    subscription: Record<string, unknown>,
    identity: string
  ) => {
    previousResponses.set(identity, {
      type: "serverEnded",
      identity,
      subscription,
      listeners,
    });
    notifyListeners(listeners, (listener) => listener.onClose?.());
  };

  const applySubscribed = (identity: string, channel: unknown) => {
    const index = pendingFrames.findIndex(
      (frame) => frame.op === "subscribe" && frame.identity === identity
    );
    if (index === -1) return;
    pendingFrames.splice(index, 1);
    // An unsubscribe already sent after this request ends its generation,
    // even if that unsubscribe's acknowledgement has not arrived yet.
    if (
      !pendingFrames.some(
        (frame) => frame.op === "unsubscribe" && frame.identity === identity
      )
    ) {
      acknowledgedIdentities.add(identity);
    }
    if (channel === "traderState") {
      previousResponses.set(identity, { type: "traderStateAck", identity });
    }
  };

  /** Applies an echoed unsubscribe independently of other subscriptions. */
  const applyUnsubscribed = (
    identity: string,
    previous: PreviousResponse,
    message: WsSubscriptionStatusMessage
  ): boolean => {
    acknowledgedIdentities.delete(identity);
    const index = pendingFrames.findIndex(
      (frame) => frame.identity === identity
    );
    const head = pendingFrames[index];
    if (identity === HEARTBEAT_IDENTITY) {
      // A later heartbeat subscribe still covers the current generation.
      heartbeatSubscribed =
        heartbeat !== null &&
        pendingFrames.some(
          (frame, frameIndex) =>
            frameIndex > index &&
            frame.op === "subscribe" &&
            frame.identity === identity
        );
    }
    if (previous.type === "traderStateAck" && previous.identity === identity) {
      if (head?.op === "unsubscribe" && head.identity === identity) {
        // Either the reply to that unsubscribe or the server ending the
        // subscription; both leave it closed.
        pendingFrames.splice(index, 1);
        previousResponses.set(identity, {
          type: "maybeEnded",
          identity,
          subscription: message.subscription,
          listeners: [],
        });
        return true;
      }
      endSubscription(dropIdentity(identity), message.subscription, identity);
      return true;
    }

    if (index !== -1) {
      pendingFrames.splice(index, 1);
      if (head.op === "subscribe") {
        // The server ended a subscription it had not acknowledged (exchange
        // bootstrap failure).
        endSubscription(
          dropRoutingKey(head.routingKey),
          message.subscription,
          identity
        );
      }
      return true;
    }
    const listeners = dropIdentity(identity);
    if (listeners.length > 0) {
      endSubscription(listeners, message.subscription, identity);
    }
    return true;
  };

  const handleSubscriptionStatusFrame = (
    message: WsSubscriptionStatusMessage
  ) => {
    const identity = subscriptionIdentity(message.subscription);
    if (identity !== null) {
      const previous = previousResponses.get(identity) ?? NO_PREVIOUS_RESPONSE;
      previousResponses.delete(identity);
      if (message.status === "subscribed") {
        applySubscribed(identity, message.subscription.channel);
      }
      if (message.status === "unsubscribed") {
        applyUnsubscribed(identity, previous, message);
      }
    }
    emitSubscriptionStatus(message, identity);
  };

  const reportAttributedError = (
    message: WsServerErrorMessage,
    subscription: Record<string, unknown>,
    listeners: readonly SubscriptionListener[],
    closeListeners: boolean
  ) => {
    const error = { code: message.code, error: message.error };
    notifyListeners(listeners, (listener) =>
      listener.onError?.(error, { terminal: closeListeners })
    );
    if (closeListeners) {
      notifyListeners(listeners, (listener) => listener.onClose?.());
    }
    emitServerError({ channel: "error", ...error, subscription });
  };

  /** Rejects the matching request without ending an acknowledged refresh. */
  const rejectPending = (
    index: number,
    message: WsServerErrorMessage,
    subscription: Record<string, unknown>
  ) => {
    const [head] = pendingFrames.splice(index, 1);
    const isRefresh =
      head.identity !== null && acknowledgedIdentities.has(head.identity);
    if (head.op === "subscribe" && head.identity === HEARTBEAT_IDENTITY) {
      heartbeatSubscribed =
        heartbeat !== null &&
        (isRefresh ||
          pendingFrames.some(
            (frame) =>
              frame.op === "subscribe" && frame.identity === HEARTBEAT_IDENTITY
          ));
    }
    const listeners =
      head.op !== "subscribe"
        ? []
        : isRefresh
          ? getListenersForRoutingKey(head.routingKey)
          : dropRoutingKey(head.routingKey);
    reportAttributedError(message, subscription, listeners, !isRefresh);
  };

  const handleServerErrorFrame = (message: WsServerErrorMessage) => {
    const head = pendingFrames[0];
    const echo = message.subscription;
    const echoIdentity = echo ? subscriptionIdentity(echo) : null;

    if (echo && echoIdentity !== null) {
      const previous =
        previousResponses.get(echoIdentity) ?? NO_PREVIOUS_RESPONSE;
      // An echo identifies a subscription, not a request attempt. Statuses
      // and errors can overtake each other even for this same identity.
      let possibleOwners =
        previous.type === "serverEnded" || previous.type === "maybeEnded"
          ? 1
          : 0;
      for (const frame of pendingFrames) {
        if (frame.identity === echoIdentity && ++possibleOwners > 1) {
          failClosed("echoed error has multiple possible requests", message);
          return;
        }
      }
      previousResponses.delete(echoIdentity);
      if (previous.type === "serverEnded" || previous.type === "maybeEnded") {
        // No pending attempt can own this ended subscription's error.
        reportAttributedError(message, echo, previous.listeners, false);
        return;
      }
      const index = pendingFrames.findIndex(
        (frame) => frame.identity === echoIdentity
      );
      if (index !== -1) {
        rejectPending(index, message, echo);
        return;
      }
      const listeners = routingKeysForIdentity(echoIdentity).flatMap(
        getListenersForRoutingKey
      );
      reportAttributedError(message, echo, listeners, false);
      return;
    }

    // Legacy errors have no correlation key. Only attribute one possible
    // request/context; independent queues make global FIFO guesses unsafe.
    const ended = [...previousResponses.values()].filter(
      (response) =>
        response.type === "serverEnded" || response.type === "maybeEnded"
    );
    if (pendingFrames.length + ended.length > 1) {
      failClosed("legacy error has multiple possible requests", message);
      return;
    }
    const [previous] = ended;
    if (
      previous &&
      (previous.type === "serverEnded" || previous.type === "maybeEnded")
    ) {
      previousResponses.delete(previous.identity);
      reportAttributedError(
        message,
        previous.subscription,
        previous.listeners,
        false
      );
      return;
    }
    if (head) {
      if (head.identity !== null) previousResponses.delete(head.identity);
      rejectPending(0, message, head.subscription);
      return;
    }
    emitServerError({
      channel: "error",
      error: message.error,
      code: message.code,
    });
  };

  const pluginRegistry = createPluginRegistry(
    createDefaultPlugins({
      onServerErrorFrame: handleServerErrorFrame,
      onSubscriptionStatusFrame: handleSubscriptionStatusFrame,
    })
  );
  const customChannelRegistry = new Map<string, WsChannelRegistration>();
  const handleMessage = createMessageHandler(pluginRegistry);

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
        sendSubscriptionMessage(listener.subMsg, routingKey);
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

  const clearHeartbeatTimer = () => {
    if (!heartbeatTimer) return;
    clearTimeout(heartbeatTimer);
    heartbeatTimer = null;
  };

  /** (Re)starts the idle timeout of `socket`; any inbound message resets it. */
  const armHeartbeat = (socket: WebSocket) => {
    if (!heartbeat) return;
    clearHeartbeatTimer();
    heartbeatTimer = setTimeout(() => {
      heartbeatTimer = null;
      if (!isCurrentSocket(socket)) return;
      debugWs("ws idle timeout", { idleTimeoutMs: heartbeat.idleTimeoutMs });
      closeSocketForReconnect(socket, IDLE_TIMEOUT_CLOSE);
    }, heartbeat.idleTimeoutMs);
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
    // A socket still in its handshake resubscribes every registered listener
    // from `onopen`, so it already covers subscriptions added in the meantime.
    if (lifecycle.state === "connecting" && ws) {
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

  const sendSubscriptionMessage = (
    message: SubscriptionMessage,
    routingKey: string
  ) => {
    if (!canSendSubscriptionMessage(message)) return;
    const socket = ws;
    if (!socket || !isOpenSocket(socket)) return;
    if (
      lifecycle.connectionAuth === "anonymous" &&
      wantsAuthenticatedConnection()
    ) {
      requestReconnect("immediate");
      return;
    }
    socket.send(JSON.stringify(message));
    const identity = subscriptionIdentity(message.subscription);
    if (message.type === "unsubscribe" && identity !== null) {
      acknowledgedIdentities.delete(identity);
    }
    pendingFrames.push({
      op: message.type,
      identity,
      routingKey,
      subscription: message.subscription,
    });
    if (message.type === "subscribe" && identity === HEARTBEAT_IDENTITY) {
      // Only an enabled heartbeat keeps its `allMids` subscription on the
      // server; a caller's own `allMids` subscribe must not latch this.
      heartbeatSubscribed = heartbeat !== null;
    }
  };

  const onClose = async (
    socket: WebSocket,
    evt?: { code?: number; reason?: string }
  ) => {
    if (!isCurrentSocket(socket)) return;
    clearHeartbeatTimer();
    resetResponseTracking();
    lifecycle.send({
      type: "SOCKET_CLOSE",
      code: evt?.code,
      reason: evt?.reason,
    });
    ws = undefined;
    flushEffects();
  };

  /**
   * Closes `socket` and detaches it right away, so the reconnect path runs
   * even when a half-open socket never completes the closing handshake.
   */
  const closeSocketForReconnect = (
    socket: WebSocket,
    { code, reason }: { code: number; reason: string }
  ) => {
    try {
      socket.close(code, reason);
    } catch {
      // Best-effort close.
    }
    void onClose(socket, { code, reason });
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
    clearHeartbeatTimer();
    resetResponseTracking();
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

        const url = new URL(config.url, globalThis.location?.href);
        const clientIdentity =
          getPhoenixClientHeader()[PHOENIX_CLIENT_HEADER_NAME];
        if (clientIdentity) {
          url.searchParams.set("phoenixClient", clientIdentity);
        }
        const websocket = new WebSocket(url.toString(), protocol);
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
          resetResponseTracking();
          armHeartbeat(websocket);
          for (const sub of subscriptionRegistry.values()) {
            sub.onResub?.();
          }
          if (heartbeat && !heartbeatSubscribed) {
            sendSubscriptionMessage(
              HEARTBEAT_SUBSCRIBE_MESSAGE,
              HEARTBEAT_ROUTING_KEY
            );
          }
        };

        websocket.onmessage = (evt) => {
          if (!isCurrentSocket(websocket)) return;
          armHeartbeat(websocket);
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
        onError: options?.onError,
        onClose: options?.onClose,
      });
      syncRoutingSubscription(routingKey);
      ensureConnection();

      if (canUseCurrentSocketForMessage(subMsg)) {
        // A caller's `allMids` rides on the heartbeat's server subscription.
        const coveredByHeartbeat =
          heartbeat !== null &&
          heartbeatSubscribed &&
          !existingListener &&
          subscriptionIdentity(subMsg.subscription) === HEARTBEAT_IDENTITY;
        if (
          (isFirstRoutingListener || existingListener) &&
          !coveredByHeartbeat
        ) {
          sendSubscriptionMessage(subMsg, routingKey);
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

      // Keep the heartbeat's `allMids` subscription on the server when the
      // last caller listener leaves.
      const keepForHeartbeat =
        heartbeat !== null &&
        subscriptionIdentity(unsubMsg.subscription) === HEARTBEAT_IDENTITY;
      if (canUseCurrentSocketForMessage(unsubMsg)) {
        if (isLastRoutingListener && !keepForHeartbeat) {
          sendSubscriptionMessage(unsubMsg, listener.routingKey);
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
      clearHeartbeatTimer();
      resetResponseTracking();
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

    onSubscriptionStatus(listener: WsSubscriptionStatusListener) {
      subscriptionStatusListeners.add(listener);
      return () => {
        subscriptionStatusListeners.delete(listener);
      };
    },
  };
};
