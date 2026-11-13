import type { AuthSession, RiseSessionControl } from "@/auth/session";
import type { AuthSessionManager } from "@/auth/manager";
import type { RiseWsAuthMode } from "@/auth/types";

export type WsServerErrorMessage = {
  channel: "error";
  error: string;
  code: number;
  /**
   * The subscription request this error was attributed to, using the server's
   * echo or a single possible owner when a legacy server omits the echo.
   * Echoes identify subscriptions, not request attempts. Ambiguous errors omit
   * this field and trigger reconnect rather than guessing an attempt. Also
   * absent when no subscription can be identified, such as a malformed frame.
   */
  subscription?: Record<string, unknown>;
};

/**
 * A `subscriptionStatus` frame from the server. `subscription` is the
 * server's echo of the request.
 */
export type WsSubscriptionStatusMessage = {
  channel: "subscriptionStatus";
  status: string;
  subscription: Record<string, unknown>;
  clientId?: string;
};

export type WsConnectMode = "eager" | "lazy";

export interface WsHeartbeatOpts {
  /**
   * Close and reconnect the socket after this much inbound silence.
   * Defaults to 90 seconds.
   */
  idleTimeoutMs: number;
}

export interface WsClientOpts {
  url: string;
  protocol?: string | string[] | undefined;
  backoff?: { baseMs: number; maxMs: number };
  connectMode?: WsConnectMode;
  idleCloseMs?: number;
  /**
   * Liveness check for half-open sockets. While enabled, the client keeps an
   * internal `allMids` subscription (never delivered to callers) so the server
   * sends a frame every few seconds, and closes the socket with code 4000
   * (`idle_timeout`) after `idleTimeoutMs` without any inbound message. The
   * normal reconnect path then opens a new socket and resubscribes.
   * Defaults to `{ idleTimeoutMs: 90_000 }`; pass `false` to disable.
   */
  heartbeat?: WsHeartbeatOpts | false;
  sessionManager?: AuthSessionManager;
  refreshFn?: (refreshToken: string) => Promise<AuthSession>;
  sessionControl?: RiseSessionControl;
  authMode?: RiseWsAuthMode;
  onServerError?: WsServerErrorListener;
  onSubscriptionStatus?: WsSubscriptionStatusListener;
}

export type WsClientConfig = Omit<WsClientOpts, "backoff" | "connectMode"> & {
  backoff: Required<NonNullable<WsClientOpts["backoff"]>>;
  connectMode: NonNullable<WsClientOpts["connectMode"]>;
};

export interface SubscriptionMessage {
  type: "subscribe" | "unsubscribe";
  subscription: Record<string, unknown>;
}

export interface WsChannelRegistration {
  validate: (message: unknown) => boolean;
  getKey: (message: unknown) => string;
}

export interface SubscriptionOptions {
  routingKey?: string;
  /**
   * Called for every server error attributed to this subscription. A rejected
   * refresh is nonterminal: the acknowledged subscription remains active.
   * Omitted context from a custom transport means terminal rejection.
   */
  onError?: (
    error: { code: number; error: string },
    context?: { terminal: boolean }
  ) => void;
  /**
   * Called once when the client drops this subscription because the server
   * rejected or ended it. The listener is removed and is not resubscribed on
   * reconnect; subscribe again to retry.
   */
  onClose?: () => void;
}

export interface WsClient {
  subscribe(
    key: string,
    subMsg: SubscriptionMessage,
    onMessage: (data: unknown) => void,
    options?: SubscriptionOptions
  ): void;
  unsubscribe(key: string, unsubMsg: SubscriptionMessage): void;
  registerChannel(
    channel: string,
    registration: WsChannelRegistration
  ): () => void;
  close(): void;
  onServerError(listener: WsServerErrorListener): () => void;
  /**
   * Listen to `subscriptionStatus` frames. Optional: transports that never
   * report statuses (older clients, custom transports) may omit it, and
   * adapters must not require it.
   */
  onSubscriptionStatus?(listener: WsSubscriptionStatusListener): () => void;
}

export type Subscription = {
  onMsg: (data: unknown) => void;
  onResub?: () => void;
};

export type WsServerErrorListener = (message: WsServerErrorMessage) => void;

export type WsSubscriptionStatusListener = (
  message: WsSubscriptionStatusMessage
) => void;
