import type { OrderbookSnapshotUpdate } from "./wire";

export interface OrderbookSubscriptionOptions {
  bypassExecutionBand?: boolean;
}

export type OrderbookPort = {
  (
    symbol: string,
    signal?: AbortSignal
  ): AsyncIterable<OrderbookSnapshotUpdate>;
  (
    symbol: string,
    options?: OrderbookSubscriptionOptions,
    signal?: AbortSignal
  ): AsyncIterable<OrderbookSnapshotUpdate>;
};
