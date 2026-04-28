import type { MarketResponse } from "@/api/markets/types";
import type { MarketsResponse, MarketSummary } from "@/types";
import type { MarketParamsBySymbol } from "./compute";
import type { MarketParams } from "./types";

export type MissingPriceBehavior = "error" | "skip" | "zero";

export interface MarketParamsBuildOptions {
  missingPriceBehavior?: MissingPriceBehavior;
}

export interface MarginMarketsClient {
  getMarkets(): Promise<MarketsResponse>;
  getMarket(symbol: string): Promise<MarketResponse>;
}

export interface MarginClientWithMarketsProperty {
  markets: MarginMarketsClient;
}

export interface MarginClientWithMarketsMethod {
  markets(): MarginMarketsClient;
}

export type MarginClient =
  | MarginClientWithMarketsProperty
  | MarginClientWithMarketsMethod;

export interface MarketPriceSource<
  ClientType extends MarginClient = MarginClient,
> {
  getMarkPrices(
    symbols: string[],
    client: ClientType
  ): Promise<Record<string, number | string | null>>;
}

export interface MarginMarketParamsSnapshot {
  paramsBySymbol: MarketParamsBySymbol;
  updatedAt: number;
  slot?: number;
}

export interface MarginMarketParamsStoreOptions<
  ClientType extends MarginClient = MarginClient,
> {
  client: ClientType;
  ttlMs?: number;
  autoRefreshIntervalMs?: number;
  missingPriceBehavior?: MissingPriceBehavior;
  priceSource?: MarketPriceSource<ClientType>;
  now?: () => number;
}

const DEFAULT_TTL_MS = 30_000;

const MICRO_USD = 1_000_000n;

const resolveMarketsClient = (client: MarginClient): MarginMarketsClient => {
  if ("markets" in client) {
    return typeof client.markets === "function"
      ? client.markets()
      : client.markets;
  }

  throw new Error("Margin client does not expose public market endpoints");
};

const expandScientificNotation = (value: string): string => {
  const match = value
    .trim()
    .toLowerCase()
    .match(/^([+-]?)(\d+)(?:\.(\d*))?e([+-]?\d+)$/);

  if (!match) {
    return value;
  }

  const sign = match[1] ?? "";
  const integerPart = match[2] ?? "0";
  const fractionalPart = match[3] ?? "";
  const exponent = Number(match[4]);

  const digits = `${integerPart}${fractionalPart}`;
  const decimalIndex = integerPart.length + exponent;

  if (decimalIndex <= 0) {
    return `${sign}0.${"0".repeat(-decimalIndex)}${digits}`;
  }

  if (decimalIndex >= digits.length) {
    return `${sign}${digits}${"0".repeat(decimalIndex - digits.length)}`;
  }

  return `${sign}${digits.slice(0, decimalIndex)}.${digits.slice(decimalIndex)}`;
};

const parseUsdToMicros = (value: number | string): bigint => {
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error("mark price is not finite");
    }
    return parseUsdToMicros(value.toString());
  }

  const normalized = expandScientificNotation(value);
  const trimmed = normalized.trim();
  if (!trimmed) {
    throw new Error("mark price is empty");
  }

  let sign = 1n;
  let digits = trimmed;
  if (digits.startsWith("+")) {
    digits = digits.slice(1);
  } else if (digits.startsWith("-")) {
    sign = -1n;
    digits = digits.slice(1);
  }

  const [whole = "0", fraction = ""] = digits.split(".");
  if (!/^\d+$/.test(whole) || (fraction && !/^\d+$/.test(fraction))) {
    throw new Error(`invalid mark price: ${value}`);
  }

  const paddedFraction = `${fraction}000000`.slice(0, 6);
  const micros = BigInt(whole) * MICRO_USD + BigInt(paddedFraction || "0");
  return sign * micros;
};

export const priceUsdToTicks = (
  priceUsd: number | string,
  units: { baseLotsDecimals: number; tickSizeInQuoteLotsPerBaseLot: number }
): string => {
  const priceMicros = parseUsdToMicros(priceUsd);
  const tickSize = BigInt(units.tickSizeInQuoteLotsPerBaseLot);
  if (tickSize === 0n) {
    throw new Error("tick size denominator is zero");
  }

  const scale = 10n ** BigInt(Math.abs(units.baseLotsDecimals));
  let numerator = priceMicros;
  let denominator = tickSize;

  if (units.baseLotsDecimals >= 0) {
    denominator *= scale;
  } else {
    numerator *= scale;
  }

  return (numerator / denominator).toString();
};

export const buildMarketParamsFromSummary = (
  market: MarketSummary,
  markPriceUsd: number | string | null | undefined,
  opts?: MarketParamsBuildOptions
): MarketParams | null => {
  const missingPriceBehavior = opts?.missingPriceBehavior ?? "error";

  if (markPriceUsd === null || markPriceUsd === undefined) {
    if (missingPriceBehavior === "skip") {
      return null;
    }
    if (missingPriceBehavior === "zero") {
      markPriceUsd = 0;
    } else {
      throw new Error(`Missing mark price for ${String(market.symbol)}`);
    }
  }

  return {
    symbol: market.symbol,
    assetId: market.assetId,
    markPriceTicks: priceUsdToTicks(markPriceUsd, market.units),
    tickSize: market.units.tickSizeInQuoteLotsPerBaseLot.toString(),
    baseLotDecimals: market.units.baseLotsDecimals,
    leverageTiers: market.leverageTiers.map((tier) => ({
      upperBoundSize: tier.maxSizeBaseLots.toString(),
      maxLeverage: tier.maxLeverage.toString(),
      limitOrderRiskFactorBps: tier.limitOrderRiskFactor.toString(),
    })),
    riskFactors: {
      maintenanceMarginFactorBps: market.riskFactors.maintenance.toString(),
      backstopMarginFactorBps: market.riskFactors.backstop.toString(),
      highRiskMarginFactorBps: market.riskFactors.highRisk.toString(),
    },
    cancelOrderRiskFactorBps: market.riskFactors.cancelOrder.toString(),
    upnlRiskFactor: market.riskFactors.upnl.toString(),
    upnlRiskFactorForWithdrawals:
      market.riskFactors.upnlForWithdrawals.toString(),
    isolatedOnly: market.isolatedOnly,
  };
};

export const buildMarketParamsBySymbol = (
  markets: MarketSummary[],
  markPricesBySymbol: Record<string, number | string | null>,
  opts?: MarketParamsBuildOptions
): MarketParamsBySymbol => {
  const paramsBySymbol: MarketParamsBySymbol = {};
  for (const market of markets) {
    const symbol = String(market.symbol);
    const params = buildMarketParamsFromSummary(
      market,
      markPricesBySymbol[symbol],
      opts
    );
    if (!params) {
      continue;
    }
    paramsBySymbol[symbol] = params;
  }
  return paramsBySymbol;
};

const defaultPriceSource: MarketPriceSource = {
  async getMarkPrices(symbols, client) {
    const marketsClient = resolveMarketsClient(client);
    const entries = await Promise.all(
      symbols.map(async (symbol) => {
        const market = await marketsClient.getMarket(symbol);
        return [symbol, market.market.markPrice?.price ?? null] as const;
      })
    );
    return Object.fromEntries(entries);
  },
};

export class MarginMarketParamsStore<
  ClientType extends MarginClient = MarginClient,
> {
  private client: ClientType;
  private ttlMs: number;
  private autoRefreshIntervalMs?: number;
  private priceSource: MarketPriceSource<ClientType>;
  private missingPriceBehavior: MissingPriceBehavior;
  private now: () => number;
  private refreshPromise: Promise<MarginMarketParamsSnapshot> | null = null;
  private lastSnapshot: MarginMarketParamsSnapshot | null = null;
  private lastMarkets: MarketSummary[] | null = null;
  private listeners = new Set<(snapshot: MarginMarketParamsSnapshot) => void>();
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor(opts: MarginMarketParamsStoreOptions<ClientType>) {
    this.client = opts.client;
    this.ttlMs = opts.ttlMs ?? DEFAULT_TTL_MS;
    this.autoRefreshIntervalMs = opts.autoRefreshIntervalMs;
    this.priceSource = opts.priceSource ?? defaultPriceSource;
    this.missingPriceBehavior = opts.missingPriceBehavior ?? "error";
    this.now = opts.now ?? Date.now;

    if (this.autoRefreshIntervalMs !== undefined) {
      this.startAutoRefresh();
    }
  }

  subscribe(
    listener: (snapshot: MarginMarketParamsSnapshot) => void
  ): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async getMarketParamsBySymbol(): Promise<MarketParamsBySymbol> {
    const snapshot = await this.getSnapshot();
    return snapshot.paramsBySymbol;
  }

  async getSnapshot(): Promise<MarginMarketParamsSnapshot> {
    if (this.lastSnapshot && !this.isExpired()) {
      return this.lastSnapshot;
    }
    return this.refresh();
  }

  async refresh(): Promise<MarginMarketParamsSnapshot> {
    if (this.refreshPromise) {
      return this.refreshPromise;
    }

    this.refreshPromise = this.refreshInternal().finally(() => {
      this.refreshPromise = null;
    });

    return this.refreshPromise;
  }

  updateMarkPrices(
    markPricesBySymbol: Record<string, number | string | null>
  ): void {
    if (!this.lastMarkets) {
      throw new Error("Market summaries have not been loaded yet");
    }

    const paramsBySymbol = buildMarketParamsBySymbol(
      this.lastMarkets,
      markPricesBySymbol,
      { missingPriceBehavior: this.missingPriceBehavior }
    );
    this.updateSnapshot(paramsBySymbol, this.lastSnapshot?.slot);
  }

  startAutoRefresh(): void {
    if (this.timer || this.autoRefreshIntervalMs === undefined) {
      return;
    }
    this.timer = setInterval(() => {
      void this.refresh().catch(() => undefined);
    }, this.autoRefreshIntervalMs);
  }

  stopAutoRefresh(): void {
    if (!this.timer) {
      return;
    }
    clearInterval(this.timer);
    this.timer = null;
  }

  private isExpired(): boolean {
    if (!this.lastSnapshot) {
      return true;
    }
    if (!Number.isFinite(this.ttlMs)) {
      return false;
    }
    return this.now() - this.lastSnapshot.updatedAt > this.ttlMs;
  }

  private async refreshInternal(): Promise<MarginMarketParamsSnapshot> {
    const marketsClient = resolveMarketsClient(this.client);
    const marketsResponse = await marketsClient.getMarkets();
    const markets = marketsResponse.markets;
    const symbols = markets.map((market) => String(market.symbol));
    const markPricesBySymbol = await this.priceSource.getMarkPrices(
      symbols,
      this.client
    );

    this.lastMarkets = markets;
    const paramsBySymbol = buildMarketParamsBySymbol(
      markets,
      markPricesBySymbol,
      {
        missingPriceBehavior: this.missingPriceBehavior,
      }
    );
    return this.updateSnapshot(paramsBySymbol, marketsResponse.slot);
  }

  private updateSnapshot(
    paramsBySymbol: MarketParamsBySymbol,
    slot: number | undefined
  ): MarginMarketParamsSnapshot {
    const snapshot = {
      paramsBySymbol,
      updatedAt: this.now(),
      slot,
    };
    this.lastSnapshot = snapshot;
    for (const listener of this.listeners) {
      listener(snapshot);
    }
    return snapshot;
  }
}
