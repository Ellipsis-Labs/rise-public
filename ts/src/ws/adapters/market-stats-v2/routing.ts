export type MarketStatsV2Selector = string | readonly string[];

export const normalizeMarketStatsV2Symbols = (
  selector?: MarketStatsV2Selector
): string[] | undefined => {
  if (selector === undefined) {
    return undefined;
  }

  const symbols = typeof selector === "string" ? [selector] : [...selector];
  if (symbols.length === 0) {
    throw new Error(
      "marketStatsV2 symbols must not be empty; omit symbols to subscribe to all markets"
    );
  }

  const normalized = symbols.map((symbol) => symbol.trim().toUpperCase());
  if (normalized.some((symbol) => symbol.length === 0)) {
    throw new Error("marketStatsV2 symbols must not contain empty values");
  }

  return [...new Set(normalized)].sort();
};

export const buildMarketStatsV2SubscriptionParams = (
  selector?: MarketStatsV2Selector
): { symbols?: string[] } => {
  const symbols = normalizeMarketStatsV2Symbols(selector);
  return symbols ? { symbols } : {};
};

export const buildMarketStatsV2RoutingKey = (
  selector?: MarketStatsV2Selector
): string => {
  const symbols = normalizeMarketStatsV2Symbols(selector);
  return symbols ? `marketStatsV2:${JSON.stringify(symbols)}` : "marketStatsV2";
};
