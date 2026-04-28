import type { StoreApi } from "zustand/vanilla";
import type { PhoenixExchangeMetadata } from "@/exchange-cache";
import type { AllMidsUpdate } from "@/ws/adapters/all-mids";
import type { MarkPriceUpdate } from "@/ws/adapters/mark-price";
import type { MarketStatsUpdate } from "@/ws/adapters/market-stats";
import type { AllMidsPort } from "@/ws/adapters/all-mids";
import type { MarketStatsPort } from "@/ws/adapters/market-stats";
import type { MarkPricePort } from "@/ws/adapters/mark-price";

export type PhoenixMarketDataHealth = "cold" | "live" | "recovering" | "closed";

export interface PhoenixMarketDataStatus {
  health: PhoenixMarketDataHealth;
  isConnected: boolean;
  isLoading: boolean;
  error: Error | null;
}

export interface PhoenixMarketDataRow {
  symbol: string;
  timestamp: number | null;
  mid: number | null;
  markPrice: number | null;
  oraclePrice: number | null;
  openInterest: number | null;
  currentFundingRate: number | null;
  eightHourFundingRate: number | null;
  annualizedFundingRate: number | null;
  prevDayMarkPrice: number | null;
  dayVolumeUsd: number | null;
  dayVolumeBase: number | null;
  priceChange24h: number | null;
  priceChange24hPercent: number | null;
  lastMidSlot: bigint | null;
  lastMidSlotIndex: number | null;
  lastMarkPriceSlot: bigint | null;
  lastMarkPriceSlotIndex: number | null;
  lastUpdatedMs: number | null;
}

export type PhoenixMarketDataChangedField =
  | "mid"
  | "markPrice"
  | "oraclePrice"
  | "openInterest"
  | "currentFundingRate"
  | "eightHourFundingRate"
  | "annualizedFundingRate"
  | "prevDayMarkPrice"
  | "dayVolumeUsd"
  | "dayVolumeBase"
  | "priceChange24h"
  | "priceChange24hPercent"
  | "timestamp";

export interface PhoenixMarketDataChange {
  receivedAtMs: number;
  source: "allMids" | "marketStats" | "markPrice";
  raw: AllMidsUpdate | MarketStatsUpdate | MarkPriceUpdate;
  symbol: string | null;
  slot: bigint | null;
  slotIndex: number | null;
  changedFields: readonly PhoenixMarketDataChangedField[];
  symbolCount: number;
  markPrice: number | null;
  previousMarkPrice: number | null;
  markPriceDelta: number | null;
  oraclePrice: number | null;
  previousOraclePrice: number | null;
  oraclePriceDelta: number | null;
  openInterest: number | null;
  previousOpenInterest: number | null;
  openInterestDelta: number | null;
  currentFundingRate: number | null;
  previousFundingRate: number | null;
  fundingRateDelta: number | null;
  dayVolumeUsd: number | null;
  previousDayVolumeUsd: number | null;
  dayVolumeUsdDelta: number | null;
  mid: number | null;
  previousMid: number | null;
  midDelta: number | null;
}

export interface PhoenixMarketDataStoreState {
  status: PhoenixMarketDataStatus;
  symbols: readonly string[];
  marketsBySymbol: Readonly<Record<string, PhoenixMarketDataRow>>;
  latestChange: PhoenixMarketDataChange | null;
  recentChanges: readonly PhoenixMarketDataChange[];
}

export type PhoenixMarketDataStore = StoreApi<PhoenixMarketDataStoreState>;

export interface PhoenixMarketDataResourceState {
  key: string;
  symbol: string;
  status: PhoenixMarketDataStatus;
  row: PhoenixMarketDataRow | undefined;
}

export type PhoenixMarketDataResourceStore =
  StoreApi<PhoenixMarketDataResourceState>;

export interface PhoenixMarketDataConfig {
  exchange?: PhoenixExchangeMetadata;
  allMids?: AllMidsPort;
  marketStats?: MarketStatsPort;
  markPrice?: MarkPricePort;
  resyncBackoffMs?: number;
  onBackgroundError?: (error: unknown) => void;
}

export interface PhoenixMarketData {
  readonly store: PhoenixMarketDataStore;
  retain(): () => void;
  ready(): Promise<Readonly<PhoenixMarketDataStoreState>>;
  status(): PhoenixMarketDataStatus;
  snapshot(): Readonly<PhoenixMarketDataStoreState>;
  market(symbol: string): PhoenixMarketDataRow | undefined;
  resource(symbol: string): PhoenixMarketDataResource;
  reconnect(): void;
  close(): void;
}

export interface PhoenixMarketDataResource {
  readonly key: string;
  readonly symbol: string;
  readonly store: PhoenixMarketDataResourceStore;
  retain(): () => void;
  ready(): Promise<PhoenixMarketDataRow | undefined>;
  status(): PhoenixMarketDataStatus;
  snapshot(): PhoenixMarketDataRow | undefined;
  reconnect(): void;
  subscribe(
    listener: (
      state: PhoenixMarketDataResourceState,
      previousState: PhoenixMarketDataResourceState
    ) => void
  ): () => void;
  close(): void;
}
