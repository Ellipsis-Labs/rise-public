import type { FillUpdate } from "./wire";

export type FillsPort = (
  symbol?: string,
  signal?: AbortSignal
) => AsyncIterable<FillUpdate>;
