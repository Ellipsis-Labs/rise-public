import type { WsClient } from "@/ws/types";
import { createUpdateStream, normalizeTimestamp } from "@/ws/adapters/_utils";
import { handleError } from "@/ws/errorHandling/ErrorSystem";
import {
  createInvalidTimestampError,
  createWrongSymbolError,
} from "@/ws/errorHandling/errors";
import type { CandlesPort } from "./ports";
import type { CandleMsg, CandleUpdate } from "./wire";
import { CandleMsgSchema } from "./wire";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";

export type CandlesAdapter = CandlesPort;

export interface CandlesAdapterOptions {
  buffer?: number;
  timestampUnit?: "s" | "ms";
}

export const createCandlesAdapter = (
  ws: WsClient,
  opts?: CandlesAdapterOptions,
  strictMode?: boolean
): CandlesAdapter => {
  const { timestampUnit = "s" } = opts ?? {};
  const schema = strictMode
    ? applyStrictModeRecursive(CandleMsgSchema)
    : CandleMsgSchema;

  return createUpdateStream<
    CandleMsg,
    CandleUpdate,
    [symbol: string, timeframe: string]
  >(
    ws,
    {
      channel: "candles",
      schema,
      buildKey: (symbol: string, timeframe: string) =>
        `candles:${symbol}:${timeframe}`,
      buildSubParams: (symbol: string, timeframe: string) => ({
        symbol,
        timeframe,
      }),
      processMessage: async (m, [symbol, timeframe], context) => {
        if (m.symbol !== symbol || m.timeframe !== timeframe) {
          const error = createWrongSymbolError(
            `${symbol}:${timeframe}`,
            `${m.symbol}:${m.timeframe}`,
            {
              operation: "candle_validation",
              subscriptionKey: context.subscriptionKey,
            }
          );
          await handleError(error);
          return null;
        }

        try {
          const ts = normalizeTimestamp(m.candle.time, timestampUnit);

          return {
            candle: {
              ...m.candle,
              time: ts,
            },
            symbol: m.symbol,
            timeframe: m.timeframe,
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
      schemaErrorMessage: "Failed to parse Candle message",
      errorMode: "console",
    },
    opts
  );
};
