import type { L2BookUpdate } from "./wire";

export interface L2BookSubscriptionOptions {
  bypassExecutionBand?: boolean;
}

export type L2BookPort = {
  (market: string, signal?: AbortSignal): AsyncIterable<L2BookUpdate>;
  (
    market: string,
    options?: L2BookSubscriptionOptions,
    signal?: AbortSignal
  ): AsyncIterable<L2BookUpdate>;
};
