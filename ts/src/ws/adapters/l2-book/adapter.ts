import { createUpdateStream, normalizeTimestamp } from "@/ws/adapters/_utils";
import { handleError } from "@/ws/errorHandling/ErrorSystem";
import {
  createInvalidTimestampError,
  createWrongMarketError,
} from "@/ws/errorHandling/errors";
import type { WsClient } from "@/ws/types";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import type { L2BookPort, L2BookSubscriptionOptions } from "./ports";
import { L2BookMsgSchema, type L2BookMsg, type L2BookUpdate } from "./wire";

export type L2BookAdapter = L2BookPort;

export interface L2BookAdapterOptions {
  buffer?: number;
  timestampUnit?: "s" | "ms";
}

export const buildL2BookSubscriptionKey = (
  coin: string,
  bypassExecutionBand?: boolean
): string => {
  return bypassExecutionBand
    ? `l2Book:${coin}:bypassExecutionBand`
    : `l2Book:${coin}`;
};

export const createL2BookAdapter = (
  ws: WsClient,
  opts?: L2BookAdapterOptions,
  strictMode?: boolean
): L2BookAdapter => {
  const { timestampUnit = "s" } = opts ?? {};
  const schema = strictMode
    ? applyStrictModeRecursive(L2BookMsgSchema)
    : L2BookMsgSchema;
  const stream = createUpdateStream<
    L2BookMsg,
    L2BookUpdate,
    [market: string, options?: L2BookSubscriptionOptions]
  >(
    ws,
    {
      channel: "l2Book",
      schema,
      buildKey: (market: string, options?: L2BookSubscriptionOptions) =>
        buildL2BookSubscriptionKey(market, options?.bypassExecutionBand),
      buildSubParams: (
        market: string,
        options?: L2BookSubscriptionOptions
      ) => ({
        coin: market,
        ...(options?.bypassExecutionBand === undefined
          ? {}
          : { bypassExecutionBand: options.bypassExecutionBand }),
      }),
      processMessage: async (message, [market], context) => {
        if (message.coin !== market) {
          const error = createWrongMarketError(market, message.coin, {
            operation: "l2_book_validation",
            subscriptionKey: context.subscriptionKey,
          });
          await handleError(error);
          return null;
        }

        try {
          const ts = normalizeTimestamp(message.timestamp, timestampUnit);

          return {
            market: message.coin,
            ts,
            slot: BigInt(message.slot),
            bids: message.bids,
            asks: message.asks,
            bypassExecutionBand: message.bypassExecutionBand,
          };
        } catch {
          const error = createInvalidTimestampError(timestampUnit, {
            operation: "timestamp_normalization",
            subscriptionKey: context.subscriptionKey,
          });
          await handleError(error);
          return null;
        }
      },
      schemaErrorMessage: "Failed to parse L2Book message",
    },
    opts
  );

  const adapter: L2BookAdapter = (
    market: string,
    optionsOrSignal?: L2BookSubscriptionOptions | AbortSignal,
    signal?: AbortSignal
  ) => {
    const options =
      optionsOrSignal instanceof AbortSignal ? undefined : optionsOrSignal;
    const resolvedSignal =
      optionsOrSignal instanceof AbortSignal ? optionsOrSignal : signal;

    return stream(market, options, resolvedSignal);
  };

  return adapter;
};
