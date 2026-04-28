import type { CandleUpdate } from "./wire";

export type CandlesPort = (
  symbol: string,
  timeframe: string,
  signal?: AbortSignal
) => AsyncIterable<CandleUpdate>;
