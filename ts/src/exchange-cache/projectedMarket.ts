import type {
  ExchangeMarketSnapshot,
  ExchangeWsCommodityMetadata,
} from "@/api/exchange/types";
import { symbol, type Symbol } from "@/primitives/Symbol";
import type {
  MarketFees,
  MarketLeverageTier,
  MarketUnits,
  RiskFactors,
} from "@/types";

const QUOTE_LOTS_DECIMALS = 6;
const FEE_MICRO_MULTIPLIER = 1_000_000;

const projectedMarketCache = new WeakMap<
  ExchangeMarketSnapshot,
  PhoenixProjectedMarket
>();
const projectedMarketsBySymbolCache = new WeakMap<
  Record<string, ExchangeMarketSnapshot>,
  PhoenixProjectedMarketsBySymbol
>();

const normalizePriceDecimalsForDisplay = (decimals: number): number => {
  const normalized = Math.max(0, Math.trunc(decimals));
  return normalized === 1 ? 2 : normalized;
};

const getDecimalPlacesFromLotTickSize = ({
  baseLotsDecimals,
  quoteLotsDecimals,
  tickSizeInQuoteLotsPerBaseLot,
}: {
  baseLotsDecimals: number;
  quoteLotsDecimals: number;
  tickSizeInQuoteLotsPerBaseLot: number;
}): number => {
  const decimalShift = Math.max(0, quoteLotsDecimals - baseLotsDecimals);

  let normalizedTickSize = Math.abs(tickSizeInQuoteLotsPerBaseLot);
  let trailingZeroCount = 0;

  while (normalizedTickSize !== 0 && normalizedTickSize % 10 === 0) {
    normalizedTickSize /= 10;
    trailingZeroCount += 1;
  }

  return normalizePriceDecimalsForDisplay(
    Math.max(0, decimalShift - trailingZeroCount)
  );
};

const convertTickSizeToDecimalUnits = ({
  baseLotsDecimals,
  quoteLotsDecimals,
  tickSizeInQuoteLotsPerBaseLot,
}: {
  baseLotsDecimals: number;
  quoteLotsDecimals: number;
  tickSizeInQuoteLotsPerBaseLot: number;
}): number =>
  tickSizeInQuoteLotsPerBaseLot *
  Math.pow(10, baseLotsDecimals - quoteLotsDecimals);

const toMarketFees = (market: ExchangeMarketSnapshot): MarketFees => ({
  takerFeeMicro: Math.round(market.takerFee * FEE_MICRO_MULTIPLIER),
  makerFeeMicro: Math.round(market.makerFee * FEE_MICRO_MULTIPLIER),
});

const toLeverageTiers = (
  market: ExchangeMarketSnapshot
): readonly MarketLeverageTier[] =>
  market.leverageTiers.map((tier) => ({
    maxLeverage: tier.maxLeverage,
    maxSizeBaseLots: Number(tier.maxSizeBaseLots),
    limitOrderRiskFactor: tier.limitOrderRiskFactor,
  }));

const toUnits = (market: ExchangeMarketSnapshot): MarketUnits => ({
  baseLotsDecimals: market.baseLotsDecimals,
  tickSizeInQuoteLotsPerBaseLot: market.tickSize,
});

const riskFactorPercentToBps = (value: number): number => {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.round(value * 100);
};

const toRiskFactors = (market: ExchangeMarketSnapshot): RiskFactors => ({
  maintenance:
    market.riskFactors.maintenanceBps ??
    riskFactorPercentToBps(market.riskFactors.maintenance),
  backstop:
    market.riskFactors.backstopBps ??
    riskFactorPercentToBps(market.riskFactors.backstop),
  highRisk:
    market.riskFactors.highRiskBps ??
    riskFactorPercentToBps(market.riskFactors.highRisk),
  upnl:
    market.riskFactors.upnlBps ??
    riskFactorPercentToBps(market.riskFactors.upnl),
  upnlForWithdrawals:
    market.riskFactors.upnlForWithdrawalsBps ??
    riskFactorPercentToBps(market.riskFactors.upnlForWithdrawals),
  cancelOrder:
    market.riskFactors.cancelOrderBps ??
    riskFactorPercentToBps(market.riskFactors.cancelOrder),
});

export interface PhoenixProjectedMarket {
  symbol: Symbol;
  assetId: number;
  marketStatus: string;
  marketPubkey: string;
  splinePubkey: string;
  commodityMetadata: ExchangeWsCommodityMetadata | null;
  units: MarketUnits;
  fees: MarketFees;
  leverageTiers: readonly MarketLeverageTier[];
  riskFactors: RiskFactors;
  isolatedOnly: boolean;
  priceDecimals: number;
  tickSize: number;
}

export type PhoenixProjectedMarketsBySymbol = Record<
  string,
  PhoenixProjectedMarket
>;

export const projectPhoenixMarket = (
  market: ExchangeMarketSnapshot
): PhoenixProjectedMarket => {
  const cached = projectedMarketCache.get(market);
  if (cached) {
    return cached;
  }

  const units = toUnits(market);
  const projected: PhoenixProjectedMarket = {
    symbol: symbol(market.symbol),
    assetId: market.assetId,
    marketStatus: market.marketStatus,
    marketPubkey: market.marketPubkey,
    splinePubkey: market.splinePubkey,
    commodityMetadata: market.commodityMetadata ?? null,
    units,
    fees: toMarketFees(market),
    leverageTiers: toLeverageTiers(market),
    riskFactors: toRiskFactors(market),
    isolatedOnly: market.isolatedOnly,
    priceDecimals: getDecimalPlacesFromLotTickSize({
      baseLotsDecimals: units.baseLotsDecimals,
      quoteLotsDecimals: QUOTE_LOTS_DECIMALS,
      tickSizeInQuoteLotsPerBaseLot: units.tickSizeInQuoteLotsPerBaseLot,
    }),
    tickSize: convertTickSizeToDecimalUnits({
      baseLotsDecimals: units.baseLotsDecimals,
      quoteLotsDecimals: QUOTE_LOTS_DECIMALS,
      tickSizeInQuoteLotsPerBaseLot: units.tickSizeInQuoteLotsPerBaseLot,
    }),
  };

  projectedMarketCache.set(market, projected);
  return projected;
};

export const projectPhoenixMarketsBySymbol = (
  marketsBySymbol: Readonly<Record<string, ExchangeMarketSnapshot>>
): PhoenixProjectedMarketsBySymbol => {
  const cacheKey = marketsBySymbol as Record<string, ExchangeMarketSnapshot>;
  const cached = projectedMarketsBySymbolCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  const projected: PhoenixProjectedMarketsBySymbol = {};
  for (const [marketSymbol, market] of Object.entries(marketsBySymbol)) {
    projected[marketSymbol] = projectPhoenixMarket(market);
  }

  projectedMarketsBySymbolCache.set(cacheKey, projected);
  return projected;
};
