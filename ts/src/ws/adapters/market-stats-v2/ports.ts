import type { MarketStatsV2Update } from "./wire";
import type { MarketStatsV2Selector } from "./routing";

export type MarketStatsV2Port = (
  symbols?: MarketStatsV2Selector,
  signal?: AbortSignal
) => AsyncIterable<MarketStatsV2Update>;
