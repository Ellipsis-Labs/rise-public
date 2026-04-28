import type { FundingRateUpdate } from "./wire";

export type FundingRatePort = (
  symbol: string,
  signal?: AbortSignal
) => AsyncIterable<FundingRateUpdate>;
