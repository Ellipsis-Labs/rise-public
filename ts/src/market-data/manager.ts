import { createStore } from "zustand/vanilla";
import { createResourceRegistry } from "@/internal/_keyedResourceManager";
import {
  createLifecycle,
  normalizeBackgroundError,
  runRetryLoop,
} from "@/internal/_resourceRuntime";
import type { AllMidsUpdate } from "@/ws/adapters/all-mids";
import type { MarkPriceUpdate } from "@/ws/adapters/mark-price";
import type { MarketStatsUpdate } from "@/ws/adapters/market-stats";
import type {
  PhoenixMarketData,
  PhoenixMarketDataChange,
  PhoenixMarketDataChangedField,
  PhoenixMarketDataConfig,
  PhoenixMarketDataResource,
  PhoenixMarketDataResourceState,
  PhoenixMarketDataRow,
  PhoenixMarketDataStatus,
  PhoenixMarketDataStoreState,
} from "./types";

const DEFAULT_RESYNC_BACKOFF_MS = 1_000;
const MAX_RECENT_CHANGES = 40;

const EMPTY_SYMBOLS: readonly string[] = [];
const EMPTY_MARKETS_BY_SYMBOL: Readonly<Record<string, PhoenixMarketDataRow>> =
  {};
const EMPTY_RECENT_CHANGES: readonly PhoenixMarketDataChange[] = [];
const RESOURCE_KEY_PREFIX = "marketData:";

const normalizeSymbol = (symbol: string): string => symbol.trim().toUpperCase();

const sortSymbols = (symbols: Iterable<string>): readonly string[] =>
  Array.from(symbols).sort((left, right) => left.localeCompare(right));

const sameStringArray = (left: readonly string[], right: readonly string[]) =>
  left.length === right.length &&
  left.every((value, index) => value === right[index]);

const EMPTY_STATUS: PhoenixMarketDataStatus = {
  health: "cold",
  isConnected: false,
  isLoading: false,
  error: null,
};

const createEmptyRow = (symbol: string): PhoenixMarketDataRow => ({
  symbol,
  timestamp: null,
  mid: null,
  markPrice: null,
  oraclePrice: null,
  openInterest: null,
  currentFundingRate: null,
  eightHourFundingRate: null,
  annualizedFundingRate: null,
  prevDayMarkPrice: null,
  dayVolumeUsd: null,
  dayVolumeBase: null,
  priceChange24h: null,
  priceChange24hPercent: null,
  lastMidSlot: null,
  lastMidSlotIndex: null,
  lastMarkPriceSlot: null,
  lastMarkPriceSlotIndex: null,
  lastUpdatedMs: null,
});

const preserveRowReference = (
  previous: PhoenixMarketDataRow | undefined,
  next: PhoenixMarketDataRow
): PhoenixMarketDataRow => {
  if (!previous) {
    return next;
  }

  if (
    previous.symbol === next.symbol &&
    previous.timestamp === next.timestamp &&
    previous.mid === next.mid &&
    previous.markPrice === next.markPrice &&
    previous.oraclePrice === next.oraclePrice &&
    previous.openInterest === next.openInterest &&
    previous.currentFundingRate === next.currentFundingRate &&
    previous.eightHourFundingRate === next.eightHourFundingRate &&
    previous.annualizedFundingRate === next.annualizedFundingRate &&
    previous.prevDayMarkPrice === next.prevDayMarkPrice &&
    previous.dayVolumeUsd === next.dayVolumeUsd &&
    previous.dayVolumeBase === next.dayVolumeBase &&
    previous.priceChange24h === next.priceChange24h &&
    previous.priceChange24hPercent === next.priceChange24hPercent &&
    previous.lastMidSlot === next.lastMidSlot &&
    previous.lastMidSlotIndex === next.lastMidSlotIndex &&
    previous.lastMarkPriceSlot === next.lastMarkPriceSlot &&
    previous.lastMarkPriceSlotIndex === next.lastMarkPriceSlotIndex &&
    previous.lastUpdatedMs === next.lastUpdatedMs
  ) {
    return previous;
  }

  return next;
};

const createInitialState = (): PhoenixMarketDataStoreState => ({
  status: EMPTY_STATUS,
  symbols: EMPTY_SYMBOLS,
  marketsBySymbol: EMPTY_MARKETS_BY_SYMBOL,
  latestChange: null,
  recentChanges: EMPTY_RECENT_CHANGES,
});

const createResourceKey = (symbol: string): string =>
  `${RESOURCE_KEY_PREFIX}${normalizeSymbol(symbol)}`;

const createResourceState = (
  symbol: string,
  state: PhoenixMarketDataStoreState
): PhoenixMarketDataResourceState => ({
  key: createResourceKey(symbol),
  symbol,
  status: state.status,
  row: state.marketsBySymbol[symbol],
});

class PhoenixMarketDataResourceImpl implements PhoenixMarketDataResource {
  readonly key: string;
  readonly symbol: string;
  readonly store;

  private parentUnsubscribe: (() => void) | null = null;
  private parentRelease: (() => void) | null = null;
  private readonly lifecycle = createLifecycle({
    onDemandStart: () => {
      if (!this.parentRelease) {
        this.parentRelease = this.parent.retain();
      }
    },
    onDemandStop: () => {
      this.parentRelease?.();
      this.parentRelease = null;
    },
  });

  constructor(
    symbol: string,
    private readonly parent: PhoenixMarketData,
    private readonly disposeFromManager: () => void
  ) {
    this.symbol = normalizeSymbol(symbol);
    this.key = createResourceKey(this.symbol);
    this.store = createStore<PhoenixMarketDataResourceState>(() =>
      createResourceState(this.symbol, this.parent.snapshot())
    );
    this.parentUnsubscribe = this.parent.store.subscribe((state) => {
      const next = createResourceState(this.symbol, state);
      this.store.setState((previous) => {
        if (previous.status === next.status && previous.row === next.row) {
          return previous;
        }

        return next;
      });
    });
  }

  retain(): () => void {
    if (this.lifecycle.isClosed()) {
      throw new Error("Market data resource is closed");
    }

    return this.lifecycle.retain();
  }

  async ready(): Promise<PhoenixMarketDataRow | undefined> {
    this.lifecycle.pin();
    await this.parent.ready();
    return this.snapshot();
  }

  status(): PhoenixMarketDataStatus {
    return this.store.getState().status;
  }

  snapshot(): PhoenixMarketDataRow | undefined {
    return this.store.getState().row;
  }

  reconnect(): void {
    this.parent.reconnect();
  }

  subscribe(
    listener: (
      state: PhoenixMarketDataResourceState,
      previousState: PhoenixMarketDataResourceState
    ) => void
  ): () => void {
    return this.store.subscribe(listener);
  }

  close(): void {
    if (!this.lifecycle.close()) {
      return;
    }
    this.store.setState((state) => ({
      ...state,
      status: {
        health: "closed",
        isConnected: false,
        isLoading: false,
        error: null,
      },
    }));
    this.parentUnsubscribe?.();
    this.parentUnsubscribe = null;
    this.disposeFromManager();
  }
}

class PhoenixMarketDataImpl implements PhoenixMarketData {
  readonly store = createStore<PhoenixMarketDataStoreState>(createInitialState);
  private readonly resources =
    createResourceRegistry<PhoenixMarketDataResource>();

  private active = false;
  private startPromise: Promise<void> | null = null;
  private allMidsTask: Promise<void> | null = null;
  private allMidsAbort: AbortController | null = null;
  private marketStatsTask: Promise<void> | null = null;
  private marketStatsAbort: AbortController | null = null;
  private readonly markPriceTasks = new Map<string, Promise<void>>();
  private readonly markPriceAborts = new Map<string, AbortController>();
  private exchangeUnsubscribe: (() => void) | null = null;
  private exchangeSymbols: readonly string[] = EMPTY_SYMBOLS;
  private streamConnections = {
    allMids: false,
    marketStats: false,
    markPrices: new Set<string>(),
  };
  private readonly lifecycle = createLifecycle({
    onDemandStart: () => {
      this.ensureActive();
    },
    onDemandStop: () => {
      this.stopStreams();
    },
  });

  constructor(private readonly config: PhoenixMarketDataConfig) {}

  retain(): () => void {
    if (this.lifecycle.isClosed()) {
      throw new Error("Market data store is closed");
    }

    return this.lifecycle.retain();
  }

  async ready(): Promise<Readonly<PhoenixMarketDataStoreState>> {
    this.lifecycle.pin();
    await this.ensureStarted();
    return this.store.getState();
  }

  status(): PhoenixMarketDataStatus {
    return this.store.getState().status;
  }

  snapshot(): Readonly<PhoenixMarketDataStoreState> {
    return this.store.getState();
  }

  market(symbol: string): PhoenixMarketDataRow | undefined {
    return this.store.getState().marketsBySymbol[normalizeSymbol(symbol)];
  }

  resource(symbol: string): PhoenixMarketDataResource {
    if (this.lifecycle.isClosed()) {
      throw new Error("Market data store is closed");
    }
    const normalized = normalizeSymbol(symbol);
    if (!normalized) {
      throw new Error("Market data resource symbol is required");
    }
    const key = createResourceKey(normalized);
    return this.resources.getOrCreate(
      key,
      () =>
        new PhoenixMarketDataResourceImpl(normalized, this, () => {
          this.resources.delete(key);
        })
    );
  }

  reconnect(): void {
    if (this.lifecycle.isClosed()) {
      return;
    }

    this.stopStreams();
    this.ensureActive();
  }

  close(): void {
    if (!this.lifecycle.close()) {
      return;
    }
    for (const resource of Array.from(this.resources.values())) {
      resource.close();
    }
    this.exchangeUnsubscribe?.();
    this.exchangeUnsubscribe = null;
    this.setStatus({
      health: "closed",
      isConnected: false,
      isLoading: false,
      error: null,
    });
  }

  private async ensureStarted(): Promise<void> {
    if (this.startPromise) {
      return this.startPromise;
    }

    this.startPromise = (async () => {
      const exchange = this.config.exchange;
      if (!exchange || this.lifecycle.isClosed()) {
        return;
      }

      try {
        await exchange.ready();
      } catch (error) {
        this.handleBackgroundError(error);
      }

      if (this.lifecycle.isClosed()) {
        return;
      }

      const syncExchangeSymbols = () => {
        const symbols = exchange.store.getState().marketSymbols;
        this.exchangeSymbols = symbols;
        this.ensureRowsForSymbols(symbols);
        if (this.active) {
          this.syncMarkPriceSubscriptions();
        }
      };

      syncExchangeSymbols();

      this.exchangeUnsubscribe = exchange.store.subscribe((next, previous) => {
        if (next.marketSymbols !== previous.marketSymbols) {
          syncExchangeSymbols();
        }
      });
    })();

    return this.startPromise;
  }

  private ensureActive(): void {
    if (this.lifecycle.isClosed() || this.active) {
      return;
    }

    this.active = true;
    void this.ensureStarted();

    if (
      this.config.allMids ||
      this.config.marketStats ||
      this.config.markPrice
    ) {
      this.setStatus({
        ...this.store.getState().status,
        health: "recovering",
        isConnected: false,
        isLoading: true,
        error: null,
      });
    }

    this.startAllMidsLoop();
    this.startMarketStatsLoop();
    this.syncMarkPriceSubscriptions();
  }

  private stopStreams(): void {
    this.active = false;

    this.allMidsAbort?.abort();
    this.allMidsAbort = null;
    this.marketStatsAbort?.abort();
    this.marketStatsAbort = null;
    for (const controller of this.markPriceAborts.values()) {
      controller.abort();
    }
    this.markPriceAborts.clear();
    this.markPriceTasks.clear();
    this.allMidsTask = null;
    this.marketStatsTask = null;
    this.streamConnections = {
      allMids: false,
      marketStats: false,
      markPrices: new Set<string>(),
    };

    if (!this.lifecycle.isClosed()) {
      this.setStatus({
        ...this.store.getState().status,
        health: "cold",
        isConnected: false,
        isLoading: false,
        error: null,
      });
    }
  }

  private setStatus(next: PhoenixMarketDataStatus): void {
    this.store.setState((state) => {
      if (
        state.status.health === next.health &&
        state.status.isConnected === next.isConnected &&
        state.status.isLoading === next.isLoading &&
        state.status.error === next.error
      ) {
        return state;
      }

      return {
        ...state,
        status: next,
      };
    });
  }

  private updateConnectivity(error: Error | null = null): void {
    if (this.lifecycle.isClosed()) {
      return;
    }

    const isConnected =
      this.streamConnections.allMids ||
      this.streamConnections.marketStats ||
      this.streamConnections.markPrices.size > 0;
    const hasStreams =
      this.config.allMids !== undefined ||
      this.config.marketStats !== undefined ||
      this.config.markPrice !== undefined;

    this.setStatus({
      health:
        this.active && hasStreams
          ? isConnected
            ? "live"
            : "recovering"
          : "cold",
      isConnected,
      isLoading: this.active && hasStreams && !isConnected,
      error,
    });
  }

  private ensureRowsForSymbols(symbols: readonly string[]): void {
    const normalizedSymbols = sortSymbols(
      symbols.map((symbol) => normalizeSymbol(symbol)).filter(Boolean)
    );

    this.store.setState((state) => {
      const marketsBySymbol: Record<string, PhoenixMarketDataRow> = {
        ...state.marketsBySymbol,
      };
      let changed = false;

      for (const symbol of normalizedSymbols) {
        if (marketsBySymbol[symbol]) {
          continue;
        }
        marketsBySymbol[symbol] = createEmptyRow(symbol);
        changed = true;
      }

      const nextSymbols =
        normalizedSymbols.length > 0 ? normalizedSymbols : state.symbols;

      if (!changed && sameStringArray(state.symbols, nextSymbols)) {
        return state;
      }

      return {
        ...state,
        symbols: nextSymbols,
        marketsBySymbol: changed ? marketsBySymbol : state.marketsBySymbol,
      };
    });
  }

  private upsertRow(
    symbol: string,
    buildNext: (previous: PhoenixMarketDataRow) => {
      row: PhoenixMarketDataRow;
      change: PhoenixMarketDataChange | null;
    }
  ): void {
    const normalized = normalizeSymbol(symbol);
    if (!normalized) {
      return;
    }

    this.store.setState((state) => {
      const previous =
        state.marketsBySymbol[normalized] ?? createEmptyRow(normalized);
      const { row: nextRow, change } = buildNext(previous);
      const stableRow = preserveRowReference(previous, nextRow);
      const hadSymbol = state.marketsBySymbol[normalized] !== undefined;
      const symbols = hadSymbol
        ? state.symbols
        : sortSymbols([...state.symbols, normalized]);

      if (
        hadSymbol &&
        stableRow === previous &&
        change === null &&
        symbols === state.symbols
      ) {
        return state;
      }

      const marketsBySymbol =
        hadSymbol && stableRow === previous
          ? state.marketsBySymbol
          : {
              ...state.marketsBySymbol,
              [normalized]: stableRow,
            };

      const recentChanges =
        change === null
          ? state.recentChanges
          : [...state.recentChanges.slice(-(MAX_RECENT_CHANGES - 1)), change];

      return {
        ...state,
        symbols,
        marketsBySymbol,
        latestChange: change ?? state.latestChange,
        recentChanges,
      };
    });

    if (this.active && this.config.markPrice) {
      this.ensureMarkPriceLoop(normalized);
    }
  }

  private startAllMidsLoop(): void {
    if (!this.active || !this.config.allMids || this.allMidsTask) {
      return;
    }

    this.allMidsTask = (async () => {
      await runRetryLoop({
        shouldContinue: () => this.active && !this.lifecycle.isClosed(),
        backoffMs: this.config.resyncBackoffMs ?? DEFAULT_RESYNC_BACKOFF_MS,
        onController: (controller) => {
          this.allMidsAbort = controller;
        },
        stream: (signal) => this.config.allMids!(signal),
        onMessage: (update) => {
          this.streamConnections.allMids = true;
          this.updateConnectivity(null);
          this.applyAllMids(update);
        },
        onDisconnect: () => {
          this.streamConnections.allMids = false;
          this.updateConnectivity(null);
        },
        onError: (error) => {
          this.streamConnections.allMids = false;
          this.updateConnectivity(
            normalizeBackgroundError(error, "all mids stream failed")
          );
          this.handleBackgroundError(error);
        },
      });
    })().finally(() => {
      this.streamConnections.allMids = false;
      this.updateConnectivity(null);
      this.allMidsTask = null;
      this.allMidsAbort = null;
    });
  }

  private startMarketStatsLoop(): void {
    if (!this.active || !this.config.marketStats || this.marketStatsTask) {
      return;
    }

    this.marketStatsTask = (async () => {
      await runRetryLoop({
        shouldContinue: () => this.active && !this.lifecycle.isClosed(),
        backoffMs: this.config.resyncBackoffMs ?? DEFAULT_RESYNC_BACKOFF_MS,
        onController: (controller) => {
          this.marketStatsAbort = controller;
        },
        stream: (signal) => this.config.marketStats!(undefined, signal),
        onMessage: (update) => {
          this.streamConnections.marketStats = true;
          this.updateConnectivity(null);
          this.applyMarketStats(update);
        },
        onDisconnect: () => {
          this.streamConnections.marketStats = false;
          this.updateConnectivity(null);
        },
        onError: (error) => {
          this.streamConnections.marketStats = false;
          this.updateConnectivity(
            normalizeBackgroundError(error, "market stats stream failed")
          );
          this.handleBackgroundError(error);
        },
      });
    })().finally(() => {
      this.streamConnections.marketStats = false;
      this.updateConnectivity(null);
      this.marketStatsTask = null;
      this.marketStatsAbort = null;
    });
  }

  private syncMarkPriceSubscriptions(): void {
    if (!this.active || !this.config.markPrice) {
      return;
    }

    const symbols = this.store.getState().symbols;
    for (const symbol of symbols) {
      this.ensureMarkPriceLoop(symbol);
    }
  }

  private ensureMarkPriceLoop(symbol: string): void {
    const normalized = normalizeSymbol(symbol);
    if (
      !this.active ||
      !this.config.markPrice ||
      !normalized ||
      this.markPriceTasks.has(normalized)
    ) {
      return;
    }

    const task = (async () => {
      await runRetryLoop({
        shouldContinue: () => this.active && !this.lifecycle.isClosed(),
        backoffMs: this.config.resyncBackoffMs ?? DEFAULT_RESYNC_BACKOFF_MS,
        onController: (controller) => {
          if (controller) {
            this.markPriceAborts.set(normalized, controller);
          } else {
            this.markPriceAborts.delete(normalized);
          }
        },
        stream: (signal) => this.config.markPrice!(normalized, signal),
        onMessage: (update) => {
          this.streamConnections.markPrices.add(normalized);
          this.updateConnectivity(null);
          this.applyMarkPrice(update);
        },
        onDisconnect: () => {
          this.streamConnections.markPrices.delete(normalized);
          this.updateConnectivity(null);
        },
        onError: (error) => {
          this.streamConnections.markPrices.delete(normalized);
          this.updateConnectivity(
            normalizeBackgroundError(error, "mark price stream failed")
          );
          this.handleBackgroundError(error);
        },
      });
    })().finally(() => {
      this.streamConnections.markPrices.delete(normalized);
      this.updateConnectivity(null);
      this.markPriceTasks.delete(normalized);
      this.markPriceAborts.delete(normalized);
    });

    this.markPriceTasks.set(normalized, task);
  }

  private applyAllMids(update: AllMidsUpdate): void {
    const midsBySymbol = new Map<string, number>();
    for (const [symbol, mid] of Object.entries(update.mids)) {
      const normalized = normalizeSymbol(symbol);
      if (!normalized) {
        continue;
      }
      midsBySymbol.set(normalized, mid);
    }

    const symbols = sortSymbols(midsBySymbol.keys());
    if (symbols.length === 0) {
      return;
    }

    this.ensureRowsForSymbols(
      sortSymbols(new Set([...this.exchangeSymbols, ...symbols]))
    );

    const receivedAtMs = Date.now();
    const changedSymbols: string[] = [];

    this.store.setState((state) => {
      const marketsBySymbol: Record<string, PhoenixMarketDataRow> = {
        ...state.marketsBySymbol,
      };
      let changed = false;

      for (const symbol of symbols) {
        const previous = marketsBySymbol[symbol] ?? createEmptyRow(symbol);
        const next = preserveRowReference(previous, {
          ...previous,
          mid: midsBySymbol.get(symbol) ?? previous.mid,
          lastMidSlot: update.slot,
          lastMidSlotIndex: update.slotIndex,
          lastUpdatedMs: receivedAtMs,
        });
        if (next === previous) {
          continue;
        }

        marketsBySymbol[symbol] = next;
        changedSymbols.push(symbol);
        changed = true;
      }

      if (!changed) {
        return state;
      }

      const latestChange: PhoenixMarketDataChange = {
        receivedAtMs,
        source: "allMids",
        raw: update,
        symbol: changedSymbols.length === 1 ? changedSymbols[0] : null,
        slot: update.slot,
        slotIndex: update.slotIndex,
        changedFields: ["mid"],
        symbolCount: changedSymbols.length,
        markPrice: null,
        previousMarkPrice: null,
        markPriceDelta: null,
        oraclePrice: null,
        previousOraclePrice: null,
        oraclePriceDelta: null,
        openInterest: null,
        previousOpenInterest: null,
        openInterestDelta: null,
        currentFundingRate: null,
        previousFundingRate: null,
        fundingRateDelta: null,
        dayVolumeUsd: null,
        previousDayVolumeUsd: null,
        dayVolumeUsdDelta: null,
        mid:
          changedSymbols.length === 1
            ? (marketsBySymbol[changedSymbols[0]]?.mid ?? null)
            : null,
        previousMid:
          changedSymbols.length === 1
            ? (state.marketsBySymbol[changedSymbols[0]]?.mid ?? null)
            : null,
        midDelta:
          changedSymbols.length === 1
            ? (() => {
                const symbol = changedSymbols[0];
                const previous = state.marketsBySymbol[symbol]?.mid ?? null;
                const next = marketsBySymbol[symbol]?.mid ?? null;
                return previous === null || next === null
                  ? null
                  : next - previous;
              })()
            : null,
      };

      return {
        ...state,
        marketsBySymbol,
        latestChange,
        recentChanges: [
          ...state.recentChanges.slice(-(MAX_RECENT_CHANGES - 1)),
          latestChange,
        ],
      };
    });
  }

  private applyMarketStats(update: MarketStatsUpdate): void {
    this.upsertRow(update.symbol, (previous) => {
      const receivedAtMs = Date.now();
      const timestamp =
        typeof update.stats.timestamp === "bigint"
          ? Number(update.stats.timestamp)
          : update.stats.timestamp;
      const priceChange24h =
        update.stats.prevDayMarkPrice === 0
          ? 0
          : update.stats.markPrice - update.stats.prevDayMarkPrice;
      const priceChange24hPercent =
        update.stats.prevDayMarkPrice > 0
          ? (priceChange24h / update.stats.prevDayMarkPrice) * 100
          : 0;
      const next = {
        ...previous,
        timestamp,
        markPrice: update.stats.markPrice,
        oraclePrice: update.stats.oraclePrice,
        openInterest: update.stats.openInterest,
        currentFundingRate: update.stats.currentFundingRate,
        eightHourFundingRate: update.stats.eightHourFundingRate,
        annualizedFundingRate: update.stats.annualizedFundingRate,
        prevDayMarkPrice: update.stats.prevDayMarkPrice,
        dayVolumeUsd: update.stats.dayVolumeUsd,
        dayVolumeBase: update.stats.dayVolumeBase,
        priceChange24h,
        priceChange24hPercent,
        lastUpdatedMs: receivedAtMs,
      };
      const stable = preserveRowReference(previous, next);
      const changedFields: PhoenixMarketDataChangedField[] = [];

      if (stable.timestamp !== previous.timestamp)
        changedFields.push("timestamp");
      if (stable.markPrice !== previous.markPrice)
        changedFields.push("markPrice");
      if (stable.oraclePrice !== previous.oraclePrice)
        changedFields.push("oraclePrice");
      if (stable.openInterest !== previous.openInterest)
        changedFields.push("openInterest");
      if (stable.currentFundingRate !== previous.currentFundingRate) {
        changedFields.push("currentFundingRate");
      }
      if (stable.eightHourFundingRate !== previous.eightHourFundingRate) {
        changedFields.push("eightHourFundingRate");
      }
      if (stable.annualizedFundingRate !== previous.annualizedFundingRate) {
        changedFields.push("annualizedFundingRate");
      }
      if (stable.prevDayMarkPrice !== previous.prevDayMarkPrice) {
        changedFields.push("prevDayMarkPrice");
      }
      if (stable.dayVolumeUsd !== previous.dayVolumeUsd) {
        changedFields.push("dayVolumeUsd");
      }
      if (stable.dayVolumeBase !== previous.dayVolumeBase) {
        changedFields.push("dayVolumeBase");
      }
      if (stable.priceChange24h !== previous.priceChange24h) {
        changedFields.push("priceChange24h");
      }
      if (stable.priceChange24hPercent !== previous.priceChange24hPercent) {
        changedFields.push("priceChange24hPercent");
      }

      const change =
        changedFields.length === 0
          ? null
          : ({
              receivedAtMs,
              source: "marketStats",
              raw: update,
              symbol: stable.symbol,
              slot: null,
              slotIndex: null,
              changedFields,
              symbolCount: 1,
              markPrice: stable.markPrice,
              previousMarkPrice: previous.markPrice,
              markPriceDelta:
                previous.markPrice === null
                  ? null
                  : stable.markPrice === null
                    ? null
                    : stable.markPrice - previous.markPrice,
              oraclePrice: stable.oraclePrice,
              previousOraclePrice: previous.oraclePrice,
              oraclePriceDelta:
                previous.oraclePrice === null
                  ? null
                  : stable.oraclePrice === null
                    ? null
                    : stable.oraclePrice - previous.oraclePrice,
              openInterest: stable.openInterest,
              previousOpenInterest: previous.openInterest,
              openInterestDelta:
                previous.openInterest === null
                  ? null
                  : stable.openInterest === null
                    ? null
                    : stable.openInterest - previous.openInterest,
              currentFundingRate: stable.currentFundingRate,
              previousFundingRate: previous.currentFundingRate,
              fundingRateDelta:
                previous.currentFundingRate === null
                  ? null
                  : stable.currentFundingRate === null
                    ? null
                    : stable.currentFundingRate - previous.currentFundingRate,
              dayVolumeUsd: stable.dayVolumeUsd,
              previousDayVolumeUsd: previous.dayVolumeUsd,
              dayVolumeUsdDelta:
                previous.dayVolumeUsd === null
                  ? null
                  : stable.dayVolumeUsd === null
                    ? null
                    : stable.dayVolumeUsd - previous.dayVolumeUsd,
              mid: stable.mid,
              previousMid: previous.mid,
              midDelta:
                previous.mid === null
                  ? null
                  : stable.mid === null
                    ? null
                    : stable.mid - previous.mid,
            } satisfies PhoenixMarketDataChange);

      return {
        row: stable,
        change,
      };
    });
  }

  private applyMarkPrice(update: MarkPriceUpdate): void {
    this.upsertRow(update.symbol, (previous) => {
      const receivedAtMs = Date.now();
      const next = {
        ...previous,
        markPrice: update.markPrice,
        lastMarkPriceSlot: update.slot,
        lastMarkPriceSlotIndex: update.slotIndex,
        lastUpdatedMs: receivedAtMs,
      };
      const stable = preserveRowReference(previous, next);
      const changedFields: PhoenixMarketDataChangedField[] = [];
      if (stable.markPrice !== previous.markPrice) {
        changedFields.push("markPrice");
      }

      const change =
        changedFields.length === 0
          ? null
          : ({
              receivedAtMs,
              source: "markPrice",
              raw: update,
              symbol: stable.symbol,
              slot: update.slot,
              slotIndex: update.slotIndex,
              changedFields,
              symbolCount: 1,
              markPrice: stable.markPrice,
              previousMarkPrice: previous.markPrice,
              markPriceDelta:
                previous.markPrice === null
                  ? null
                  : stable.markPrice === null
                    ? null
                    : stable.markPrice - previous.markPrice,
              oraclePrice: stable.oraclePrice,
              previousOraclePrice: previous.oraclePrice,
              oraclePriceDelta: null,
              openInterest: stable.openInterest,
              previousOpenInterest: previous.openInterest,
              openInterestDelta: null,
              currentFundingRate: stable.currentFundingRate,
              previousFundingRate: previous.currentFundingRate,
              fundingRateDelta: null,
              dayVolumeUsd: stable.dayVolumeUsd,
              previousDayVolumeUsd: previous.dayVolumeUsd,
              dayVolumeUsdDelta: null,
              mid: stable.mid,
              previousMid: previous.mid,
              midDelta: null,
            } satisfies PhoenixMarketDataChange);

      return {
        row: stable,
        change,
      };
    });
  }

  private handleBackgroundError(error: unknown): void {
    this.config.onBackgroundError?.(error);
  }
}

export const createPhoenixMarketData = (
  config: PhoenixMarketDataConfig
): PhoenixMarketData => new PhoenixMarketDataImpl(config);
