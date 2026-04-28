import type { AuthSession } from "@/auth/session";
import type { AuthSessionManager } from "@/auth/manager";
import type { RiseSessionControl, RiseWsAuthMode } from "@/auth/types";

export type WsServerErrorMessage = {
  channel: "error";
  error: string;
  code: number;
};

export type WsConnectMode = "eager" | "lazy";

export interface WsClientOpts {
  url: string;
  protocol?: string | string[] | undefined;
  backoff?: { baseMs: number; maxMs: number };
  connectMode?: WsConnectMode;
  idleCloseMs?: number;
  sessionManager?: AuthSessionManager;
  refreshFn?: (refreshToken: string) => Promise<AuthSession>;
  sessionControl?: RiseSessionControl;
  authMode?: RiseWsAuthMode;
  onServerError?: WsServerErrorListener;
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
}

export type Subscription = {
  onMsg: (data: unknown) => void;
  onResub?: () => void;
};

export type WsServerErrorListener = (message: WsServerErrorMessage) => void;
