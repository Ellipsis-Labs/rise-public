import { createStore, type StoreApi } from "zustand/vanilla";
import type {
  PhoenixMarketData,
  PhoenixMarketDataChange,
  PhoenixMarketDataRow,
  PhoenixMarketDataStatus,
  PhoenixMarketDataStoreState,
} from "./types";

const normalizeSymbol = (symbol: string): string => symbol.trim().toUpperCase();

const normalizeSelectedSymbol = (symbol: string | null): string | null =>
  symbol ? normalizeSymbol(symbol) : null;

const projectedMarketDataStateCache = new WeakMap<
  PhoenixMarketDataStoreState,
  Map<string | null, PhoenixMarketDataSelectionState>
>();

export interface PhoenixMarketDataSelectionState {
  symbol: string | null;
  row: PhoenixMarketDataRow | null;
  markPrice: number | null;
  midPrice: number | null;
  latestChange: PhoenixMarketDataChange | null;
  status: PhoenixMarketDataStatus;
  lastUpdatedMs: number | null;
}

export interface PhoenixMarketDataSelection {
  readonly store: StoreApi<PhoenixMarketDataSelectionState>;
  symbol(): string | null;
  row(): PhoenixMarketDataRow | null;
  set(symbol: string | null): PhoenixMarketDataRow | null;
  clear(): void;
  ready(): Promise<PhoenixMarketDataRow | null>;
  close(): void;
}

export const projectPhoenixMarketDataSelectionState = (
  state: PhoenixMarketDataStoreState,
  symbol: string | null
): PhoenixMarketDataSelectionState => {
  const normalizedSymbol = normalizeSelectedSymbol(symbol);
  const cachedBySymbol = projectedMarketDataStateCache.get(state);
  const cached = cachedBySymbol?.get(normalizedSymbol);
  if (cached) {
    return cached;
  }

  const row = normalizedSymbol
    ? (state.marketsBySymbol[normalizedSymbol] ?? null)
    : null;
  const projected: PhoenixMarketDataSelectionState = {
    symbol: normalizedSymbol,
    row,
    markPrice: row?.markPrice ?? null,
    midPrice: row?.mid ?? null,
    latestChange:
      normalizedSymbol && state.latestChange?.symbol === normalizedSymbol
        ? state.latestChange
        : null,
    status: state.status,
    lastUpdatedMs: row?.lastUpdatedMs ?? null,
  };

  const nextCache =
    cachedBySymbol ?? new Map<string | null, PhoenixMarketDataSelectionState>();
  nextCache.set(normalizedSymbol, projected);
  if (!cachedBySymbol) {
    projectedMarketDataStateCache.set(state, nextCache);
  }
  return projected;
};

export const selectPhoenixMarketDataRow =
  (symbol: string) =>
  (state: PhoenixMarketDataStoreState): PhoenixMarketDataRow | null =>
    state.marketsBySymbol[normalizeSymbol(symbol)] ?? null;

export const selectMarketDataRow =
  (symbol: string) =>
  (state: PhoenixMarketDataStoreState): PhoenixMarketDataRow | undefined =>
    selectPhoenixMarketDataRow(symbol)(state) ?? undefined;

export const selectPhoenixMarkPrice =
  (symbol: string) =>
  (state: PhoenixMarketDataStoreState): number | null =>
    state.marketsBySymbol[normalizeSymbol(symbol)]?.markPrice ?? null;

export const selectPhoenixMidPrice =
  (symbol: string) =>
  (state: PhoenixMarketDataStoreState): number | null =>
    state.marketsBySymbol[normalizeSymbol(symbol)]?.mid ?? null;

export const createPhoenixMarketDataSelection = (
  marketData: PhoenixMarketData,
  initialSymbol: string | null = null
): PhoenixMarketDataSelection => {
  let currentSymbol = normalizeSelectedSymbol(initialSymbol);
  let closed = false;

  const projectCurrentState = (): PhoenixMarketDataSelectionState =>
    projectPhoenixMarketDataSelectionState(
      marketData.store.getState(),
      currentSymbol
    );

  const store = createStore<PhoenixMarketDataSelectionState>(() =>
    projectCurrentState()
  );
  const release = marketData.retain();
  const unsubscribe = marketData.store.subscribe((state) => {
    if (closed) {
      return;
    }
    store.setState(
      projectPhoenixMarketDataSelectionState(state, currentSymbol)
    );
  });

  const selection: PhoenixMarketDataSelection = {
    store,
    symbol: () => currentSymbol,
    row: () => store.getState().row,
    set: (symbol) => {
      if (closed) {
        throw new Error("Phoenix market data selection is closed");
      }
      currentSymbol = normalizeSelectedSymbol(symbol);
      const nextState = projectCurrentState();
      store.setState(nextState);
      return nextState.row;
    },
    clear: () => {
      if (closed) {
        return;
      }
      currentSymbol = null;
      store.setState(projectCurrentState());
    },
    ready: async () => {
      await marketData.ready();
      if (closed) {
        return null;
      }
      const nextState = projectCurrentState();
      store.setState(nextState);
      return nextState.row;
    },
    close: () => {
      if (closed) {
        return;
      }
      closed = true;
      unsubscribe();
      release();
    },
  };

  return selection;
};
