import { createStore } from "zustand/vanilla";
import type {
  ExchangeMarketSnapshot,
  ExchangeSnapshotView,
  ExchangeStateSnapshot,
  ExchangeWsCommodityMetadata,
  ExchangeWsLeverageTier,
  ExchangeWsMarketPriceBand,
  ExchangeWsMarkPriceParameters,
  MarketPublicMetadata,
} from "@/api/exchange/types";
import type {
  ExchangeDeltaMsg,
  ExchangeMarketParameterUpdate,
  ExchangeMsg,
  ExchangeSnapshotMsg,
} from "@/ws/adapters/exchange";
import type {
  ExchangeCacheEvent,
  ExchangeCacheHealth,
  ExchangeCacheMarketChangeKind,
  ExchangeCacheSnapshotSource,
  ExchangeInstructionContext,
  ExchangeMarketLifecycle,
  ExchangeMarketStatusSummary,
  ExchangeRelevantChange,
  PhoenixExchangeCacheStore,
  PhoenixExchangeMetadataSourceState,
  PhoenixExchangeStoreState,
} from "./types";

const MARKET_STATUS_ACTIVE = "active";
const MARKET_STATUS_CLOSED = "closed";
const MARKET_STATUS_TOMBSTONED = "tombstoned";
const MAX_RECENT_CHANGES = 40;

type IndexedSnapshot = {
  snapshot: Readonly<ExchangeSnapshotView>;
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
};

const EMPTY_MARKETS_BY_SYMBOL: Readonly<
  Record<string, ExchangeMarketSnapshot>
> = {};
const EMPTY_MARKETS_BY_ASSET_ID: Readonly<
  Record<number, ExchangeMarketSnapshot>
> = {};
const EMPTY_MARKETS_BY_PUBKEY: Readonly<
  Record<string, ExchangeMarketSnapshot>
> = {};
const EMPTY_MARKET_METADATA_BY_SYMBOL: Readonly<
  Record<string, MarketPublicMetadata | null>
> = {};
const EMPTY_MARKET_METADATA_BY_ASSET_ID: Readonly<
  Record<number, MarketPublicMetadata | null>
> = {};
const EMPTY_MARKET_METADATA_BY_PUBKEY: Readonly<
  Record<string, MarketPublicMetadata | null>
> = {};
const EMPTY_MARKET_SYMBOLS: readonly string[] = [];
const EMPTY_MARKET_STATUS_BY_SYMBOL: Readonly<
  Record<string, ExchangeMarketStatusSummary>
> = {};
const EMPTY_RECENT_CHANGES: readonly ExchangeRelevantChange[] = [];
const EMPTY_LATEST_CHANGE_BY_SYMBOL: Readonly<
  Record<string, ExchangeRelevantChange>
> = {};

const DEFAULT_SOURCE_STATE: PhoenixExchangeMetadataSourceState = {
  active: "none",
  priority: "api",
  fallbackAvailable: false,
};

const normalizeSymbol = (symbol: string): string => symbol.toUpperCase();

const cloneBand = (
  band: ExchangeWsMarketPriceBand | null | undefined
): ExchangeWsMarketPriceBand | null | undefined =>
  band
    ? {
        lower: { ...band.lower },
        upper: { ...band.upper },
      }
    : band;

const cloneCommodityMetadata = (
  metadata: ExchangeWsCommodityMetadata | null | undefined
): ExchangeWsCommodityMetadata | null | undefined =>
  metadata
    ? {
        ...metadata,
        afterHoursRadius: { ...metadata.afterHoursRadius },
        lastKnownIndexPrice: metadata.lastKnownIndexPrice
          ? { ...metadata.lastKnownIndexPrice }
          : metadata.lastKnownIndexPrice,
        markPriceBand: cloneBand(metadata.markPriceBand),
        executionPriceBand: cloneBand(metadata.executionPriceBand),
      }
    : metadata;

const cloneLeverageTier = (
  tier: ExchangeWsLeverageTier
): ExchangeWsLeverageTier => ({
  ...tier,
});

const cloneMarkPriceParameters = (
  params: ExchangeWsMarkPriceParameters
): ExchangeWsMarkPriceParameters => ({
  ...params,
});

const cloneMarket = (
  market: ExchangeMarketSnapshot
): ExchangeMarketSnapshot => ({
  ...market,
  leverageTiers: market.leverageTiers.map(cloneLeverageTier),
  riskFactors: { ...market.riskFactors },
  fundingConfig: { ...market.fundingConfig },
  markPriceParameters: cloneMarkPriceParameters(market.markPriceParameters),
  commodityMetadata: cloneCommodityMetadata(market.commodityMetadata),
  metadata: market.metadata ? { ...market.metadata } : market.metadata,
});

const cloneExchange = (
  exchange: ExchangeStateSnapshot
): ExchangeStateSnapshot => ({
  ...exchange,
  currentAuthorities: { ...exchange.currentAuthorities },
  globalTraderIndex: [...exchange.globalTraderIndex],
  activeTraderBuffer: [...exchange.activeTraderBuffer],
  exchangeStatusFeatures: [...exchange.exchangeStatusFeatures],
});

const sortMarkets = (markets: ExchangeMarketSnapshot[]): void => {
  markets.sort(
    (left, right) =>
      left.assetId - right.assetId || left.symbol.localeCompare(right.symbol)
  );
};

const deepFreeze = <T>(value: T): Readonly<T> => {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value as Record<string, unknown>)) {
      deepFreeze(child);
    }
  }
  return value as Readonly<T>;
};

const normalizeSnapshot = (
  snapshot: ExchangeSnapshotView
): ExchangeSnapshotView => {
  const normalized: ExchangeSnapshotView = {
    version: snapshot.version,
    sequenceNumber: snapshot.sequenceNumber,
    slot: BigInt(snapshot.slot),
    slotIndex: snapshot.slotIndex,
    exchange: cloneExchange(snapshot.exchange),
    markets: snapshot.markets.map(cloneMarket),
  };
  sortMarkets(normalized.markets);
  return normalized;
};

const toLifecycle = (marketStatus: string): ExchangeMarketLifecycle => {
  const normalized = marketStatus.trim().toLowerCase();
  if (normalized === MARKET_STATUS_ACTIVE) {
    return "active";
  }
  if (
    normalized === MARKET_STATUS_CLOSED ||
    normalized === MARKET_STATUS_TOMBSTONED
  ) {
    return "closed";
  }
  return "gated";
};

const buildMarketStatusSummary = (
  market: ExchangeMarketSnapshot
): ExchangeMarketStatusSummary => {
  const lifecycle = toLifecycle(market.marketStatus);
  return {
    symbol: market.symbol,
    marketStatus: market.marketStatus,
    lifecycle,
    isActive: lifecycle === "active",
    isGated: lifecycle === "gated",
    isClosed: lifecycle === "closed",
  };
};

const sortSymbols = (symbols: Iterable<string>): readonly string[] =>
  Array.from(symbols).sort((left, right) => left.localeCompare(right));

const sameMarketPublicMetadata = (
  left: MarketPublicMetadata | null | undefined,
  right: MarketPublicMetadata | null | undefined
): boolean => {
  const leftCalendar = left?.calendar ?? null;
  const rightCalendar = right?.calendar ?? null;
  return (
    (left?.name ?? null) === (right?.name ?? null) &&
    (left?.description ?? null) === (right?.description ?? null) &&
    (left?.logoUri ?? null) === (right?.logoUri ?? null) &&
    (left?.coinGeckoId ?? null) === (right?.coinGeckoId ?? null) &&
    (left?.coinMarketCapId ?? null) === (right?.coinMarketCapId ?? null) &&
    (left?.tokensXyzAssetId ?? null) === (right?.tokensXyzAssetId ?? null) &&
    (leftCalendar?.id ?? null) === (rightCalendar?.id ?? null) &&
    (leftCalendar?.description ?? null) ===
      (rightCalendar?.description ?? null) &&
    (leftCalendar?.calendarUri ?? null) ===
      (rightCalendar?.calendarUri ?? null) &&
    (leftCalendar?.contentSha256 ?? null) ===
      (rightCalendar?.contentSha256 ?? null) &&
    (leftCalendar?.nextMarketTransitionUtc ?? null) ===
      (rightCalendar?.nextMarketTransitionUtc ?? null) &&
    (left?.displayColor ?? null) === (right?.displayColor ?? null)
  );
};

const buildIndexedSnapshot = (
  snapshot: ExchangeSnapshotView,
  previous?: IndexedSnapshot | null
): IndexedSnapshot => {
  const normalized = normalizeSnapshot(snapshot);
  for (const market of normalized.markets) {
    const symbol = normalizeSymbol(market.symbol);
    const previousMetadata = previous?.marketMetadataBySymbol[symbol];
    if (
      market.metadata &&
      previousMetadata &&
      sameMarketPublicMetadata(market.metadata, previousMetadata)
    ) {
      market.metadata = previousMetadata;
    }
  }
  const frozen = deepFreeze(normalized);
  const marketsBySymbol: Record<string, ExchangeMarketSnapshot> = {};
  const marketsByAssetId: Record<number, ExchangeMarketSnapshot> = {};
  const marketsByPubkey: Record<string, ExchangeMarketSnapshot> = {};
  const marketMetadataBySymbol: Record<string, MarketPublicMetadata | null> =
    {};
  const marketMetadataByAssetId: Record<number, MarketPublicMetadata | null> =
    {};
  const marketMetadataByPubkey: Record<string, MarketPublicMetadata | null> =
    {};
  const marketStatusBySymbol: Record<string, ExchangeMarketStatusSummary> = {};
  const activeSymbols: string[] = [];
  const gatedSymbols: string[] = [];
  const closedSymbols: string[] = [];

  for (const market of frozen.markets) {
    const symbol = normalizeSymbol(market.symbol);
    marketsBySymbol[symbol] = market;
    marketsByAssetId[market.assetId] = market;
    marketsByPubkey[market.marketPubkey] = market;
    const metadata = market.metadata ?? null;
    marketMetadataBySymbol[symbol] = metadata;
    marketMetadataByAssetId[market.assetId] = metadata;
    marketMetadataByPubkey[market.marketPubkey] = metadata;

    const summary = buildMarketStatusSummary(market);
    marketStatusBySymbol[symbol] = summary;
    if (summary.isActive) {
      activeSymbols.push(symbol);
    } else if (summary.isClosed) {
      closedSymbols.push(symbol);
    } else {
      gatedSymbols.push(symbol);
    }
  }

  return {
    snapshot: frozen,
    marketsBySymbol,
    marketsByAssetId,
    marketsByPubkey,
    marketMetadataBySymbol,
    marketMetadataByAssetId,
    marketMetadataByPubkey,
    marketSymbols: sortSymbols(Object.keys(marketsBySymbol)),
    activeMarketSymbols: sortSymbols(activeSymbols),
    gatedMarketSymbols: sortSymbols(gatedSymbols),
    closedMarketSymbols: sortSymbols(closedSymbols),
    marketStatusBySymbol,
  };
};

const toRelevantChange = (
  event: ExchangeCacheEvent
): ExchangeRelevantChange | null => {
  const receivedAtMs = Date.now();

  switch (event.type) {
    case "exchangeUpdated":
      return {
        receivedAtMs,
        scope: "exchange",
        symbol: null,
        eventType: event.type,
        detail:
          event.change === "status"
            ? "Exchange status changed"
            : "Exchange keys updated",
        slot: event.slot,
        slotIndex: event.slotIndex,
      };
    case "marketAdded":
      return {
        receivedAtMs,
        scope: "market",
        symbol: event.symbol,
        eventType: event.type,
        detail: `Market added: ${event.symbol}`,
        slot: event.slot,
        slotIndex: event.slotIndex,
      };
    case "marketUpdated":
      return {
        receivedAtMs,
        scope: "market",
        symbol: event.symbol,
        eventType: `${event.type}:${event.change}`,
        detail: `${event.symbol} updated (${event.change})`,
        slot: event.slot,
        slotIndex: event.slotIndex,
      };
    case "marketRemoved":
      return {
        receivedAtMs,
        scope: "market",
        symbol: event.symbol,
        eventType: event.type,
        detail: `Market removed: ${event.symbol}`,
        slot: event.slot,
        slotIndex: event.slotIndex,
      };
    default:
      return null;
  }
};

const buildState = (
  indexed: IndexedSnapshot | null,
  current: PhoenixExchangeStoreState
): PhoenixExchangeStoreState => ({
  ...current,
  snapshot: indexed?.snapshot ?? null,
  marketsBySymbol: indexed?.marketsBySymbol ?? EMPTY_MARKETS_BY_SYMBOL,
  marketsByAssetId: indexed?.marketsByAssetId ?? EMPTY_MARKETS_BY_ASSET_ID,
  marketsByPubkey: indexed?.marketsByPubkey ?? EMPTY_MARKETS_BY_PUBKEY,
  marketMetadataBySymbol:
    indexed?.marketMetadataBySymbol ?? EMPTY_MARKET_METADATA_BY_SYMBOL,
  marketMetadataByAssetId:
    indexed?.marketMetadataByAssetId ?? EMPTY_MARKET_METADATA_BY_ASSET_ID,
  marketMetadataByPubkey:
    indexed?.marketMetadataByPubkey ?? EMPTY_MARKET_METADATA_BY_PUBKEY,
  marketSymbols: indexed?.marketSymbols ?? EMPTY_MARKET_SYMBOLS,
  activeMarketSymbols: indexed?.activeMarketSymbols ?? EMPTY_MARKET_SYMBOLS,
  gatedMarketSymbols: indexed?.gatedMarketSymbols ?? EMPTY_MARKET_SYMBOLS,
  closedMarketSymbols: indexed?.closedMarketSymbols ?? EMPTY_MARKET_SYMBOLS,
  marketStatusBySymbol:
    indexed?.marketStatusBySymbol ?? EMPTY_MARKET_STATUS_BY_SYMBOL,
  lastUpdatedMs: indexed ? Date.now() : current.lastUpdatedMs,
});

export class ExchangeCacheSequenceError extends Error {
  constructor(
    readonly expected: bigint | undefined,
    readonly found: bigint
  ) {
    super(
      expected === undefined
        ? `exchange delta ${found} cannot be applied before a sequenced snapshot`
        : `exchange delta sequence gap: expected ${expected} but received ${found}`
    );
    this.name = "ExchangeCacheSequenceError";
  }
}

class PhoenixExchangeCacheStoreImpl implements PhoenixExchangeCacheStore {
  private indexed: IndexedSnapshot | null;
  private readonly listeners = new Set<(event: ExchangeCacheEvent) => void>();

  readonly store = createStore<PhoenixExchangeStoreState>(() => ({
    health: "cold",
    source: DEFAULT_SOURCE_STATE,
    snapshot: null,
    marketsBySymbol: EMPTY_MARKETS_BY_SYMBOL,
    marketsByAssetId: EMPTY_MARKETS_BY_ASSET_ID,
    marketsByPubkey: EMPTY_MARKETS_BY_PUBKEY,
    marketMetadataBySymbol: EMPTY_MARKET_METADATA_BY_SYMBOL,
    marketMetadataByAssetId: EMPTY_MARKET_METADATA_BY_ASSET_ID,
    marketMetadataByPubkey: EMPTY_MARKET_METADATA_BY_PUBKEY,
    marketSymbols: EMPTY_MARKET_SYMBOLS,
    activeMarketSymbols: EMPTY_MARKET_SYMBOLS,
    gatedMarketSymbols: EMPTY_MARKET_SYMBOLS,
    closedMarketSymbols: EMPTY_MARKET_SYMBOLS,
    marketStatusBySymbol: EMPTY_MARKET_STATUS_BY_SYMBOL,
    latestChange: null,
    recentChanges: EMPTY_RECENT_CHANGES,
    latestChangeBySymbol: EMPTY_LATEST_CHANGE_BY_SYMBOL,
    lastUpdatedMs: null,
  }));

  constructor(initialSnapshot?: ExchangeSnapshotView | null) {
    this.indexed = initialSnapshot
      ? buildIndexedSnapshot(initialSnapshot)
      : null;
    if (this.indexed) {
      this.store.setState((state) => buildState(this.indexed, state));
    }
  }

  snapshot(): Readonly<ExchangeSnapshotView> {
    const snapshot = this.store.getState().snapshot;
    if (!snapshot) {
      throw new Error("Exchange cache snapshot is not available yet.");
    }
    return snapshot;
  }

  market(symbol: string): ExchangeMarketSnapshot | undefined {
    return this.store.getState().marketsBySymbol[normalizeSymbol(symbol)];
  }

  marketByAssetId(assetId: number): ExchangeMarketSnapshot | undefined {
    return this.store.getState().marketsByAssetId[assetId];
  }

  marketByPubkey(pubkey: string): ExchangeMarketSnapshot | undefined {
    return this.store.getState().marketsByPubkey[pubkey];
  }

  marketMetadata(symbol: string): MarketPublicMetadata | null | undefined {
    return this.store.getState().marketMetadataBySymbol[
      normalizeSymbol(symbol)
    ];
  }

  marketMetadataByAssetId(
    assetId: number
  ): MarketPublicMetadata | null | undefined {
    return this.store.getState().marketMetadataByAssetId[assetId];
  }

  marketMetadataByPubkey(
    pubkey: string
  ): MarketPublicMetadata | null | undefined {
    return this.store.getState().marketMetadataByPubkey[pubkey];
  }

  instructionContext(symbol: string): ExchangeInstructionContext | undefined {
    const state = this.store.getState();
    const market = state.marketsBySymbol[normalizeSymbol(symbol)];
    if (!market || !state.snapshot) return undefined;
    return {
      exchange: state.snapshot.exchange,
      market,
    };
  }

  onEvent(listener: (event: ExchangeCacheEvent) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  applySnapshot(
    snapshot: ExchangeSnapshotView,
    source: ExchangeCacheSnapshotSource = "http"
  ): void {
    this.replaceSnapshot(snapshot, source);
  }

  applySnapshotMessage(message: ExchangeSnapshotMsg): void {
    this.replaceSnapshot(
      {
        version: message.version,
        sequenceNumber: message.sequenceNumber,
        slot: message.slot,
        slotIndex: message.slotIndex,
        exchange: message.exchange,
        markets: message.markets,
      },
      "websocket"
    );
  }

  applyDelta(message: ExchangeDeltaMsg): void {
    const current = this.snapshot();
    const previousSequence = current.sequenceNumber;
    if (previousSequence === undefined) {
      throw new ExchangeCacheSequenceError(undefined, message.sequenceNumber);
    }
    const expectedSequence = previousSequence + 1n;
    if (message.sequenceNumber !== expectedSequence) {
      throw new ExchangeCacheSequenceError(
        expectedSequence,
        message.sequenceNumber
      );
    }

    let nextExchange = current.exchange;
    let nextMarkets = current.markets;
    const events: ExchangeCacheEvent[] = [];

    const replaceMarket = (
      symbol: string,
      updater: (market: ExchangeMarketSnapshot) => ExchangeMarketSnapshot
    ): void => {
      const marketIndex = nextMarkets.findIndex(
        (market) => normalizeSymbol(market.symbol) === normalizeSymbol(symbol)
      );
      if (marketIndex < 0) return;
      const updated = updater(nextMarkets[marketIndex]);
      if (updated === nextMarkets[marketIndex]) return;
      if (nextMarkets === current.markets) {
        nextMarkets = [...current.markets];
      }
      nextMarkets[marketIndex] = updated;
    };

    for (const op of message.ops) {
      switch (op.kind) {
        case "exchangeKeysUpdated":
          nextExchange = cloneExchange(op.exchange);
          events.push({
            type: "exchangeUpdated",
            change: "keys",
            slot: message.slot,
            slotIndex: message.slotIndex,
          });
          break;
        case "exchangeStatusChanged":
          nextExchange = {
            ...nextExchange,
            exchangeStatusBits: op.newBits,
            exchangeStatusFeatures: [...op.newFeatures],
            active: op.active,
            gated: op.gated,
          };
          events.push({
            type: "exchangeUpdated",
            change: "status",
            slot: message.slot,
            slotIndex: message.slotIndex,
          });
          break;
        case "marketAdded":
          nextMarkets =
            nextMarkets === current.markets
              ? [...current.markets]
              : nextMarkets;
          nextMarkets.push(cloneMarket(op.market));
          sortMarkets(nextMarkets);
          events.push({
            type: "marketAdded",
            symbol: op.market.symbol,
            slot: message.slot,
            slotIndex: message.slotIndex,
          });
          break;
        case "marketStatusChanged":
          replaceMarket(op.symbol, (market) => ({
            ...cloneMarket(market),
            marketStatus: op.newMarketStatus,
          }));
          events.push({
            type: "marketUpdated",
            symbol: op.symbol,
            change: "status",
            slot: message.slot,
            slotIndex: message.slotIndex,
          });
          break;
        case "marketClosed":
          replaceMarket(op.symbol, (market) => ({
            ...cloneMarket(market),
            marketStatus: MARKET_STATUS_CLOSED,
          }));
          events.push({
            type: "marketUpdated",
            symbol: op.symbol,
            change: "closed",
            slot: message.slot,
            slotIndex: message.slotIndex,
          });
          break;
        case "marketTombstoned":
          replaceMarket(op.symbol, (market) => ({
            ...cloneMarket(market),
            marketStatus: MARKET_STATUS_TOMBSTONED,
          }));
          events.push({
            type: "marketUpdated",
            symbol: op.symbol,
            change: "tombstoned",
            slot: message.slot,
            slotIndex: message.slotIndex,
          });
          break;
        case "marketDeleted":
          nextMarkets =
            nextMarkets === current.markets
              ? [...current.markets]
              : nextMarkets;
          nextMarkets = nextMarkets.filter(
            (market) =>
              normalizeSymbol(market.symbol) !== normalizeSymbol(op.symbol)
          );
          events.push({
            type: "marketRemoved",
            symbol: op.symbol,
            assetId: op.assetId,
            slot: message.slot,
            slotIndex: message.slotIndex,
          });
          break;
        case "marketParameterUpdated":
          {
            const change = this.marketChangeKind(op.update);
            if (change) {
              replaceMarket(op.symbol, (market) =>
                this.applyMarketParameterUpdate(market, op.update)
              );
              events.push({
                type: "marketUpdated",
                symbol: op.symbol,
                change,
                slot: message.slot,
                slotIndex: message.slotIndex,
              });
            }
          }
          break;
        case "marketMetadataUpdated":
          replaceMarket(op.symbol, (market) => ({
            ...cloneMarket(market),
            metadata: op.metadata ? { ...op.metadata } : op.metadata,
          }));
          events.push({
            type: "marketUpdated",
            symbol: op.symbol,
            change: "metadata",
            slot: message.slot,
            slotIndex: message.slotIndex,
          });
          break;
        case "unknown":
          break;
      }
    }

    this.replaceSnapshot(
      {
        version: message.version,
        sequenceNumber: message.sequenceNumber,
        slot: message.slot,
        slotIndex: message.slotIndex,
        exchange: nextExchange,
        markets: nextMarkets,
      },
      "websocket"
    );

    for (const event of events) {
      this.emit(event);
    }
  }

  applyMessage(message: ExchangeMsg): void {
    if (message.messageType === "snapshot") {
      this.applySnapshotMessage(message);
      return;
    }

    this.applyDelta(message);
  }

  setHealth(health: ExchangeCacheHealth): void {
    if (this.store.getState().health === health) {
      return;
    }
    this.store.setState({ health });
  }

  setSource(source: PhoenixExchangeMetadataSourceState): void {
    const current = this.store.getState().source;
    if (
      current.active === source.active &&
      current.priority === source.priority &&
      current.fallbackAvailable === source.fallbackAvailable &&
      current.lastSyncAt === source.lastSyncAt
    ) {
      return;
    }
    this.store.setState({
      source: {
        active: source.active,
        priority: source.priority,
        fallbackAvailable: source.fallbackAvailable,
        lastSyncAt: source.lastSyncAt,
      },
    });
  }

  private emit(event: ExchangeCacheEvent): void {
    const change = toRelevantChange(event);
    if (change) {
      this.store.setState((state) => ({
        latestChange: change,
        recentChanges: [
          ...state.recentChanges.slice(-(MAX_RECENT_CHANGES - 1)),
          change,
        ],
        latestChangeBySymbol:
          change.scope === "market" && change.symbol
            ? {
                ...state.latestChangeBySymbol,
                [change.symbol]: change,
              }
            : state.latestChangeBySymbol,
      }));
    }

    for (const listener of this.listeners) {
      listener(event);
    }
  }

  private replaceSnapshot(
    snapshot: ExchangeSnapshotView | Readonly<ExchangeSnapshotView>,
    source: ExchangeCacheSnapshotSource
  ): void {
    this.indexed = buildIndexedSnapshot(
      snapshot as ExchangeSnapshotView,
      this.indexed
    );
    this.store.setState((state) => buildState(this.indexed, state));
    this.emit({
      type: "snapshotApplied",
      source,
      slot: this.indexed.snapshot.slot,
      slotIndex: this.indexed.snapshot.slotIndex,
      marketCount: this.indexed.snapshot.markets.length,
    });
  }

  private marketChangeKind(
    update: ExchangeMarketParameterUpdate
  ): ExchangeCacheMarketChangeKind | null {
    switch (update.kind) {
      case "cancelRiskFactorUpdated":
        return "cancelRiskFactor";
      case "isolatedOnlyUpdated":
        return "isolatedOnly";
      case "leverageTiersUpdated":
        return "leverageTiers";
      case "markPriceParametersUpdated":
        return "markPriceParameters";
      case "openInterestCapUpdated":
        return "openInterestCap";
      case "upnlRiskFactorUpdated":
        return "upnlRiskFactor";
      case "upnlRiskFactorForWithdrawalsUpdated":
        return "upnlRiskFactorForWithdrawals";
      case "fundingParametersUpdated":
        return "fundingParameters";
      case "marketFeesUpdated":
        return "marketFees";
      case "commodityMetadataUpdated":
        return "commodityMetadata";
      case "unknown":
        return null;
    }
  }

  private applyMarketParameterUpdate(
    market: ExchangeMarketSnapshot,
    update: ExchangeMarketParameterUpdate
  ): ExchangeMarketSnapshot {
    const nextMarket = cloneMarket(market);

    switch (update.kind) {
      case "cancelRiskFactorUpdated":
        nextMarket.riskFactors = {
          ...nextMarket.riskFactors,
          cancelOrder: update.new,
        };
        return nextMarket;
      case "isolatedOnlyUpdated":
        nextMarket.isolatedOnly = update.new;
        return nextMarket;
      case "leverageTiersUpdated":
        nextMarket.leverageTiers = update.new.map(cloneLeverageTier);
        return nextMarket;
      case "markPriceParametersUpdated":
        nextMarket.markPriceParameters = cloneMarkPriceParameters(update.new);
        return nextMarket;
      case "openInterestCapUpdated":
        nextMarket.openInterestCapBaseLots = update.newBaseLots;
        return nextMarket;
      case "upnlRiskFactorUpdated":
        nextMarket.riskFactors = {
          ...nextMarket.riskFactors,
          upnl: update.new,
        };
        return nextMarket;
      case "upnlRiskFactorForWithdrawalsUpdated":
        nextMarket.riskFactors = {
          ...nextMarket.riskFactors,
          upnlForWithdrawals: update.new,
        };
        return nextMarket;
      case "fundingParametersUpdated":
        nextMarket.fundingConfig = { ...update.new };
        return nextMarket;
      case "marketFeesUpdated":
        nextMarket.takerFee = update.new.takerFee;
        nextMarket.makerFee = update.new.makerFee;
        return nextMarket;
      case "commodityMetadataUpdated":
        nextMarket.commodityMetadata = cloneCommodityMetadata(update.new);
        return nextMarket;
      case "unknown":
        return market;
    }
  }
}

export const createPhoenixExchangeCacheStore = (
  initialSnapshot?: ExchangeSnapshotView | null
): PhoenixExchangeCacheStore =>
  new PhoenixExchangeCacheStoreImpl(initialSnapshot);
