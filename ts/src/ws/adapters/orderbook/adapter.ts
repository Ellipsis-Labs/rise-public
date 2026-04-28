import { createUpdateStream } from "@/ws/adapters/_utils";
import { handleError } from "@/ws/errorHandling/ErrorSystem";
import { createWrongSymbolError } from "@/ws/errorHandling/errors";
import type { WsClient } from "@/ws/types";
import type { OrderbookPort, OrderbookSubscriptionOptions } from "./ports";
import {
  OrderbookMsgSchema,
  type OrderbookMsg,
  type OrderbookSnapshotUpdate,
} from "./wire";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";

export type OrderbookAdapter = OrderbookPort;

export interface OrderbookAdapterOptions {
  buffer?: number;
}

export const buildOrderbookSubscriptionKey = (
  symbol: string,
  bypassExecutionBand?: boolean
): string => {
  return bypassExecutionBand
    ? `orderbook:${symbol}:bypassExecutionBand`
    : `orderbook:${symbol}`;
};

export const createOrderbookAdapter = (
  ws: WsClient,
  opts?: OrderbookAdapterOptions,
  strictMode?: boolean
): OrderbookAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(OrderbookMsgSchema)
    : OrderbookMsgSchema;
  const stream = createUpdateStream<
    OrderbookMsg,
    OrderbookSnapshotUpdate,
    [symbol: string, options?: OrderbookSubscriptionOptions]
  >(
    ws,
    {
      channel: "orderbook",
      schema,
      buildKey: (symbol: string, options?: OrderbookSubscriptionOptions) =>
        buildOrderbookSubscriptionKey(symbol, options?.bypassExecutionBand),
      buildSubParams: (
        symbol: string,
        options?: OrderbookSubscriptionOptions
      ) => ({
        symbol,
        ...(options?.bypassExecutionBand === undefined
          ? {}
          : { bypassExecutionBand: options.bypassExecutionBand }),
      }),
      processMessage: async (message, [symbol], context) => {
        if (message.symbol !== symbol) {
          const error = createWrongSymbolError(symbol, message.symbol, {
            operation: "orderbook_snapshot_validation",
            subscriptionKey: context.subscriptionKey,
          });
          await handleError(error);
          return null;
        }

        return {
          symbol: message.symbol,
          orderbook: message.orderbook,
          bypassExecutionBand: message.bypassExecutionBand,
        };
      },
      schemaErrorMessage: "Failed to parse Orderbook message",
    },
    opts
  );

  const adapter: OrderbookAdapter = (
    symbol: string,
    optionsOrSignal?: OrderbookSubscriptionOptions | AbortSignal,
    signal?: AbortSignal
  ) => {
    const options =
      optionsOrSignal instanceof AbortSignal ? undefined : optionsOrSignal;
    const resolvedSignal =
      optionsOrSignal instanceof AbortSignal ? optionsOrSignal : signal;

    return stream(symbol, options, resolvedSignal);
  };

  return adapter;
};
