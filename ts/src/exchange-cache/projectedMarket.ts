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
import {
  displayTickSize,
  feeRateToMicro,
  priceDecimalsFromTickSize,
  riskFactorPercentToBps,
} from "@/units";

const projectedMarketCache = new WeakMap<
  ExchangeMarketSnapshot,
  PhoenixProjectedMarket
>();
const projectedMarketsBySymbolCache = new WeakMap<
  Record<string, ExchangeMarketSnapshot>,
  PhoenixProjectedMarketsBySymbol
>();

const toMarketFees = (market: ExchangeMarketSnapshot): MarketFees => ({
  takerFeeMicro: feeRateToMicro(market.takerFee),
  makerFeeMicro: feeRateToMicro(market.makerFee),
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
    priceDecimals: priceDecimalsFromTickSize(units),
    tickSize: displayTickSize(units),
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
