import { createUpdateStream } from "@/ws/adapters/_utils";
import { handleError } from "@/ws/errorHandling/ErrorSystem";
import { createWrongSymbolError } from "@/ws/errorHandling/errors";
import type { WsClient } from "@/ws/types";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import type { MarketPort } from "./ports";
import { MarketMsgSchema, type MarketMsg, type MarketUpdate } from "./wire";

export type MarketAdapter = MarketPort;

export interface MarketAdapterOptions {
  buffer?: number;
}

export const createMarketAdapter = (
  ws: WsClient,
  opts?: MarketAdapterOptions,
  strictMode?: boolean
): MarketAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(MarketMsgSchema)
    : MarketMsgSchema;

  return createUpdateStream<MarketMsg, MarketUpdate, [symbol: string]>(
    ws,
    {
      channel: "market",
      schema,
      buildKey: (symbol: string) => `market:${symbol}`,
      buildSubParams: (symbol: string) => ({ symbol }),
      processMessage: async (message, [symbol], context) => {
        if (message.symbol !== symbol) {
          const error = createWrongSymbolError(symbol, message.symbol, {
            operation: "market_validation",
            subscriptionKey: context.subscriptionKey,
          });
          await handleError(error);
          return null;
        }

        return {
          symbol: message.symbol,
          dayNtlVlm: message.dayNtlVlm,
          prevDayPx: message.prevDayPx,
          markPx: message.markPx,
          midPx: message.midPx,
          funding: message.funding,
          openInterest: message.openInterest,
          oraclePx: message.oraclePx,
        };
      },
      schemaErrorMessage: "Failed to parse Market message",
    },
    opts
  );
};
