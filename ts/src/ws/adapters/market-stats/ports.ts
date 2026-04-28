import type { MarketStatsUpdate } from "./wire";

export type MarketStatsPort = (
  symbol?: string,
  signal?: AbortSignal
) => AsyncIterable<MarketStatsUpdate>;
