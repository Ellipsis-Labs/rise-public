import { assign, createActor, setup } from "xstate";
import { computeBackoffMs } from "./backoff";
import { debugWs } from "./debug";

export type WsCoreEffect =
  | { type: "close_socket" }
  | { type: "schedule_connect"; waitMs: number };
export type WsCoreErrorReason = "socket_error" | "socket_close";
export type WsCoreError = {
  reason: WsCoreErrorReason;
  message?: string;
};

type ReconnectMode = "backoff" | "immediate";

type WsCoreContext = {
  closeForReconnect: boolean;
  reconnectAttempt: number;
  reconnectQueued: boolean;
  reconnectMode: ReconnectMode;
  lastError: WsCoreError | null;
  effects: WsCoreEffect[];
  backoff: { baseMs: number; maxMs: number };
};

type WsCoreEvent =
  | { type: "CONNECT_START" }
  | { type: "SOCKET_OPEN" }
  | { type: "SOCKET_CLOSE"; message?: string }
  | { type: "SOCKET_ERROR"; message?: string }
  | { type: "REQUEST_RECONNECT"; hasSocket: boolean }
  | { type: "CLOSE_CLIENT" }
  | { type: "CLEAR_EFFECTS" };

const queueConnect = (
  context: WsCoreContext,
  mode: ReconnectMode
): Partial<WsCoreContext> => {
  if (mode === "immediate") {
    return {
      reconnectAttempt: 0,
      reconnectQueued: false,
      reconnectMode: mode,
      closeForReconnect: false,
      effects: [...context.effects, { type: "schedule_connect", waitMs: 0 }],
    };
  }

  const reconnectAttempt = context.reconnectAttempt + 1;
  return {
    reconnectAttempt,
    reconnectQueued: false,
    reconnectMode: mode,
    closeForReconnect: false,
    effects: [
      ...context.effects,
      {
        type: "schedule_connect",
        waitMs: computeBackoffMs(reconnectAttempt, context.backoff, {
          jitter: "equal",
        }),
      },
    ],
  };
};

const wsCoreMachine = setup({
  types: {
    context: {} as WsCoreContext,
    events: {} as WsCoreEvent,
    input: {} as { backoff: { baseMs: number; maxMs: number } },
  },
}).createMachine({
  id: "wsCoreLifecycle",
  initial: "idle",
  context: ({ input }) => ({
    closeForReconnect: false,
    reconnectAttempt: 0,
    reconnectQueued: false,
    reconnectMode: "backoff",
    lastError: null,
    effects: [],
    backoff: input.backoff,
  }),
  states: {
    idle: {
      on: {
        CONNECT_START: {
          target: "connecting",
          actions: assign({ lastError: null }),
        },
        REQUEST_RECONNECT: {
          actions: assign(({ context, event }) => {
            if (event.hasSocket) {
              return {
                reconnectQueued: true,
                reconnectMode: "immediate",
                closeForReconnect: true,
                effects: context.closeForReconnect
                  ? context.effects
                  : [...context.effects, { type: "close_socket" }],
              };
            }
            return queueConnect(context, "backoff");
          }),
        },
        CLOSE_CLIENT: { target: "closed" },
        CLEAR_EFFECTS: { actions: assign({ effects: [] }) },
      },
    },
    connecting: {
      on: {
        SOCKET_OPEN: {
          target: "connected",
          actions: assign({
            reconnectAttempt: 0,
            reconnectQueued: false,
            reconnectMode: "backoff",
            closeForReconnect: false,
            lastError: null,
          }),
        },
        SOCKET_CLOSE: {
          target: "idle",
          actions: assign(({ context, event }) => ({
            ...queueConnect(
              context,
              context.closeForReconnect ? context.reconnectMode : "backoff"
            ),
            lastError: context.closeForReconnect
              ? null
              : { reason: "socket_close", message: event.message },
          })),
        },
        SOCKET_ERROR: {
          target: "idle",
          actions: assign(({ context, event }) => ({
            ...queueConnect(context, "backoff"),
            lastError: { reason: "socket_error", message: event.message },
          })),
        },
        REQUEST_RECONNECT: {
          actions: assign(({ context, event }) => {
            if (!event.hasSocket) {
              return queueConnect(context, "backoff");
            }
            return {
              reconnectQueued: true,
              reconnectMode: "immediate",
              closeForReconnect: true,
              effects: context.closeForReconnect
                ? context.effects
                : [...context.effects, { type: "close_socket" }],
            };
          }),
        },
        CLOSE_CLIENT: { target: "closed" },
        CLEAR_EFFECTS: { actions: assign({ effects: [] }) },
      },
    },
    connected: {
      on: {
        REQUEST_RECONNECT: {
          actions: assign(({ context, event }) => {
            if (!event.hasSocket) {
              return queueConnect(context, "backoff");
            }
            return {
              reconnectQueued: true,
              reconnectMode: "immediate",
              closeForReconnect: true,
              effects: context.closeForReconnect
                ? context.effects
                : [...context.effects, { type: "close_socket" }],
            };
          }),
        },
        SOCKET_CLOSE: {
          target: "idle",
          actions: assign(({ context, event }) => ({
            ...queueConnect(
              context,
              context.closeForReconnect ? context.reconnectMode : "backoff"
            ),
            lastError: context.closeForReconnect
              ? null
              : { reason: "socket_close", message: event.message },
          })),
        },
        SOCKET_ERROR: {
          target: "idle",
          actions: assign(({ context, event }) => ({
            ...queueConnect(context, "backoff"),
            lastError: { reason: "socket_error", message: event.message },
          })),
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

export class WsCoreLifecycleController {
  private actor: ReturnType<typeof createActor<typeof wsCoreMachine>>;

  constructor(backoff: { baseMs: number; maxMs: number }) {
    this.actor = createActor(wsCoreMachine, { input: { backoff } }).start();
  }

  send(event: WsCoreEvent): void {
    const prev = this.actor.getSnapshot().value;
    this.actor.send(event);
    const next = this.actor.getSnapshot().value;
    if (prev !== next) {
      debugWs("ws core lifecycle transition", {
        event: event.type,
        from: prev,
        to: next,
        reconnectAttempt: this.reconnectAttempt,
      });
    }
  }

  drainEffects(): WsCoreEffect[] {
    const { effects } = this.actor.getSnapshot().context;
    if (effects.length === 0) return [];
    this.send({ type: "CLEAR_EFFECTS" });
    return effects;
  }

  get isConnected(): boolean {
    return this.actor.getSnapshot().matches("connected");
  }

  get isClosing(): boolean {
    return this.actor.getSnapshot().matches("closed");
  }

  get reconnectAttempt(): number {
    return this.actor.getSnapshot().context.reconnectAttempt;
  }

  get lastError(): WsCoreError | null {
    return this.actor.getSnapshot().context.lastError;
  }

  stop(): void {
    this.actor.stop();
  }
}
