import type { MarketUpdate } from "./wire";

export type MarketPort = (
  symbol: string,
  signal?: AbortSignal
) => AsyncIterable<MarketUpdate>;
