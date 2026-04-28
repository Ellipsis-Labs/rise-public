import { assign, createActor, setup } from "xstate";
import { computeBackoffMs } from "./backoff";
import { debugWs } from "./debug";

export type WsConnectionAuth = "anonymous" | "authenticated";
export type ReconnectMode = "backoff" | "immediate";
export type WsAuthMode = "auto" | "authenticated" | "anonymous";
export type WsLifecycleErrorReason =
  | "socket_error"
  | "socket_close"
  | "auth_session_missing"
  | "auth_close_policy";

export type WsLifecycleError = {
  reason: WsLifecycleErrorReason;
  code?: number;
  message?: string;
};

export type WsLifecycleEffect =
  | { type: "close_socket" }
  | { type: "schedule_connect"; waitMs: number; mode: ReconnectMode }
  | { type: "handle_auth_close_policy" };

type WsLifecycleContext = {
  closeForReconnect: boolean;
  connectionAuth: WsConnectionAuth;
  pendingAuth: WsConnectionAuth;
  reconnectAttempt: number;
  reconnectQueued: boolean;
  reconnectMode: ReconnectMode;
  knownAuthToken: string | null;
  authMode: WsAuthMode;
  lastError: WsLifecycleError | null;
  effects: WsLifecycleEffect[];
  backoff: { baseMs: number; maxMs: number };
};

type WsLifecycleEvent =
  | { type: "CONNECT_START"; pendingAuth: WsConnectionAuth }
  | { type: "AWAIT_AUTH" }
  | { type: "SOCKET_OPEN" }
  | { type: "SOCKET_CLOSE"; code?: number; reason?: string }
  | { type: "SOCKET_ERROR"; message?: string }
  | { type: "AUTH_SESSION_CHANGED"; token: string | null; hasSocket: boolean }
  | { type: "AUTH_TOKEN_OBSERVED"; token: string | null }
  | { type: "REQUEST_RECONNECT"; mode: ReconnectMode; hasSocket: boolean }
  | { type: "CLOSE_CLIENT" }
  | { type: "CLEAR_EFFECTS" };

const AUTH_CLOSE_CODES = new Set([4401, 4403, 1008]);
const AUTH_CLOSE_REASONS = new Set([
  "access_token_expired",
  "invalid_access_token",
  "access_jti_mismatch",
  "session_missing",
]);

const nextReconnect = (
  context: WsLifecycleContext,
  mode: ReconnectMode
): { reconnectAttempt: number; effect: WsLifecycleEffect } => {
  if (mode === "immediate") {
    return {
      reconnectAttempt: 0,
      effect: { type: "schedule_connect", waitMs: 0, mode },
    };
  }

  const reconnectAttempt = context.reconnectAttempt + 1;
  return {
    reconnectAttempt,
    effect: {
      type: "schedule_connect",
      waitMs: computeBackoffMs(reconnectAttempt, context.backoff, {
        jitter: "equal",
      }),
      mode,
    },
  };
};

const withQueuedReconnect = (
  context: WsLifecycleContext,
  mode: ReconnectMode,
  hasSocket: boolean
): Partial<WsLifecycleContext> => {
  if (hasSocket) {
    const reconnectMode: ReconnectMode =
      context.reconnectQueued &&
      context.reconnectMode === "backoff" &&
      mode === "immediate"
        ? "immediate"
        : context.reconnectQueued
          ? context.reconnectMode
          : mode;

    return {
      reconnectQueued: true,
      reconnectMode,
      closeForReconnect: true,
      effects: context.closeForReconnect
        ? context.effects
        : [...context.effects, { type: "close_socket" }],
    };
  }

  const { reconnectAttempt, effect } = nextReconnect(context, mode);
  return {
    reconnectQueued: false,
    reconnectMode: mode,
    reconnectAttempt,
    closeForReconnect: false,
    effects: [...context.effects, effect],
  };
};

const onSocketClosed = (
  context: WsLifecycleContext,
  mode: ReconnectMode,
  event: Extract<WsLifecycleEvent, { type: "SOCKET_CLOSE" }>
): Partial<WsLifecycleContext> => {
  const { reconnectAttempt, effect } = nextReconnect(context, mode);
  const effects = [...context.effects, effect];

  let lastError: WsLifecycleError | null = null;
  if (!context.closeForReconnect) {
    if (
      (event.reason && AUTH_CLOSE_REASONS.has(event.reason)) ||
      (event.code !== undefined && AUTH_CLOSE_CODES.has(event.code))
    ) {
      effects.push({ type: "handle_auth_close_policy" });
      lastError = {
        reason: "auth_close_policy",
        code: event.code,
        message: event.reason,
      };
    } else {
      lastError = {
        reason: "socket_close",
        code: event.code,
        message: event.reason,
      };
    }
  }

  return {
    reconnectAttempt,
    reconnectQueued: false,
    reconnectMode: mode,
    closeForReconnect: false,
    connectionAuth: "anonymous",
    lastError,
    effects,
  };
};

const onAuthSessionChanged = (
  context: WsLifecycleContext,
  event: Extract<WsLifecycleEvent, { type: "AUTH_SESSION_CHANGED" }>,
  inAwaitingAuth: boolean
): Partial<WsLifecycleContext> => {
  if (context.authMode === "anonymous") {
    return {};
  }

  if (context.authMode === "authenticated") {
    if (!event.token) {
      return {
        knownAuthToken: null,
        lastError: {
          reason: "auth_session_missing",
          message: "Authenticated websocket session is missing",
        },
        closeForReconnect: false,
        effects: event.hasSocket
          ? [...context.effects, { type: "close_socket" }]
          : context.effects,
      };
    }

    if (inAwaitingAuth || event.token !== context.knownAuthToken) {
      return {
        knownAuthToken: event.token,
        ...withQueuedReconnect(context, "immediate", event.hasSocket),
      };
    }
    return { knownAuthToken: event.token };
  }

  if (
    event.token &&
    (inAwaitingAuth || event.token !== context.knownAuthToken)
  ) {
    return {
      knownAuthToken: event.token,
      ...withQueuedReconnect(context, "immediate", event.hasSocket),
    };
  }

  if (!event.token && context.connectionAuth !== "anonymous") {
    return {
      knownAuthToken: null,
      ...withQueuedReconnect(context, "immediate", event.hasSocket),
    };
  }

  return { knownAuthToken: event.token };
};

const wsLifecycleMachine = setup({
  types: {
    context: {} as WsLifecycleContext,
    events: {} as WsLifecycleEvent,
    input: {} as {
      backoff: { baseMs: number; maxMs: number };
      authMode: WsAuthMode;
    },
  },
}).createMachine({
  id: "riseWsLifecycle",
  initial: "idle",
  context: ({ input }) => ({
    closeForReconnect: false,
    connectionAuth: "anonymous",
    pendingAuth: "anonymous",
    reconnectAttempt: 0,
    reconnectQueued: false,
    reconnectMode: "backoff",
    knownAuthToken: null,
    authMode: input.authMode,
    effects: [],
    lastError: null,
    backoff: input.backoff,
  }),
  states: {
    idle: {
      on: {
        CONNECT_START: {
          target: "connecting",
          actions: assign(({ event }) => ({
            pendingAuth: event.pendingAuth,
            closeForReconnect: false,
            lastError: null,
          })),
        },
        AWAIT_AUTH: { target: "awaitingAuth" },
        AUTH_SESSION_CHANGED: [
          {
            guard: ({ context, event }) =>
              context.authMode === "authenticated" && event.token === null,
            target: "awaitingAuth",
            actions: assign(({ context, event }) =>
              onAuthSessionChanged(context, event, false)
            ),
          },
          {
            actions: assign(({ context, event }) =>
              onAuthSessionChanged(context, event, false)
            ),
          },
        ],
        AUTH_TOKEN_OBSERVED: {
          actions: assign(({ event }) => ({ knownAuthToken: event.token })),
        },
        REQUEST_RECONNECT: {
          actions: assign(({ context, event }) =>
            withQueuedReconnect(context, event.mode, event.hasSocket)
          ),
        },
        CLOSE_CLIENT: { target: "closed" },
        CLEAR_EFFECTS: { actions: assign({ effects: [] }) },
      },
    },
    connecting: {
      on: {
        SOCKET_OPEN: {
          target: "connected",
          actions: assign(({ context }) => ({
            closeForReconnect: false,
            reconnectAttempt: 0,
            reconnectQueued: false,
            connectionAuth: context.pendingAuth,
            lastError: null,
          })),
        },
        AWAIT_AUTH: { target: "awaitingAuth" },
        SOCKET_CLOSE: {
          target: "idle",
          actions: assign(({ context, event }) => {
            const mode: ReconnectMode = context.closeForReconnect
              ? context.reconnectMode
              : "backoff";
            return onSocketClosed(context, mode, event);
          }),
        },
        SOCKET_ERROR: {
          target: "idle",
          actions: assign(({ context, event }) => ({
            ...onSocketClosed(context, "backoff", { type: "SOCKET_CLOSE" }),
            lastError: {
              reason: "socket_error",
              message: event.message,
            },
          })),
        },
        AUTH_SESSION_CHANGED: [
          {
            guard: ({ context, event }) =>
              context.authMode === "authenticated" && event.token === null,
            target: "awaitingAuth",
            actions: assign(({ context, event }) =>
              onAuthSessionChanged(context, event, false)
            ),
          },
          {
            actions: assign(({ context, event }) =>
              onAuthSessionChanged(context, event, false)
            ),
          },
        ],
        AUTH_TOKEN_OBSERVED: {
          actions: assign(({ event }) => ({ knownAuthToken: event.token })),
        },
        REQUEST_RECONNECT: {
          actions: assign(({ context, event }) =>
            withQueuedReconnect(context, event.mode, event.hasSocket)
          ),
        },
        CLOSE_CLIENT: { target: "closed" },
        CLEAR_EFFECTS: { actions: assign({ effects: [] }) },
      },
    },
    awaitingAuth: {
      on: {
        CONNECT_START: {
          target: "connecting",
          actions: assign(({ event }) => ({
            pendingAuth: event.pendingAuth,
            closeForReconnect: false,
            lastError: null,
          })),
        },
        AUTH_SESSION_CHANGED: [
          {
            guard: ({ context, event }) =>
              context.authMode === "authenticated" && event.token === null,
            actions: assign(({ context, event }) =>
              onAuthSessionChanged(context, event, true)
            ),
          },
          {
            actions: assign(({ context, event }) =>
              onAuthSessionChanged(context, event, true)
            ),
          },
        ],
        AUTH_TOKEN_OBSERVED: {
          actions: assign(({ event }) => ({ knownAuthToken: event.token })),
        },
        REQUEST_RECONNECT: {
          actions: assign(({ context, event }) =>
            withQueuedReconnect(context, event.mode, event.hasSocket)
          ),
        },
        SOCKET_CLOSE: {
          target: "idle",
          actions: assign(({ context, event }) => {
            const mode: ReconnectMode = context.closeForReconnect
              ? context.reconnectMode
              : "backoff";
            return onSocketClosed(context, mode, event);
          }),
        },
        SOCKET_ERROR: {
          target: "idle",
          actions: assign(({ context, event }) => ({
            ...onSocketClosed(context, "backoff", { type: "SOCKET_CLOSE" }),
            lastError: {
              reason: "socket_error",
              message: event.message,
            },
          })),
        },
        CLOSE_CLIENT: { target: "closed" },
        CLEAR_EFFECTS: { actions: assign({ effects: [] }) },
      },
    },
    connected: {
      on: {
        REQUEST_RECONNECT: {
          actions: assign(({ context, event }) =>
            withQueuedReconnect(context, event.mode, event.hasSocket)
          ),
        },
        SOCKET_CLOSE: {
          target: "idle",
          actions: assign(({ context, event }) => {
            const mode: ReconnectMode = context.closeForReconnect
              ? context.reconnectMode
              : "backoff";
            return onSocketClosed(context, mode, event);
          }),
        },
        SOCKET_ERROR: {
          target: "idle",
          actions: assign(({ context, event }) => ({
            ...onSocketClosed(context, "backoff", { type: "SOCKET_CLOSE" }),
            lastError: {
              reason: "socket_error",
              message: event.message,
            },
          })),
        },
        AUTH_SESSION_CHANGED: [
          {
            guard: ({ context, event }) =>
              context.authMode === "authenticated" && event.token === null,
            target: "awaitingAuth",
            actions: assign(({ context, event }) =>
              onAuthSessionChanged(context, event, false)
            ),
          },
          {
            actions: assign(({ context, event }) =>
              onAuthSessionChanged(context, event, false)
            ),
          },
        ],
        AUTH_TOKEN_OBSERVED: {
          actions: assign(({ event }) => ({ knownAuthToken: event.token })),
        },
        CLOSE_CLIENT: { target: "closed" },
        CLEAR_EFFECTS: { actions: assign({ effects: [] }) },
      },
    },
    closed: {
      type: "final",
    },
  },
});

export type RiseWsLifecycleState =
  | "idle"
  | "connecting"
  | "awaitingAuth"
  | "connected"
  | "closed";

export class AuthWsLifecycleController {
  private readonly actor: ReturnType<
    typeof createActor<typeof wsLifecycleMachine>
  >;

  constructor(
    backoff: { baseMs: number; maxMs: number },
    authMode: WsAuthMode
  ) {
    this.actor = createActor(wsLifecycleMachine, {
      input: { backoff, authMode },
    }).start();
  }

  send(event: WsLifecycleEvent): void {
    const prev = this.actor.getSnapshot().value as RiseWsLifecycleState;
    this.actor.send(event);
    const next = this.actor.getSnapshot().value as RiseWsLifecycleState;
    if (prev !== next) {
      debugWs("rise ws lifecycle transition", {
        event: event.type,
        from: prev,
        to: next,
        connectionAuth: this.connectionAuth,
        reconnectAttempt: this.reconnectAttempt,
      });
    }
  }

  drainEffects(): WsLifecycleEffect[] {
    const { effects } = this.actor.getSnapshot().context;
    if (effects.length === 0) return [];
    this.send({ type: "CLEAR_EFFECTS" });
    return effects;
  }

  get isConnected(): boolean {
    return this.actor.getSnapshot().matches("connected");
  }

  get state(): RiseWsLifecycleState {
    return this.actor.getSnapshot().value as RiseWsLifecycleState;
  }

  get isClosing(): boolean {
    return this.actor.getSnapshot().matches("closed");
  }

  get isAwaitingAuth(): boolean {
    return this.actor.getSnapshot().matches("awaitingAuth");
  }

  get closeForReconnect(): boolean {
    return this.actor.getSnapshot().context.closeForReconnect;
  }

  get connectionAuth(): WsConnectionAuth {
    return this.actor.getSnapshot().context.connectionAuth;
  }

  get reconnectAttempt(): number {
    return this.actor.getSnapshot().context.reconnectAttempt;
  }

  get knownAuthToken(): string | null {
    return this.actor.getSnapshot().context.knownAuthToken;
  }

  get lastError(): WsLifecycleError | null {
    return this.actor.getSnapshot().context.lastError;
  }

  stop(): void {
    this.actor.stop();
  }
}
