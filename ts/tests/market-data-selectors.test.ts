import { createStore } from "zustand/vanilla";
import { describe, expect, it, vi } from "vitest";
import type { PhoenixMarketData, PhoenixMarketDataStoreState } from "@/index";
import {
  createPhoenixMarketDataSelection,
  projectPhoenixMarketDataSelectionState,
  selectPhoenixMarkPrice,
  selectPhoenixMarketDataRow,
  selectPhoenixMidPrice,
} from "@/index";

const createMarketDataState = (): PhoenixMarketDataStoreState => ({
  status: {
    health: "live",
    isConnected: true,
    isLoading: false,
    error: null,
  },
  symbols: ["SOL"],
  marketsBySymbol: {
    SOL: {
      symbol: "SOL",
      timestamp: 1_700_000_000,
      mid: 125.5,
      markPrice: 126.25,
      oraclePrice: 126,
      openInterest: 1000,
      currentFundingRate: 0.01,
      eightHourFundingRate: 0.08,
      annualizedFundingRate: 1.2,
      prevDayMarkPrice: 120,
      dayVolumeUsd: 10_000,
      dayVolumeBase: 500,
      priceChange24h: 6.25,
      priceChange24hPercent: 5.2,
      lastMidSlot: 10n,
      lastMidSlotIndex: 1,
      lastMarkPriceSlot: 11n,
      lastMarkPriceSlotIndex: 2,
      lastUpdatedMs: 1_700_000_000_500,
    },
  },
  latestChange: {
    receivedAtMs: 1_700_000_000_500,
    source: "marketStats",
    raw: {
      channel: "marketStats",
      messageType: "update",
      timestamp: 1_700_000_000,
      stats: [
        {
          symbol: "SOL",
          markPrice: 126.25,
          oraclePrice: 126,
          currentFundingRate: 0.01,
          annualizedFundingRate: 1.2,
          eightHourFundingRate: 0.08,
          openInterest: 1000,
          dayVolumeUsd: 10_000,
          dayVolumeBase: 500,
          prevDayMarkPrice: 120,
          priceChange24h: 6.25,
          priceChange24hPercent: 5.2,
        },
      ],
    },
    symbol: "SOL",
    slot: 11n,
    slotIndex: 2,
    changedFields: ["markPrice", "oraclePrice", "timestamp"],
    symbolCount: 1,
    markPrice: 126.25,
    previousMarkPrice: 125,
    markPriceDelta: 1.25,
    oraclePrice: 126,
    previousOraclePrice: 125.5,
    oraclePriceDelta: 0.5,
    openInterest: 1000,
    previousOpenInterest: 950,
    openInterestDelta: 50,
    currentFundingRate: 0.01,
    previousFundingRate: 0.009,
    fundingRateDelta: 0.001,
    dayVolumeUsd: 10_000,
    previousDayVolumeUsd: 9_500,
    dayVolumeUsdDelta: 500,
    mid: 125.5,
    previousMid: 125,
    midDelta: 0.5,
  },
  recentChanges: [],
});

describe("market-data selectors", () => {
  it("projects market data rows and supports narrow selections", async () => {
    const store = createStore<PhoenixMarketDataStoreState>(
      createMarketDataState
    );
    const release = vi.fn();
    const marketData: PhoenixMarketData = {
      store,
      retain: vi.fn(() => release),
      ready: vi.fn(async () => store.getState()),
      status: () => store.getState().status,
      snapshot: () => store.getState(),
      market: (symbol) =>
        store.getState().marketsBySymbol[symbol.toUpperCase()],
      reconnect: vi.fn(),
      close: vi.fn(),
    };

    expect(selectPhoenixMarketDataRow("sol")(store.getState())?.symbol).toBe(
      "SOL"
    );
    expect(selectPhoenixMarkPrice("sol")(store.getState())).toBe(126.25);
    expect(selectPhoenixMidPrice("sol")(store.getState())).toBe(125.5);

    const projectedA = projectPhoenixMarketDataSelectionState(
      store.getState(),
      "sol"
    );
    const projectedB = projectPhoenixMarketDataSelectionState(
      store.getState(),
      "SOL"
    );
    expect(projectedA).toBe(projectedB);

    const selection = createPhoenixMarketDataSelection(marketData, "SOL");
    expect(marketData.retain).toHaveBeenCalledTimes(1);
    expect(selection.store.getState().row?.markPrice).toBe(126.25);

    store.setState((state) => ({
      ...state,
      marketsBySymbol: {
        ...state.marketsBySymbol,
        SOL: {
          ...state.marketsBySymbol.SOL!,
          markPrice: 127.5,
          lastUpdatedMs: 1_700_000_001_000,
        },
      },
    }));

    expect(selection.store.getState().markPrice).toBe(127.5);
    expect(selection.store.getState().lastUpdatedMs).toBe(1_700_000_001_000);

    await selection.ready();
    expect(marketData.ready).toHaveBeenCalledTimes(1);

    selection.clear();
    expect(selection.store.getState().row).toBeNull();

    selection.close();
    expect(release).toHaveBeenCalledTimes(1);
  });
});
