import type { StoreApi } from "zustand/vanilla";
import type { L2BookPort } from "@/ws/adapters/l2-book";

export type PhoenixOrderbookHealth =
  | "cold"
  | "bootstrapped"
  | "live"
  | "recovering"
  | "closed";

export interface PhoenixOrderbookLevel {
  price: number;
  size: number;
}

export interface PhoenixOrderbookRequestOptions {
  bypassExecutionBand?: boolean;
}

export interface PhoenixOrderbookRequest extends PhoenixOrderbookRequestOptions {
  symbol: string;
}

export type PhoenixOrderbookResourceInput = string | PhoenixOrderbookRequest;

export type PhoenixOrderbookLevelTuple =
  | readonly [price: number, size: number]
  | readonly [price: number, size: number, depth: number];

export interface PhoenixOrderbookSnapshot {
  symbol: string;
  bypassExecutionBand: boolean;
  timestamp: number;
  timestampMs: number;
  slot: bigint | null;
  bids: readonly PhoenixOrderbookLevel[];
  asks: readonly PhoenixOrderbookLevel[];
  bestBid: PhoenixOrderbookLevel | null;
  bestAsk: PhoenixOrderbookLevel | null;
  mid: number;
  spread: number;
}

export interface PhoenixOrderbookStatus {
  health: PhoenixOrderbookHealth;
  isConnected: boolean;
  isLoading: boolean;
  error: Error | null;
}

export interface PhoenixOrderbookStoreState {
  key: string;
  symbol: string;
  bypassExecutionBand: boolean;
  status: PhoenixOrderbookStatus;
  book: PhoenixOrderbookSnapshot | null;
  bids: readonly PhoenixOrderbookLevel[];
  asks: readonly PhoenixOrderbookLevel[];
  bestBid: PhoenixOrderbookLevel | null;
  bestAsk: PhoenixOrderbookLevel | null;
  mid: number | null;
  spread: number | null;
  slot: bigint | null;
  timestamp: number | null;
  timestampMs: number | null;
  lastUpdatedMs: number | null;
}

export interface PhoenixOrderbookBootstrapSnapshot {
  symbol: string;
  slot?: number | bigint | null;
  timestamp?: number | null;
  bids: readonly PhoenixOrderbookLevelTuple[];
  asks: readonly PhoenixOrderbookLevelTuple[];
  mid?: number | null;
}

export interface PhoenixOrderbookSnapshotApi {
  getOrderbook(
    symbol: string,
    options?: PhoenixOrderbookRequestOptions
  ): Promise<PhoenixOrderbookBootstrapSnapshot>;
}

export interface PhoenixOrderbookManagerConfig {
  api?: PhoenixOrderbookSnapshotApi;
  l2Book?: L2BookPort;
  resyncBackoffMs?: number;
  onBackgroundError?: (error: unknown) => void;
}

export interface PhoenixOrderbookResource {
  readonly key: string;
  readonly symbol: string;
  readonly bypassExecutionBand: boolean;
  readonly store: StoreApi<PhoenixOrderbookStoreState>;
  retain(): () => void;
  ready(): Promise<PhoenixOrderbookSnapshot | null>;
  status(): PhoenixOrderbookStatus;
  snapshot(): PhoenixOrderbookSnapshot | null;
  refresh(): Promise<PhoenixOrderbookSnapshot | null>;
  reconnect(): void;
  subscribe(
    listener: (
      state: PhoenixOrderbookStoreState,
      previousState: PhoenixOrderbookStoreState
    ) => void
  ): () => void;
  close(): void;
}

export interface PhoenixOrderbookManager {
  resource(
    input: PhoenixOrderbookResourceInput,
    options?: PhoenixOrderbookRequestOptions
  ): PhoenixOrderbookResource;
  prefetch(
    input: PhoenixOrderbookResourceInput,
    options?: PhoenixOrderbookRequestOptions
  ): Promise<PhoenixOrderbookResource>;
  peek(
    input: PhoenixOrderbookResourceInput,
    options?: PhoenixOrderbookRequestOptions
  ): PhoenixOrderbookResource | undefined;
  clear(
    input: PhoenixOrderbookResourceInput,
    options?: PhoenixOrderbookRequestOptions
  ): void;
  close(): void;
}
