import type {
  ExchangeMarketSnapshot,
  ExchangeSnapshotView,
  ExchangeStateSnapshot,
  MarketPublicMetadata,
} from "@/api/exchange/types";
import type { StoreApi } from "zustand/vanilla";
import type {
  ExchangeDeltaMsg,
  ExchangeMsg,
  ExchangeSnapshotMsg,
} from "@/ws/adapters/exchange";
import type { ExchangePort } from "@/ws/adapters/exchange";
import type { PhoenixPdaClient } from "@/pdaClient";

export type ExchangeCacheHealth =
  | "cold"
  | "bootstrapped"
  | "live"
  | "recovering"
  | "closed";

export type ExchangeCacheHealthReason =
  | "bootstrap"
  | "rpcBootstrap"
  | "pollRefresh"
  | "ttlRefresh"
  | "apiUnavailable"
  | "rpcUnavailable"
  | "websocketSnapshot"
  | "sequenceGap"
  | "streamEnded"
  | "streamError"
  | "closed";

export type ExchangeCacheSnapshotSource = "http" | "websocket" | "rpc";

export type ExchangeMetadataPriority = "api" | "rpc";

export type ExchangeMetadataSource = "api" | "rpc" | "none";

export type ExchangeMarketLifecycle = "active" | "gated" | "closed";

export type ExchangeCacheExchangeChangeKind = "keys" | "status";

export type ExchangeCacheMarketChangeKind =
  | "status"
  | "closed"
  | "tombstoned"
  | "cancelRiskFactor"
  | "isolatedOnly"
  | "leverageTiers"
  | "markPriceParameters"
  | "openInterestCap"
  | "upnlRiskFactor"
  | "upnlRiskFactorForWithdrawals"
  | "fundingParameters"
  | "marketFees"
  | "commodityMetadata"
  | "metadata";

export type ExchangeCacheEvent =
  | {
      type: "healthChanged";
      previous: ExchangeCacheHealth;
      current: ExchangeCacheHealth;
      reason: ExchangeCacheHealthReason;
    }
  | {
      type: "sourceChanged";
      previous: ExchangeMetadataSource;
      current: ExchangeMetadataSource;
    }
  | {
      type: "snapshotApplied";
      source: ExchangeCacheSnapshotSource;
      slot: bigint;
      slotIndex: number;
      marketCount: number;
    }
  | {
      type: "exchangeUpdated";
      change: ExchangeCacheExchangeChangeKind;
      slot: bigint;
      slotIndex: number;
    }
  | {
      type: "marketAdded";
      symbol: string;
      slot: bigint;
      slotIndex: number;
    }
  | {
      type: "marketUpdated";
      symbol: string;
      change: ExchangeCacheMarketChangeKind;
      slot: bigint;
      slotIndex: number;
    }
  | {
      type: "marketRemoved";
      symbol: string;
      assetId: number;
      slot: bigint;
      slotIndex: number;
    };

export interface ExchangeInstructionContext {
  exchange: ExchangeStateSnapshot;
  market: ExchangeMarketSnapshot;
}

export interface ExchangeSnapshotApi {
  getSnapshot(): Promise<ExchangeSnapshotView>;
}

export interface PhoenixExchangeMetadataApiConfig {
  api: ExchangeSnapshotApi;
  exchange?: ExchangePort;
  enabled?: boolean;
  liveUpdates?: "ws" | "none";
}

export interface PhoenixExchangeMetadataRpcConfig {
  url?: string;
  enabled?: boolean;
  pollIntervalMs?: number;
  ttlMs?: number;
}

export interface PhoenixExchangeMetadataSourceState {
  active: ExchangeMetadataSource;
  priority: ExchangeMetadataPriority;
  fallbackAvailable: boolean;
  lastSyncAt?: number;
}

export interface ExchangeMarketStatusSummary {
  symbol: string;
  marketStatus: string;
  lifecycle: ExchangeMarketLifecycle;
  isActive: boolean;
  isGated: boolean;
  isClosed: boolean;
}

export interface ExchangeRelevantChange {
  receivedAtMs: number;
  scope: "exchange" | "market";
  symbol: string | null;
  eventType: string;
  detail: string;
  slot: bigint | null;
  slotIndex: number | null;
}

export interface PhoenixExchangeStoreState {
  health: ExchangeCacheHealth;
  source: PhoenixExchangeMetadataSourceState;
  snapshot: Readonly<ExchangeSnapshotView> | null;
  marketsBySymbol: Readonly<Record<string, ExchangeMarketSnapshot>>;
  marketsByAssetId: Readonly<Record<number, ExchangeMarketSnapshot>>;
  marketsByPubkey: Readonly<Record<string, ExchangeMarketSnapshot>>;
  marketMetadataBySymbol: Readonly<Record<string, MarketPublicMetadata | null>>;
  marketMetadataByAssetId: Readonly<
    Record<number, MarketPublicMetadata | null>
  >;
  marketMetadataByPubkey: Readonly<Record<string, MarketPublicMetadata | null>>;
  marketSymbols: readonly string[];
  activeMarketSymbols: readonly string[];
  gatedMarketSymbols: readonly string[];
  closedMarketSymbols: readonly string[];
  marketStatusBySymbol: Readonly<Record<string, ExchangeMarketStatusSummary>>;
  latestChange: ExchangeRelevantChange | null;
  recentChanges: readonly ExchangeRelevantChange[];
  latestChangeBySymbol: Readonly<Record<string, ExchangeRelevantChange>>;
  lastUpdatedMs: number | null;
}

export type PhoenixExchangeStore = StoreApi<PhoenixExchangeStoreState>;

export interface PhoenixExchangeMetadataConfig {
  priority?: ExchangeMetadataPriority;
  api?: PhoenixExchangeMetadataApiConfig;
  rpc?: PhoenixExchangeMetadataRpcConfig;
  pda?: PhoenixPdaClient;
  onBackgroundError?: (error: unknown) => void;
}

export interface PhoenixExchangeCacheConfig {
  api: ExchangeSnapshotApi;
  exchange: ExchangePort;
  resyncBackoffMs?: number;
  onBackgroundError?: (error: unknown) => void;
}

export interface PhoenixExchangeCacheStore {
  readonly store: PhoenixExchangeStore;
  snapshot(): Readonly<ExchangeSnapshotView>;
  market(symbol: string): ExchangeMarketSnapshot | undefined;
  marketByAssetId(assetId: number): ExchangeMarketSnapshot | undefined;
  marketByPubkey(pubkey: string): ExchangeMarketSnapshot | undefined;
  marketMetadata(symbol: string): MarketPublicMetadata | null | undefined;
  marketMetadataByAssetId(
    assetId: number
  ): MarketPublicMetadata | null | undefined;
  marketMetadataByPubkey(
    pubkey: string
  ): MarketPublicMetadata | null | undefined;
  instructionContext(symbol: string): ExchangeInstructionContext | undefined;
  onEvent(listener: (event: ExchangeCacheEvent) => void): () => void;
  applySnapshot(
    snapshot: ExchangeSnapshotView,
    source?: ExchangeCacheSnapshotSource
  ): void;
  applySnapshotMessage(message: ExchangeSnapshotMsg): void;
  applyDelta(message: ExchangeDeltaMsg): void;
  applyMessage(message: ExchangeMsg): void;
  setHealth(health: ExchangeCacheHealth): void;
  setSource(source: PhoenixExchangeMetadataSourceState): void;
}

export interface PhoenixExchangeCache {
  ready(): Promise<Readonly<ExchangeSnapshotView>>;
  health(): ExchangeCacheHealth;
  snapshot(): Readonly<ExchangeSnapshotView>;
  market(symbol: string): ExchangeMarketSnapshot | undefined;
  marketByAssetId(assetId: number): ExchangeMarketSnapshot | undefined;
  marketByPubkey(pubkey: string): ExchangeMarketSnapshot | undefined;
  marketMetadata(symbol: string): MarketPublicMetadata | null | undefined;
  marketMetadataByAssetId(
    assetId: number
  ): MarketPublicMetadata | null | undefined;
  marketMetadataByPubkey(
    pubkey: string
  ): MarketPublicMetadata | null | undefined;
  instructionContext(symbol: string): ExchangeInstructionContext | undefined;
  source(): PhoenixExchangeMetadataSourceState;
  subscribe(listener: (event: ExchangeCacheEvent) => void): () => void;
  onEvent(listener: (event: ExchangeCacheEvent) => void): () => void;
  close(): void;
}

export interface PhoenixExchangeMetadata extends PhoenixExchangeCache {
  readonly store: PhoenixExchangeStore;
}
