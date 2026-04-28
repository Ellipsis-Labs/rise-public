import { createUpdateStream, normalizeTimestamp } from "@/ws/adapters/_utils";
import type { WsClient } from "@/ws/types";

import type { MarketStatsPort } from "./ports";
import {
  MarketStatsMsgSchema,
  type MarketStatsMsg,
  type MarketStatsUpdate,
} from "./wire";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import { handleError } from "@/ws/errorHandling/ErrorSystem";
import { createInvalidTimestampError } from "@/ws/errorHandling/errors";

export type MarketStatsAdapter = MarketStatsPort;

export interface MarketStatsAdapterOptions {
  buffer?: number;
}

export const createMarketStatsAdapter = (
  ws: WsClient,
  opts?: MarketStatsAdapterOptions,
  strictMode?: boolean
): MarketStatsAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(MarketStatsMsgSchema)
    : MarketStatsMsgSchema;
  return createUpdateStream<
    MarketStatsMsg,
    MarketStatsUpdate,
    [symbol?: string]
  >(
    ws,
    {
      channel: "marketStats",
      schema,
      buildKey: (symbol?: string) =>
        symbol ? `marketStats:${symbol}` : "marketStats",
      buildSubParams: (symbol?: string) => (symbol ? { symbol } : {}),
      processMessage: (m, [symbol]) => {
        if (symbol && m.symbol !== symbol) {
          return null;
        }

        let timestampMs: number;
        try {
          timestampMs = normalizeTimestamp(m.timestamp, "s");
        } catch {
          void handleError(
            createInvalidTimestampError("s", {
              operation: "timestamp_normalization",
            })
          );
          return null;
        }

        return {
          symbol: m.symbol,
          stats: {
            timestamp: BigInt(timestampMs),
            openInterest: m.openInterest,
            markPrice: m.markPrice,
            oraclePrice: m.oraclePrice,
            prevDayMarkPrice: m.prevDayMarkPrice,
            dayVolumeUsd: m.dayVolumeUsd,
            dayVolumeBase: m.dayVolumeBase,
            currentFundingRate: m.currentFundingRate,
            eightHourFundingRate: m.eightHourFundingRate,
            annualizedFundingRate: m.annualizedFundingRate,
          },
        };
      },
      schemaErrorMessage: "Failed to parse MarketStats message",
    },
    opts
  );
};
