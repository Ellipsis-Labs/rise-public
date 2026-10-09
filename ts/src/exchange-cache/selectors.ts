import { createStore, type StoreApi } from "zustand/vanilla";
import type {
  ExchangeMarketSnapshot,
  MarketPublicMetadata,
} from "@/api/exchange/types";
import type {
  ExchangeCacheHealth,
  ExchangeMarketStatusSummary,
  ExchangeRelevantChange,
  PhoenixExchangeMetadata,
  PhoenixExchangeMetadataSourceState,
  PhoenixExchangeStoreState,
} from "./types";
import {
  displayTickSize,
  feeRateToMicro,
  priceDecimalsFromTickSize,
  QUOTE_DECIMALS,
} from "@/units";

const normalizeSymbol = (symbol: string): string => symbol.trim().toLowerCase();

const normalizeSelectedSymbol = (symbol: string | null): string | null =>
  symbol ? symbol.trim() : null;

export interface ProjectedExchangeMarket {
  market: ExchangeMarketSnapshot;
  units: {
    tickSizeInQuoteLotsPerBaseLot: number;
    baseLotsDecimals: number;
    quoteDecimals: number;
    tickSize: number;
  };
  fees: {
    takerFeeRate: number;
    makerFeeRate: number;
    takerFeeMicro: number;
    makerFeeMicro: number;
  };
  display: {
    priceDecimals: number;
  };
}

const projectedMarketStateCache = new WeakMap<
  PhoenixExchangeStoreState,
  Map<string | null, PhoenixExchangeMarketState>
>();

export interface PhoenixExchangeMarketState {
  symbol: string | null;
  market: ExchangeMarketSnapshot | null;
  metadata: MarketPublicMetadata | null;
  status: ExchangeMarketStatusSummary | null;
  latestChange: ExchangeRelevantChange | null;
  health: ExchangeCacheHealth;
  source: PhoenixExchangeMetadataSourceState;
  lastUpdatedMs: number | null;
}

export interface PhoenixExchangeMarketSelection {
  readonly store: StoreApi<PhoenixExchangeMarketState>;
  symbol(): string | null;
  market(): ExchangeMarketSnapshot | null;
  set(symbol: string | null): ExchangeMarketSnapshot | null;
  clear(): void;
  ready(): Promise<ExchangeMarketSnapshot | null>;
  close(): void;
}

export const projectPhoenixExchangeMarketState = (
  state: PhoenixExchangeStoreState,
  symbol: string | null
): PhoenixExchangeMarketState => {
  const normalizedSymbol = symbol ? normalizeSymbol(symbol) : null;
  const cachedBySymbol = projectedMarketStateCache.get(state);
  const cached = cachedBySymbol?.get(normalizedSymbol);
  if (cached) {
    return cached;
  }

  const marketSymbol = state.marketSymbols.find(
    (candidate) => normalizeSymbol(candidate) === normalizedSymbol
  );
  const market = marketSymbol
    ? (state.marketsBySymbol[marketSymbol] ?? null)
    : null;
  const projected: PhoenixExchangeMarketState = {
    symbol: market?.symbol ?? normalizeSelectedSymbol(symbol),
    market,
    metadata: marketSymbol
      ? (state.marketMetadataBySymbol[marketSymbol] ?? null)
      : null,
    status: marketSymbol
      ? (state.marketStatusBySymbol[marketSymbol] ?? null)
      : null,
    latestChange: marketSymbol
      ? (state.latestChangeBySymbol[marketSymbol] ?? null)
      : null,
    health: state.health,
    source: state.source,
    lastUpdatedMs: state.lastUpdatedMs,
  };

  const nextCache =
    cachedBySymbol ?? new Map<string | null, PhoenixExchangeMarketState>();
  nextCache.set(normalizedSymbol, projected);
  if (!cachedBySymbol) {
    projectedMarketStateCache.set(state, nextCache);
  }
  return projected;
};

export const selectPhoenixExchangeMarket =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): ExchangeMarketSnapshot | null =>
    projectPhoenixExchangeMarketState(state, symbol).market;

export const selectPhoenixExchangeMarketStatus =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): ExchangeMarketStatusSummary | null =>
    projectPhoenixExchangeMarketState(state, symbol).status;

export const selectPhoenixExchangeMarketMetadata =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): MarketPublicMetadata | null =>
    projectPhoenixExchangeMarketState(state, symbol).metadata;

export const selectPhoenixExchangeMarketChange =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): ExchangeRelevantChange | null =>
    projectPhoenixExchangeMarketState(state, symbol).latestChange;

export const selectPhoenixExchangeMarketState =
  (symbol: string | null) =>
  (state: PhoenixExchangeStoreState): PhoenixExchangeMarketState =>
    projectPhoenixExchangeMarketState(state, symbol);

export const selectExchangeMarket =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): ExchangeMarketSnapshot | undefined =>
    selectPhoenixExchangeMarket(symbol)(state) ?? undefined;

export const selectExchangeMarketStatus =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): ExchangeMarketStatusSummary | undefined =>
    selectPhoenixExchangeMarketStatus(symbol)(state) ?? undefined;

export const selectExchangeMarketMetadata =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): MarketPublicMetadata | undefined =>
    selectPhoenixExchangeMarketMetadata(symbol)(state) ?? undefined;

export const projectExchangeMarket = (
  market: ExchangeMarketSnapshot,
  options: {
    quoteDecimals?: number;
  } = {}
): ProjectedExchangeMarket => {
  const quoteDecimals = options.quoteDecimals ?? QUOTE_DECIMALS;
  return {
    market,
    units: {
      tickSizeInQuoteLotsPerBaseLot: market.tickSize,
      baseLotsDecimals: market.baseLotsDecimals,
      quoteDecimals,
      tickSize: displayTickSize({
        baseLotsDecimals: market.baseLotsDecimals,
        quoteDecimals,
        tickSizeInQuoteLotsPerBaseLot: market.tickSize,
      }),
    },
    fees: {
      takerFeeRate: market.takerFee,
      makerFeeRate: market.makerFee,
      takerFeeMicro: feeRateToMicro(market.takerFee),
      makerFeeMicro: feeRateToMicro(market.makerFee),
    },
    display: {
      priceDecimals: priceDecimalsFromTickSize({
        baseLotsDecimals: market.baseLotsDecimals,
        quoteDecimals,
        tickSizeInQuoteLotsPerBaseLot: market.tickSize,
      }),
    },
  };
};

export const createPhoenixExchangeMarketSelection = (
  exchange: PhoenixExchangeMetadata,
  initialSymbol: string | null = null
): PhoenixExchangeMarketSelection => {
  let currentSymbol = normalizeSelectedSymbol(initialSymbol);
  let closed = false;

  const projectCurrentState = (): PhoenixExchangeMarketState =>
    projectPhoenixExchangeMarketState(exchange.store.getState(), currentSymbol);

  const store = createStore<PhoenixExchangeMarketState>(() =>
    projectCurrentState()
  );

  const unsubscribe = exchange.store.subscribe((state) => {
    if (closed) {
      return;
    }
    store.setState(projectPhoenixExchangeMarketState(state, currentSymbol));
  });

  const selection: PhoenixExchangeMarketSelection = {
    store,
    symbol: () => store.getState().symbol,
    market: () => store.getState().market,
    set: (symbol) => {
      if (closed) {
        throw new Error("Phoenix exchange market selection is closed");
      }
      currentSymbol = normalizeSelectedSymbol(symbol);
      const nextState = projectCurrentState();
      store.setState(nextState);
      return nextState.market;
    },
    clear: () => {
      if (closed) {
        return;
      }
      currentSymbol = null;
      store.setState(projectCurrentState());
    },
    ready: async () => {
      await exchange.ready();
      if (closed) {
        return null;
      }
      const nextState = projectCurrentState();
      store.setState(nextState);
      return nextState.market;
    },
    close: () => {
      if (closed) {
        return;
      }
      closed = true;
      unsubscribe();
    },
  };

  return selection;
};
