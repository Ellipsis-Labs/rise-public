import type { MarkPriceUpdate } from "./wire";

export type MarkPricePort = (
  symbol: string,
  signal?: AbortSignal
) => AsyncIterable<MarkPriceUpdate>;
