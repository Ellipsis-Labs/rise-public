import { normalizeTimestamp } from "@/ws/adapters/_utils";
import { handleError } from "@/ws/errorHandling/ErrorSystem";
import { createInvalidTimestampError } from "@/ws/errorHandling/errors";

import type { MarketStatsWireData } from "./shared";
import type { MarketStatsUpdate } from "./wire";

interface NormalizedMarketStatsEntry<TWire extends MarketStatsWireData> {
  raw: TWire;
  update: MarketStatsUpdate;
}

const normalizeMarketStatsUpdate = (
  message: MarketStatsWireData
): MarketStatsUpdate | null => {
  let timestampMs: number;
  try {
    timestampMs = normalizeTimestamp(message.timestamp, "s");
  } catch {
    void handleError(
      createInvalidTimestampError("s", {
        operation: "timestamp_normalization",
      })
    );
    return null;
  }

  return {
    symbol: message.symbol,
    stats: {
      timestamp: BigInt(timestampMs),
      openInterest: message.openInterest,
      markPrice: message.markPrice,
      oraclePrice: message.oraclePrice,
      prevDayMarkPrice: message.prevDayMarkPrice,
      dayVolumeUsd: message.dayVolumeUsd,
      dayVolumeBase: message.dayVolumeBase,
      currentFundingRate: message.currentFundingRate,
      eightHourFundingRate: message.eightHourFundingRate,
      annualizedFundingRate: message.annualizedFundingRate,
    },
  };
};

export const normalizeMarketStatsEntries = <TWire extends MarketStatsWireData>(
  messages: readonly TWire[]
): NormalizedMarketStatsEntry<TWire>[] | null => {
  const entries: NormalizedMarketStatsEntry<TWire>[] = [];
  for (const raw of messages) {
    const update = normalizeMarketStatsUpdate(raw);
    if (!update) {
      return null;
    }
    entries.push({ raw, update });
  }
  return entries;
};
