import { createUpdateStream } from "@/ws/adapters/_utils";
import { handleError } from "@/ws/errorHandling/ErrorSystem";
import { createWrongSymbolError } from "@/ws/errorHandling/errors";
import type { WsClient } from "@/ws/types";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import type { FundingRatePort } from "./ports";
import {
  FundingRateMsgSchema,
  type FundingRateMsg,
  type FundingRateUpdate,
} from "./wire";

export type FundingRateAdapter = FundingRatePort;

export interface FundingRateAdapterOptions {
  buffer?: number;
}

export const createFundingRateAdapter = (
  ws: WsClient,
  opts?: FundingRateAdapterOptions,
  strictMode?: boolean
): FundingRateAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(FundingRateMsgSchema)
    : FundingRateMsgSchema;

  return createUpdateStream<
    FundingRateMsg,
    FundingRateUpdate,
    [symbol: string]
  >(
    ws,
    {
      channel: "fundingRate",
      schema,
      buildKey: (symbol: string) => `fundingRate:${symbol}`,
      buildSubParams: (symbol: string) => ({ symbol }),
      processMessage: async (message, [symbol], context) => {
        if (message.symbol !== symbol) {
          const error = createWrongSymbolError(symbol, message.symbol, {
            operation: "funding_rate_validation",
            subscriptionKey: context.subscriptionKey,
          });
          await handleError(error);
          return null;
        }

        return {
          symbol: message.symbol,
          funding: message.funding,
        };
      },
      schemaErrorMessage: "Failed to parse FundingRate message",
    },
    opts
  );
};
