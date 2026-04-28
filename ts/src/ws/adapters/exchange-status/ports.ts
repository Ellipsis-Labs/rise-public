import type { ExchangeStatusUpdate } from "./wire";

export type ExchangeStatusPort = (
  signal?: AbortSignal
) => AsyncIterable<ExchangeStatusUpdate>;
