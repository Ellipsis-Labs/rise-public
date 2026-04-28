import { createStore, type StoreApi } from "zustand/vanilla";
import type { ExchangeMarketSnapshot } from "@/api/exchange/types";
import type {
  ExchangeCacheHealth,
  ExchangeMarketStatusSummary,
  ExchangeRelevantChange,
  PhoenixExchangeMetadata,
  PhoenixExchangeMetadataSourceState,
  PhoenixExchangeStoreState,
} from "./types";

const DEFAULT_QUOTE_DECIMALS = 6;
const FEE_MICRO_MULTIPLIER = 1_000_000;

const normalizePriceDecimalsForDisplay = (decimals: number): number => {
  const normalized = Math.max(0, decimals);
  return normalized === 1 ? 2 : normalized;
};

const normalizeSymbol = (symbol: string): string => symbol.trim().toUpperCase();

const normalizeSelectedSymbol = (symbol: string | null): string | null =>
  symbol ? normalizeSymbol(symbol) : null;

const getPriceDecimalsFromLotTickSize = (params: {
  baseLotsDecimals: number;
  quoteDecimals: number;
  tickSizeInQuoteLotsPerBaseLot: number;
}): number => {
  const decimalShift = Math.max(
    0,
    params.quoteDecimals - params.baseLotsDecimals
  );
  let normalizedTickSize = Math.abs(params.tickSizeInQuoteLotsPerBaseLot);
  let trailingZeroCount = 0;

  while (normalizedTickSize !== 0 && normalizedTickSize % 10 === 0) {
    normalizedTickSize /= 10;
    trailingZeroCount += 1;
  }

  return normalizePriceDecimalsForDisplay(
    Math.max(0, decimalShift - trailingZeroCount)
  );
};

const getDisplayTickSize = (params: {
  baseLotsDecimals: number;
  quoteDecimals: number;
  tickSizeInQuoteLotsPerBaseLot: number;
}): number => {
  const tickSize =
    (10 ** params.baseLotsDecimals / 10 ** params.quoteDecimals) *
    params.tickSizeInQuoteLotsPerBaseLot;
  const decimals = getPriceDecimalsFromLotTickSize(params);
  return Number(tickSize.toFixed(decimals));
};

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
  const normalizedSymbol = normalizeSelectedSymbol(symbol);
  const cachedBySymbol = projectedMarketStateCache.get(state);
  const cached = cachedBySymbol?.get(normalizedSymbol);
  if (cached) {
    return cached;
  }

  const projected: PhoenixExchangeMarketState = {
    symbol: normalizedSymbol,
    market: normalizedSymbol
      ? (state.marketsBySymbol[normalizedSymbol] ?? null)
      : null,
    status: normalizedSymbol
      ? (state.marketStatusBySymbol[normalizedSymbol] ?? null)
      : null,
    latestChange: normalizedSymbol
      ? (state.latestChangeBySymbol[normalizedSymbol] ?? null)
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
    state.marketsBySymbol[normalizeSymbol(symbol)] ?? null;

export const selectPhoenixExchangeMarketStatus =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): ExchangeMarketStatusSummary | null =>
    state.marketStatusBySymbol[normalizeSymbol(symbol)] ?? null;

export const selectPhoenixExchangeMarketChange =
  (symbol: string) =>
  (state: PhoenixExchangeStoreState): ExchangeRelevantChange | null =>
    state.latestChangeBySymbol[normalizeSymbol(symbol)] ?? null;

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

export const projectExchangeMarket = (
  market: ExchangeMarketSnapshot,
  options: {
    quoteDecimals?: number;
  } = {}
): ProjectedExchangeMarket => {
  const quoteDecimals = options.quoteDecimals ?? DEFAULT_QUOTE_DECIMALS;
  return {
    market,
    units: {
      tickSizeInQuoteLotsPerBaseLot: market.tickSize,
      baseLotsDecimals: market.baseLotsDecimals,
      quoteDecimals,
      tickSize: getDisplayTickSize({
        baseLotsDecimals: market.baseLotsDecimals,
        quoteDecimals,
        tickSizeInQuoteLotsPerBaseLot: market.tickSize,
      }),
    },
    fees: {
      takerFeeRate: market.takerFee,
      makerFeeRate: market.makerFee,
      takerFeeMicro: Math.round(market.takerFee * FEE_MICRO_MULTIPLIER),
      makerFeeMicro: Math.round(market.makerFee * FEE_MICRO_MULTIPLIER),
    },
    display: {
      priceDecimals: getPriceDecimalsFromLotTickSize({
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
    symbol: () => currentSymbol,
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
