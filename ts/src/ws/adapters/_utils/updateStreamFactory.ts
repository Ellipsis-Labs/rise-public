import { createAsyncQueue } from "../../AsyncQueue";
import type { WsClient } from "@/ws/types";
import type { ZodSchema } from "zod";
import { createSubUnsubMessages } from "./createSubUnsubMessages";

let nextUpdateStreamListenerId = 0;

export interface UpdateStreamConfig<
  TRaw,
  TUpdate,
  TParams extends unknown[] = [],
> {
  channel: string;
  schema: ZodSchema<TRaw>;
  buildKey: (...params: TParams) => string;
  buildSubParams: (...params: TParams) => Record<string, unknown>;
  processMessage: (
    message: TRaw,
    params: TParams,
    context: ProcessMessageContext
  ) => TUpdate | TUpdate[] | null | Promise<TUpdate | TUpdate[] | null>;
  errorMode?: "console" | "system";
  schemaErrorMessage?: string;
}

export interface ProcessMessageContext {
  subscriptionKey: string;
  buffer: number;
}

export interface UpdateStreamOptions {
  buffer?: number;
}

export interface RefreshableAsyncIterable<T> extends AsyncIterable<T> {
  refresh(): void;
}

export function createUpdateStream<
  TRaw,
  TUpdate,
  TParams extends unknown[] = [],
>(
  ws: WsClient,
  config: UpdateStreamConfig<TRaw, TUpdate, TParams>,
  options?: UpdateStreamOptions
): (...args: [...TParams, AbortSignal?]) => RefreshableAsyncIterable<TUpdate> {
  return (
    ...args: [...TParams, AbortSignal?]
  ): RefreshableAsyncIterable<TUpdate> => {
    const lastArg = args[args.length - 1];
    const signal = lastArg instanceof AbortSignal ? lastArg : undefined;
    const params = (signal ? args.slice(0, -1) : args) as TParams;

    const { buffer = 2048 } = options ?? {};
    const { errorMode = "console" } = config;

    const queue = createAsyncQueue<TUpdate>(buffer, "drop-oldest");
    const routingKey = config.buildKey(...params);
    const listenerKey = `${routingKey}::listener:${nextUpdateStreamListenerId++}`;
    const subParams = config.buildSubParams(...params);

    const { sub, unsub } = createSubUnsubMessages(config.channel, subParams);

    const context: ProcessMessageContext = {
      subscriptionKey: routingKey,
      buffer,
    };

    const subscribe = () => {
      if (signal?.aborted) {
        return;
      }
      ws.subscribe(listenerKey, sub, onMessage, { routingKey });
    };

    const refresh = () => {
      if (signal?.aborted) {
        return;
      }
      // Re-subscribe in place so the server can treat this as a refresh of the
      // existing subscription instead of a fresh unsubscribe/subscribe cycle.
      ws.subscribe(listenerKey, sub, onMessage, { routingKey });
    };

    const onMessage = (raw: unknown) => {
      void (async () => {
        const parsed = config.schema.safeParse(raw);

        if (!parsed.success) {
          if (errorMode === "console") {
            const errorMsg =
              config.schemaErrorMessage ||
              `Failed to parse ${config.channel} message`;

            const serialize = (value: unknown) =>
              JSON.stringify(
                value,
                (_key: string, val: unknown) =>
                  typeof val === "bigint" ? val.toString() : val,
                2
              );

            console.error(
              `${errorMsg}:`,
              parsed.error,
              typeof raw === "string" ? raw : serialize(raw)
            );
          }
          return;
        }

        const result = await config.processMessage(
          parsed.data,
          params,
          context
        );

        if (result === null) {
          return;
        }

        if (Array.isArray(result)) {
          for (const update of result) {
            queue.push(update);
          }
        } else {
          queue.push(result);
        }
      })();
    };

    subscribe();

    signal?.addEventListener(
      "abort",
      () => {
        ws.unsubscribe(listenerKey, unsub);
        queue.close();
      },
      { once: true }
    );

    return {
      refresh,
      [Symbol.asyncIterator]: () => queue[Symbol.asyncIterator](),
    };
  };
}
