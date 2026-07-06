import { absBigInt, applyBps, getLeverageConstant, toBigInt } from "./math";
import type {
  NormalizedMarketParams,
  NormalizedMarketParamsBySymbol,
} from "./normalize";
import type {
  MarketMarginResult,
  SubaccountMarginResult,
  TraderMarginResult,
} from "./types";

const QUOTE_LOTS_PER_USD = 1_000_000;
const EPSILON = 1e-12;
const BPS_DENOMINATOR = 10_000;
const U64_MAX_NUMBER = Number(2n ** 64n - 1n);

export interface CalculateLiquidationPriceUsdInput {
  positionSize: number;
  entryPriceUsd: number;
  leverage: number;
  maintenanceMarginBps: number;
  upnlRiskFactorBps?: number;
  collateralUsd: number;
  otherAssetUnrealizedPnlUsd?: number;
  otherAssetMaintenanceMarginUsd?: number;
  /**
   * By default, non-positive liquidation prices return null because they are
   * not actionable prices. Set this to true when callers need the raw equation
   * root for diagnostics or parity checks.
   */
  allowNonPositive?: boolean;
}

export interface MarketLiquidationPriceResult {
  symbol: string;
  basePositionLots: string;
  basePositionUnits: number;
  entryPriceUsd: number;
  liquidationPriceUsd: number | null;
  liquidationPriceTicks: string | null;
}

export interface SubaccountLiquidationPricesResult {
  subaccountIndex: number;
  positions: MarketLiquidationPriceResult[];
}

export interface TraderLiquidationPricesResult {
  authority: string;
  traderPdaIndex: number;
  subaccounts: SubaccountLiquidationPricesResult[];
}

/**
 * Solves the SDK liquidation-price preview equation.
 *
 * Assumptions: target-market limit-order maintenance is supplied as a fixed
 * outside term by callers, soft-stale oracle margin inflation is not modeled,
 * and positive target-market uPnL is discounted with `upnlRiskFactorBps`.
 */
export const calculateLiquidationPriceUsd = ({
  positionSize,
  entryPriceUsd,
  leverage,
  maintenanceMarginBps,
  upnlRiskFactorBps = BPS_DENOMINATOR,
  collateralUsd,
  otherAssetUnrealizedPnlUsd = 0,
  otherAssetMaintenanceMarginUsd = 0,
  allowNonPositive = false,
}: CalculateLiquidationPriceUsdInput): number | null => {
  if (
    Math.abs(positionSize) < EPSILON ||
    leverage <= 0 ||
    maintenanceMarginBps <= 0
  ) {
    return null;
  }

  const price = solveLiquidationPriceUsd({
    positionSize,
    entryPriceUsd,
    leverage,
    maintenanceMarginBps,
    targetUpnlMultiplier: 1,
    collateralUsd,
    otherAssetUnrealizedPnlUsd,
    otherAssetMaintenanceMarginUsd,
  });
  if (price === null) {
    return null;
  }

  const targetUnrealizedPnl = targetUnrealizedPnlUsd(
    positionSize,
    entryPriceUsd,
    price
  );
  const discountedPrice =
    targetUnrealizedPnl > EPSILON && upnlRiskFactorBps < BPS_DENOMINATOR
      ? solveLiquidationPriceUsd({
          positionSize,
          entryPriceUsd,
          leverage,
          maintenanceMarginBps,
          targetUpnlMultiplier: upnlRiskFactorBps / BPS_DENOMINATOR,
          collateralUsd,
          otherAssetUnrealizedPnlUsd,
          otherAssetMaintenanceMarginUsd,
        })
      : price;

  if (
    discountedPrice === null ||
    !Number.isFinite(discountedPrice) ||
    (!allowNonPositive && discountedPrice <= 0)
  ) {
    return null;
  }

  return discountedPrice;
};

const solveLiquidationPriceUsd = ({
  positionSize,
  entryPriceUsd,
  leverage,
  maintenanceMarginBps,
  targetUpnlMultiplier,
  collateralUsd,
  otherAssetUnrealizedPnlUsd,
  otherAssetMaintenanceMarginUsd,
}: Required<
  Pick<
    CalculateLiquidationPriceUsdInput,
    | "positionSize"
    | "entryPriceUsd"
    | "leverage"
    | "maintenanceMarginBps"
    | "collateralUsd"
    | "otherAssetUnrealizedPnlUsd"
    | "otherAssetMaintenanceMarginUsd"
  >
> & { targetUpnlMultiplier: number }): number | null => {
  const maintenanceRatio = maintenanceMarginBps / BPS_DENOMINATOR / leverage;
  const maintenanceCoefficient = Math.abs(positionSize) * maintenanceRatio;
  const numerator =
    collateralUsd +
    otherAssetUnrealizedPnlUsd -
    otherAssetMaintenanceMarginUsd -
    targetUpnlMultiplier * entryPriceUsd * positionSize;
  const denominator =
    maintenanceCoefficient - targetUpnlMultiplier * positionSize;

  if (Math.abs(denominator) < EPSILON) {
    return null;
  }

  const price = numerator / denominator;
  return Number.isFinite(price) ? price : null;
};

const targetUnrealizedPnlUsd = (
  positionSize: number,
  entryPriceUsd: number,
  priceUsd: number
): number => positionSize * (priceUsd - entryPriceUsd);

export const computeSubaccountLiquidationPricesFromMargin = (
  subaccount: SubaccountMarginResult,
  marketsBySymbol: NormalizedMarketParamsBySymbol
): SubaccountLiquidationPricesResult => ({
  subaccountIndex: subaccount.subaccountIndex,
  positions: subaccount.marketMargins.flatMap((market) => {
    const marketParams = marketsBySymbol[market.symbol];
    if (!marketParams) {
      throw new Error(`Missing market params for symbol ${market.symbol}`);
    }

    const liquidationPrice = computeMarketLiquidationPriceFromMargin(
      market,
      subaccount,
      marketParams
    );
    return liquidationPrice ? [liquidationPrice] : [];
  }),
});

export const computeTraderLiquidationPricesFromMargin = (
  trader: TraderMarginResult,
  marketsBySymbol: NormalizedMarketParamsBySymbol
): TraderLiquidationPricesResult => ({
  authority: trader.authority,
  traderPdaIndex: trader.traderPdaIndex,
  subaccounts: trader.subaccounts.map((subaccount) =>
    computeSubaccountLiquidationPricesFromMargin(subaccount, marketsBySymbol)
  ),
});

export const computeMarketLiquidationPriceFromMargin = (
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  marketParams: NormalizedMarketParams
): MarketLiquidationPriceResult | null => {
  const basePositionLots = toBigInt(market.basePositionLots);
  if (basePositionLots === 0n) {
    return null;
  }

  const basePositionUnits = baseLotsToUnits(
    basePositionLots,
    marketParams.baseLotDecimals
  );
  if (
    !Number.isFinite(basePositionUnits) ||
    Math.abs(basePositionUnits) === 0
  ) {
    return null;
  }

  const virtualQuotePositionLots = toBigInt(market.virtualQuotePositionLots);
  validateEntryPriceSignInvariant(
    market.symbol,
    basePositionLots,
    virtualQuotePositionLots
  );

  const entryPriceUsd =
    quoteLotsToUsd(absBigInt(virtualQuotePositionLots)) /
    Math.abs(basePositionUnits);
  const collateralUsd = quoteLotsToUsd(
    toBigInt(subaccount.margin.collateralBalanceQuoteLots) +
      toBigInt(subaccount.margin.unsettledFundingQuoteLots)
  );
  const otherAssetUnrealizedPnlUsd = quoteLotsToUsd(
    toBigInt(subaccount.margin.discountedUnrealizedPnlQuoteLots) -
      toBigInt(market.discountedUnrealizedPnlQuoteLots)
  );
  const otherAssetMaintenanceMarginUsd = quoteLotsToUsd(
    otherAssetMaintenanceMarginQuoteLots(market, subaccount, marketParams)
  );
  const leverage = Number(
    getLeverageConstant(marketParams.leverageTiers, absBigInt(basePositionLots))
  );
  const maintenanceBps = Number(
    marketParams.riskFactors.maintenanceMarginFactorBps
  );

  const liquidationPriceUsd = calculateLiquidationPriceUsd({
    positionSize: basePositionUnits,
    entryPriceUsd,
    leverage,
    maintenanceMarginBps: maintenanceBps,
    upnlRiskFactorBps: Number(marketParams.upnlRiskFactor),
    collateralUsd,
    otherAssetUnrealizedPnlUsd,
    otherAssetMaintenanceMarginUsd,
  });

  return {
    symbol: market.symbol,
    basePositionLots: market.basePositionLots,
    basePositionUnits,
    entryPriceUsd,
    liquidationPriceUsd,
    liquidationPriceTicks:
      liquidationPriceUsd === null
        ? null
        : priceUsdToRoundedTicks(liquidationPriceUsd, marketParams),
  };
};

const otherAssetMaintenanceMarginQuoteLots = (
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  marketParams: NormalizedMarketParams
): bigint => {
  const targetPositionOnlyMaintenanceMargin =
    positionOnlyMaintenanceMarginQuoteLots(market, marketParams);
  const portfolioMaintenanceMargin = toBigInt(
    subaccount.margin.maintenanceMarginQuoteLots
  );

  return checkedSubtractQuoteLots(
    portfolioMaintenanceMargin,
    targetPositionOnlyMaintenanceMargin,
    `Portfolio maintenance margin underflow for symbol ${market.symbol}`
  );
};

const positionOnlyMaintenanceMarginQuoteLots = (
  market: MarketMarginResult,
  marketParams: NormalizedMarketParams
): bigint => {
  const maintenanceBps = marketParams.riskFactors.maintenanceMarginFactorBps;
  const discountedLimitOrderMaintenanceMargin = applyBps(
    toBigInt(market.limitOrderMarginQuoteLots),
    maintenanceBps
  );

  // Mirrors phoenix-state Margin::position_only_maintenance_margin.
  return checkedSubtractQuoteLots(
    toBigInt(market.maintenanceMarginQuoteLots),
    discountedLimitOrderMaintenanceMargin,
    `Position maintenance margin underflow for symbol ${market.symbol}`
  );
};

const checkedSubtractQuoteLots = (
  minuend: bigint,
  subtrahend: bigint,
  underflowMessage: string
): bigint => {
  if (minuend < subtrahend) {
    throw new Error(
      `${underflowMessage}: ${minuend.toString()} < ${subtrahend.toString()}`
    );
  }

  return minuend - subtrahend;
};

const validateEntryPriceSignInvariant = (
  symbol: string,
  basePositionLots: bigint,
  virtualQuotePositionLots: bigint
): void => {
  if (
    (basePositionLots > 0n && virtualQuotePositionLots > 0n) ||
    (basePositionLots < 0n && virtualQuotePositionLots < 0n)
  ) {
    throw new Error(
      `Invalid position sign invariant for ${symbol}: basePositionLots and virtualQuotePositionLots must have opposite signs`
    );
  }
};

const quoteLotsToUsd = (quoteLots: bigint): number =>
  Number(quoteLots) / QUOTE_LOTS_PER_USD;

const baseLotsToUnits = (baseLots: bigint, baseLotDecimals: number): number =>
  Number(baseLots) / Math.pow(10, baseLotDecimals);

const priceUsdToRoundedTicks = (
  priceUsd: number,
  marketParams: NormalizedMarketParams
): string | null => {
  if (!Number.isFinite(priceUsd) || priceUsd <= 0) {
    return null;
  }

  const tickSize = Number(marketParams.tickSize);
  const baseLotsPerBaseUnit = Math.pow(10, marketParams.baseLotDecimals);
  const denominator = tickSize * baseLotsPerBaseUnit;
  if (!Number.isFinite(denominator) || denominator <= 0) {
    return null;
  }

  const ticks = Math.round((priceUsd * QUOTE_LOTS_PER_USD) / denominator);
  if (!Number.isFinite(ticks) || ticks < 0 || ticks > U64_MAX_NUMBER) {
    return null;
  }

  return BigInt(ticks).toString();
};
