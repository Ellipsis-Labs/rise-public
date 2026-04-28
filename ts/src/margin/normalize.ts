import type { LeverageTier } from "./math";
import { toBigInt } from "./math";
import type { LeverageTierParams, MarketParams } from "./types";

export interface NormalizedRiskFactors {
  maintenanceMarginFactorBps: bigint;
  backstopMarginFactorBps: bigint;
  highRiskMarginFactorBps: bigint;
}

export interface NormalizedMarketParams {
  symbol: string;
  assetId: number;
  markPriceTicks: bigint;
  tickSize: bigint;
  baseLotDecimals: number;
  leverageTiers: LeverageTier[];
  riskFactors: NormalizedRiskFactors;
  cancelOrderRiskFactorBps: bigint;
  upnlRiskFactor: bigint;
  upnlRiskFactorForWithdrawals: bigint;
  isolatedOnly: boolean;
}

export type NormalizedMarketParamsBySymbol = Record<
  string,
  NormalizedMarketParams
>;

const parseLeverageTiers = (tiers: LeverageTierParams[]): LeverageTier[] =>
  tiers.map((tier) => ({
    upperBoundSize: toBigInt(tier.upperBoundSize),
    maxLeverage: toBigInt(tier.maxLeverage),
    limitOrderRiskFactorBps: toBigInt(tier.limitOrderRiskFactorBps),
  }));

export const normalizeMarketParams = (
  market: MarketParams
): NormalizedMarketParams => ({
  symbol: market.symbol,
  assetId: market.assetId,
  markPriceTicks: toBigInt(market.markPriceTicks),
  tickSize: toBigInt(market.tickSize),
  baseLotDecimals: market.baseLotDecimals,
  leverageTiers: parseLeverageTiers(market.leverageTiers),
  riskFactors: {
    maintenanceMarginFactorBps: toBigInt(
      market.riskFactors.maintenanceMarginFactorBps
    ),
    backstopMarginFactorBps: toBigInt(
      market.riskFactors.backstopMarginFactorBps
    ),
    highRiskMarginFactorBps: toBigInt(
      market.riskFactors.highRiskMarginFactorBps
    ),
  },
  cancelOrderRiskFactorBps: toBigInt(market.cancelOrderRiskFactorBps),
  upnlRiskFactor: toBigInt(market.upnlRiskFactor),
  upnlRiskFactorForWithdrawals: toBigInt(market.upnlRiskFactorForWithdrawals),
  isolatedOnly: market.isolatedOnly,
});

export const buildNormalizedMarketParamsBySymbol = (
  markets: MarketParams[]
): NormalizedMarketParamsBySymbol => {
  const map: NormalizedMarketParamsBySymbol = {};
  for (const market of markets) {
    if (map[market.symbol]) {
      throw new Error(`Duplicate market params for symbol ${market.symbol}`);
    }
    map[market.symbol] = normalizeMarketParams(market);
  }
  return map;
};
