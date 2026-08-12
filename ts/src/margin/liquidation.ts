import {
  absBigInt,
  applyBps,
  applyBpsCeil,
  divCeil,
  getLeverageConstant,
  getLimitOrderRiskFactor,
  maxBigInt,
  toBigInt,
} from "./math";
import type {
  NormalizedMarketParams,
  NormalizedMarketParamsBySymbol,
} from "./normalize";
import type {
  MarketMarginInputs,
  MarketMarginResult,
  SpotCollateralMarginResult,
  SubaccountMarginInputs,
  SubaccountMarginResult,
  TraderMarginInputs,
  TraderMarginResult,
} from "./types";
import { notionalSpotCollateral } from "./spotCollateral";

const QUOTE_LOTS_PER_USD = 1_000_000;
const EPSILON = 1e-12;
const BPS_DENOMINATOR = 10_000;
const MAX_HAWKEYE_LIQUIDATION_TICKS = 0xffff_ffffn;

export type LiquidationLimitOrderState = {
  totalNonReduceOnlyAskBaseLots: bigint;
  totalReduceOnlyAskBaseLots: bigint;
  totalNonReduceOnlyBidBaseLots: bigint;
  totalReduceOnlyBidBaseLots: bigint;
  lowestAsk: bigint;
  highestBid: bigint;
};

/**
 * Inputs for the static/current-state liquidation equation.
 *
 * This equation answers the Hawkeye-style question: "at what target-market
 * mark would this current account state become liquidatable if only the mark
 * changed?" It does not simulate resting orders filling before that boundary.
 */
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
   * Target-market limit-order maintenance in USD per $1 move in the target
   * mark price. This keeps same-market order margin price-dependent while the
   * root is solved.
   */
  targetLimitOrderMaintenanceCoefficient?: number;
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
 * Solves the static/current-state SDK liquidation-price preview equation.
 *
 * This is the closed-form version of the Hawkeye-compatible boundary used by
 * {@link computeMarketLiquidationPriceFromMargin}. Use it when integrators
 * need the liquidation price for the current account state, such as API
 * snapshots, parity checks, or liquidation eligibility previews.
 *
 * This function does not mutate the account state while the price moves. In
 * particular, target-market resting orders remain limit-order exposure only;
 * they are not filled, removed, or used to change entry price/collateral. For
 * "what if my orders fill before liquidation?" displays, use
 * {@link computeProjectedLiquidation} or
 * {@link computeSubaccountProjectedLiquidationFromMargin} alongside the static
 * value.
 *
 * Assumptions: target-market limit-order maintenance is linear in the target
 * mark price over the solved regime, soft-stale oracle margin inflation is not
 * modeled, and positive target-market uPnL is discounted with
 * `upnlRiskFactorBps`.
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
  targetLimitOrderMaintenanceCoefficient = 0,
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
    targetLimitOrderMaintenanceCoefficient,
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
          targetLimitOrderMaintenanceCoefficient,
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
  targetLimitOrderMaintenanceCoefficient,
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
    | "targetLimitOrderMaintenanceCoefficient"
  >
> & { targetUpnlMultiplier: number }): number | null => {
  if (
    !Number.isFinite(targetLimitOrderMaintenanceCoefficient) ||
    targetLimitOrderMaintenanceCoefficient < 0
  ) {
    return null;
  }

  const maintenanceRatio = maintenanceMarginBps / BPS_DENOMINATOR / leverage;
  const maintenanceCoefficient =
    Math.abs(positionSize) * maintenanceRatio +
    targetLimitOrderMaintenanceCoefficient;
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

/**
 * Computes Hawkeye-compatible static liquidation prices for a subaccount.
 *
 * These prices use the current margin snapshot and do not simulate any future
 * fills. They are the canonical value to compare against program/Hawkeye
 * output for the current state.
 */
export const computeSubaccountLiquidationPricesFromMargin = (
  subaccount: SubaccountMarginResult,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  inputs?: SubaccountMarginInputs
): SubaccountLiquidationPricesResult => {
  const inputsBySymbol = new Map(
    inputs?.markets.map((input) => [input.symbol, input])
  );
  return {
    subaccountIndex: subaccount.subaccountIndex,
    positions: subaccount.marketMargins.flatMap((market) => {
      const marketParams = marketsBySymbol[market.symbol];
      if (!marketParams) {
        throw new Error(`Missing market params for symbol ${market.symbol}`);
      }

      const liquidationPrice = computeMarketLiquidationPriceFromMargin(
        market,
        subaccount,
        marketParams,
        inputsBySymbol.get(market.symbol)
      );
      return liquidationPrice ? [liquidationPrice] : [];
    }),
  };
};

/**
 * Computes Hawkeye-compatible static liquidation prices for every subaccount
 * in a trader margin result.
 */
export const computeTraderLiquidationPricesFromMargin = (
  trader: TraderMarginResult,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  inputs?: TraderMarginInputs
): TraderLiquidationPricesResult => {
  const inputsBySubaccount = new Map(
    inputs?.subaccounts.map((input) => [input.subaccountIndex, input])
  );
  return {
    authority: trader.authority,
    traderPdaIndex: trader.traderPdaIndex,
    subaccounts: trader.subaccounts.map((subaccount) =>
      computeSubaccountLiquidationPricesFromMargin(
        subaccount,
        marketsBySymbol,
        inputsBySubaccount.get(subaccount.subaccountIndex)
      )
    ),
  };
};

/**
 * Computes one market's static/current-state liquidation price from a margin
 * result.
 *
 * The returned boundary assumes only the target-market mark changes. Resting
 * orders affect current maintenance margin through the supplied margin result,
 * but they do not fill along the price path.
 */
export const computeMarketLiquidationPriceFromMargin = (
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  marketParams: NormalizedMarketParams,
  marketInput?: MarketMarginInputs
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
  const currentMarkTicks = toBigInt(marketParams.markPriceTicks);

  if (
    isLiquidatableAtTicks(
      currentMarkTicks,
      market,
      subaccount,
      marketParams,
      marketInput
    )
  ) {
    return {
      symbol: market.symbol,
      basePositionLots: market.basePositionLots,
      basePositionUnits,
      entryPriceUsd,
      liquidationPriceUsd: markPriceUsdFromParams(marketParams),
      liquidationPriceTicks: currentMarkTicks.toString(),
    };
  }

  const liquidationPriceTicks = findLiquidationBoundaryTicks(
    market,
    subaccount,
    marketParams,
    marketInput
  );
  const liquidationPriceUsd =
    liquidationPriceTicks === null
      ? null
      : ticksToPriceUsd(liquidationPriceTicks, marketParams);

  return {
    symbol: market.symbol,
    basePositionLots: market.basePositionLots,
    basePositionUnits,
    entryPriceUsd,
    liquidationPriceUsd,
    liquidationPriceTicks: liquidationPriceTicks?.toString() ?? null,
  };
};

const otherMarketMaintenanceMarginQuoteLots = (
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult
): bigint => {
  const portfolioMaintenanceMargin = toBigInt(
    subaccount.margin.maintenanceMarginQuoteLots
  );

  return checkedSubtractQuoteLots(
    portfolioMaintenanceMargin,
    toBigInt(market.maintenanceMarginQuoteLots),
    `Portfolio maintenance margin underflow for symbol ${market.symbol}`
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

const ticksToPriceUsd = (
  ticks: bigint,
  marketParams: NormalizedMarketParams
): number => {
  const markPriceQuoteLotsPerBaseLot = Number(
    ticks * toBigInt(marketParams.tickSize)
  );
  return (
    (markPriceQuoteLotsPerBaseLot *
      Math.pow(10, marketParams.baseLotDecimals)) /
    QUOTE_LOTS_PER_USD
  );
};

const markPriceUsdFromParams = (marketParams: NormalizedMarketParams): number =>
  ticksToPriceUsd(toBigInt(marketParams.markPriceTicks), marketParams);

const findLiquidationBoundaryTicks = (
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  marketParams: NormalizedMarketParams,
  marketInput?: MarketMarginInputs
): bigint | null => {
  const basePositionLots = toBigInt(market.basePositionLots);
  if (basePositionLots === 0n) {
    return null;
  }

  const currentMarkTicks = toBigInt(marketParams.markPriceTicks);
  if (currentMarkTicks <= 0n) {
    return null;
  }

  if (basePositionLots > 0n) {
    if (
      !isLiquidatableAtTicks(0n, market, subaccount, marketParams, marketInput)
    ) {
      return null;
    }

    let low = 0n;
    let high = currentMarkTicks;
    while (low + 1n < high) {
      const mid = low + (high - low) / 2n;
      if (
        isLiquidatableAtTicks(
          mid,
          market,
          subaccount,
          marketParams,
          marketInput
        )
      ) {
        low = mid;
      } else {
        high = mid;
      }
    }

    return low;
  }

  if (
    !isLiquidatableAtTicks(
      MAX_HAWKEYE_LIQUIDATION_TICKS,
      market,
      subaccount,
      marketParams,
      marketInput
    )
  ) {
    return null;
  }

  let low = currentMarkTicks;
  let high = MAX_HAWKEYE_LIQUIDATION_TICKS;
  while (low + 1n < high) {
    const mid = low + (high - low) / 2n;
    if (
      isLiquidatableAtTicks(mid, market, subaccount, marketParams, marketInput)
    ) {
      high = mid;
    } else {
      low = mid;
    }
  }

  return high;
};

const isLiquidatableAtTicks = (
  ticks: bigint,
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  marketParams: NormalizedMarketParams,
  marketInput?: MarketMarginInputs
): boolean => {
  const basePositionLots = toBigInt(market.basePositionLots);
  const virtualQuotePositionLots = toBigInt(market.virtualQuotePositionLots);
  const tickSize = toBigInt(marketParams.tickSize);
  const rawTargetPnl =
    virtualQuotePositionLots + basePositionLots * ticks * tickSize;
  const upnlRiskFactor = toBigInt(marketParams.upnlRiskFactor);
  const targetDiscountedPnl =
    rawTargetPnl > 0n
      ? applyBpsCeil(rawTargetPnl, upnlRiskFactor)
      : rawTargetPnl;
  const spotCollateral = spotCollateralValueAtTicks(
    ticks,
    market.symbol,
    subaccount.spotCollaterals ?? [],
    toBigInt(marketParams.tickSize)
  );
  const effectiveCollateral =
    toBigInt(subaccount.margin.collateralBalanceQuoteLots) +
    toBigInt(subaccount.margin.unsettledFundingQuoteLots) +
    (toBigInt(subaccount.margin.discountedUnrealizedPnlQuoteLots) -
      toBigInt(market.discountedUnrealizedPnlQuoteLots)) +
    targetDiscountedPnl +
    spotCollateral;
  if (effectiveCollateral < 0n) {
    return true;
  }

  const targetMaintenance = maintenanceMarginAtTicks(
    ticks,
    market,
    subaccount,
    marketParams,
    marketInput
  );
  const maintenance =
    otherMarketMaintenanceMarginQuoteLots(market, subaccount) +
    targetMaintenance;
  return effectiveCollateral < maintenance;
};

const spotCollateralValueAtTicks = (
  ticks: bigint,
  targetMarketSymbol: string,
  spotCollaterals: readonly SpotCollateralMarginResult[],
  tickSize: bigint
): bigint => {
  let total = 0n;
  const priceQuoteLotsPerBaseLot = ticks * tickSize;
  for (const spot of spotCollaterals) {
    if (spot.pricingMarketSymbol === targetMarketSymbol) {
      const nativeUnitsPerBaseLot = toBigInt(spot.nativeUnitsPerBaseLot);
      if (nativeUnitsPerBaseLot === 0n) {
        throw new Error(
          `Spot collateral nativeUnitsPerBaseLot for ${spot.symbol} must be positive`
        );
      }
      const notional = notionalSpotCollateral(
        { priceQuoteLotsPerBaseLot, nativeUnitsPerBaseLot },
        toBigInt(spot.balance)
      );
      total += applyBps(notional, toBigInt(spot.retainedBps));
    } else {
      total += toBigInt(spot.discountedQuoteLots);
    }
  }
  return total;
};

const maintenanceMarginAtTicks = (
  ticks: bigint,
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  marketParams: NormalizedMarketParams,
  marketInput?: MarketMarginInputs
): bigint => {
  const basePositionLots = toBigInt(market.basePositionLots);
  const inputLimitOrderState = marketInput?.limitOrderMargin;
  const limitOrderState = inputLimitOrderState
    ? {
        totalNonReduceOnlyAskBaseLots: toBigInt(
          inputLimitOrderState.totalNonReduceOnlyAskBaseLots
        ),
        totalReduceOnlyAskBaseLots: toBigInt(
          inputLimitOrderState.totalReduceOnlyAskBaseLots
        ),
        totalNonReduceOnlyBidBaseLots: toBigInt(
          inputLimitOrderState.totalNonReduceOnlyBidBaseLots
        ),
        totalReduceOnlyBidBaseLots: toBigInt(
          inputLimitOrderState.totalReduceOnlyBidBaseLots
        ),
        lowestAsk: toBigInt(inputLimitOrderState.lowestAsk),
        highestBid: toBigInt(inputLimitOrderState.highestBid),
      }
    : aggregateLimitOrderState(
        market,
        subaccount,
        toBigInt(marketParams.markPriceTicks)
      );
  const initialMargin = initialMarginForAssetAtTicks(
    basePositionLots,
    limitOrderState,
    ticks,
    marketParams
  );
  const maintenanceBps = toBigInt(
    marketParams.riskFactors.maintenanceMarginFactorBps
  );
  return applyBps(initialMargin, maintenanceBps);
};

const aggregateLimitOrderState = (
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  markPriceTicks: bigint
): LiquidationLimitOrderState =>
  aggregateLimitOrderStateFromOrders(
    subaccount.limitOrders
      .filter((order) => order.symbol === market.symbol)
      .map((order) => ({
        side: order.side,
        priceTicks: toBigInt(order.priceTicks),
        baseLotsRemaining: toBigInt(order.tradeSizeRemainingLots),
        reduceOnly: order.reduceOnly,
      })),
    toBigInt(market.basePositionLots),
    markPriceTicks
  );

/**
 * Aggregates resting orders into the limit-order margin state used by the
 * liquidation predicates. Best-price sentinels are built relative to the
 * given mark: only orders that could cross at that mark reserve fill-loss
 * margin, mirroring the canonical adverse-only sentinel semantics.
 */
export const aggregateLimitOrderStateFromOrders = (
  orders: ProjectedLiquidationOrderInput[],
  basePositionLots: bigint,
  markPriceTicks: bigint
): LiquidationLimitOrderState => {
  let totalNonReduceOnlyAskBaseLots = 0n;
  let totalReduceOnlyAskBaseLots = 0n;
  let totalNonReduceOnlyBidBaseLots = 0n;
  let totalReduceOnlyBidBaseLots = 0n;
  let lowestAsk = 0n;
  let highestBid = 0n;

  for (const order of orders) {
    const size = order.baseLotsRemaining;
    const priceTicks = order.priceTicks;
    if (order.side === "ask") {
      if (priceTicks < markPriceTicks) {
        lowestAsk =
          lowestAsk === 0n ? priceTicks : minBigInt(lowestAsk, priceTicks);
      }
      if (order.reduceOnly) {
        totalReduceOnlyAskBaseLots += size;
      } else {
        totalNonReduceOnlyAskBaseLots += size;
      }
    } else {
      if (priceTicks > markPriceTicks) {
        highestBid = maxBigInt(highestBid, priceTicks);
      }
      if (order.reduceOnly) {
        totalReduceOnlyBidBaseLots += size;
      } else {
        totalNonReduceOnlyBidBaseLots += size;
      }
    }
  }

  if (basePositionLots >= 0n) {
    totalReduceOnlyAskBaseLots = minBigInt(
      totalReduceOnlyAskBaseLots,
      basePositionLots
    );
    totalReduceOnlyBidBaseLots = 0n;
  } else {
    totalReduceOnlyAskBaseLots = 0n;
    totalReduceOnlyBidBaseLots = minBigInt(
      totalReduceOnlyBidBaseLots,
      absBigInt(basePositionLots)
    );
  }

  return {
    totalNonReduceOnlyAskBaseLots,
    totalReduceOnlyAskBaseLots,
    totalNonReduceOnlyBidBaseLots,
    totalReduceOnlyBidBaseLots,
    lowestAsk,
    highestBid,
  };
};

const initialMarginForAssetAtTicks = (
  position: bigint,
  limitOrderState: LiquidationLimitOrderState,
  ticks: bigint,
  marketParams: NormalizedMarketParams
): bigint => {
  const totalBid = limitOrderState.totalNonReduceOnlyBidBaseLots;
  const totalAsk = limitOrderState.totalNonReduceOnlyAskBaseLots;
  const totalBidAll = totalBid + limitOrderState.totalReduceOnlyBidBaseLots;
  const totalAskAll = totalAsk + limitOrderState.totalReduceOnlyAskBaseLots;
  if (
    position === 0n &&
    totalBid === 0n &&
    totalAsk === 0n &&
    totalBidAll === 0n &&
    totalAskAll === 0n
  ) {
    return 0n;
  }

  const tickSize = toBigInt(marketParams.tickSize);
  const assetUnitPrice = ticks * tickSize;
  let collateralRequired = 0n;
  let existingPositionMarginOffset = 0n;

  if (position !== 0n) {
    const absolutePositionSize = absBigInt(position);
    const absoluteBookValue = assetUnitPrice * absolutePositionSize;
    const leverage = getLeverageConstant(
      marketParams.leverageTiers,
      absolutePositionSize
    );
    const leverageBasedMargin = divCeil(absoluteBookValue, leverage);
    collateralRequired += leverageBasedMargin;
    existingPositionMarginOffset = leverageBasedMargin;
  }

  const marginBid =
    totalBid > 0n
      ? marginIncreaseForBids(
          position,
          totalBid,
          assetUnitPrice,
          marketParams.leverageTiers,
          existingPositionMarginOffset
        )
      : 0n;
  const marginAsk =
    totalAsk > 0n
      ? marginIncreaseForAsks(
          position,
          totalAsk,
          assetUnitPrice,
          marketParams.leverageTiers,
          existingPositionMarginOffset
        )
      : 0n;

  collateralRequired += maxBigInt(marginBid, marginAsk);
  collateralRequired +=
    totalBidAll > 0n
      ? limitOrderFillLoss(
          "bid",
          totalBidAll,
          limitOrderState.highestBid,
          assetUnitPrice,
          tickSize
        )
      : 0n;
  collateralRequired +=
    totalAskAll > 0n
      ? limitOrderFillLoss(
          "ask",
          totalAskAll,
          limitOrderState.lowestAsk,
          assetUnitPrice,
          tickSize
        )
      : 0n;

  return collateralRequired;
};

const marginIncreaseForBids = (
  position: bigint,
  bidSize: bigint,
  assetUnitPrice: bigint,
  tiers: NormalizedMarketParams["leverageTiers"],
  existingPositionMarginOffset: bigint
): bigint => {
  const newExposureSigned = bidSize + position - absBigInt(position);
  if (newExposureSigned <= 0n) {
    return 0n;
  }

  const totalExposure = absBigInt(position + bidSize);
  const totalGrossValue = assetUnitPrice * totalExposure;
  const totalLeverage = getLeverageConstant(tiers, totalExposure);
  const totalMargin = divCeil(totalGrossValue, totalLeverage);
  const incrementalMargin = maxBigInt(
    totalMargin - existingPositionMarginOffset,
    0n
  );
  return applyBpsCeil(
    incrementalMargin,
    getLimitOrderRiskFactor(tiers, totalExposure)
  );
};

const marginIncreaseForAsks = (
  position: bigint,
  askSize: bigint,
  assetUnitPrice: bigint,
  tiers: NormalizedMarketParams["leverageTiers"],
  existingPositionMarginOffset: bigint
): bigint => {
  const newExposureSigned = askSize - position - absBigInt(position);
  if (newExposureSigned <= 0n) {
    return 0n;
  }

  const totalExposure = absBigInt(position - askSize);
  const totalGrossValue = assetUnitPrice * totalExposure;
  const totalLeverage = getLeverageConstant(tiers, totalExposure);
  const totalMargin = divCeil(totalGrossValue, totalLeverage);
  const incrementalMargin = maxBigInt(
    totalMargin - existingPositionMarginOffset,
    0n
  );
  return applyBpsCeil(
    incrementalMargin,
    getLimitOrderRiskFactor(tiers, totalExposure)
  );
};

const limitOrderFillLoss = (
  side: "bid" | "ask",
  size: bigint,
  limitPriceTicks: bigint,
  assetUnitPrice: bigint,
  tickSize: bigint
): bigint => {
  if (limitPriceTicks === 0n) {
    return 0n;
  }
  const limitPrice = limitPriceTicks * tickSize;
  if (side === "bid") {
    return limitPrice > assetUnitPrice
      ? size * (limitPrice - assetUnitPrice)
      : 0n;
  }
  return limitPrice < assetUnitPrice
    ? size * (assetUnitPrice - limitPrice)
    : 0n;
};

const minBigInt = (a: bigint, b: bigint): bigint => (a < b ? a : b);

const I64_MIN = -(2n ** 63n);
const I64_MAX = 2n ** 63n - 1n;

/**
 * A resting order considered by the projected liquidation path.
 *
 * Only visible target-market orders are needed here. The projected path will
 * consider non-reduce-only orders that increase the current position direction
 * and will ignore reduce-only or opposite-side orders.
 */
export interface ProjectedLiquidationOrderInput {
  side: "bid" | "ask";
  priceTicks: bigint;
  baseLotsRemaining: bigint;
  reduceOnly: boolean;
}

/** A resting-order fill applied along the projected liquidation path. */
export interface ProjectedLiquidationFill {
  side: "bid" | "ask";
  priceTicks: bigint;
  baseLots: bigint;
}

/**
 * Result of the projected (path-dependent) liquidation price calculation.
 *
 * `liquidationPriceTicks` is `null` when the account cannot become
 * liquidatable at any tick in the adverse direction, even after every
 * projected fill is applied. `staticLiquidationPriceTicks` is the ordinary
 * current-state boundary for the same inputs, computed as the projection's
 * first path segment.
 *
 * When `fills` is empty, the projected and static boundaries are the same.
 * When `fills` is non-empty, `liquidationPriceTicks` is a scenario estimate for
 * the post-fill account state and should be displayed as projected risk rather
 * than as the canonical Hawkeye liquidation price.
 */
export interface ProjectedLiquidationResult {
  liquidationPriceTicks: bigint | null;
  staticLiquidationPriceTicks: bigint | null;
  /** Fills applied before the boundary, in path order. */
  fills: ProjectedLiquidationFill[];
}

/**
 * Inputs for {@link computeProjectedLiquidation}.
 *
 * The portfolio fields carry the same aggregates the static liquidation search
 * uses; the target fields are the target market's contribution to those
 * aggregates at the current mark. The first path segment therefore starts from
 * the same account state as {@link computeMarketLiquidationPriceFromMargin}.
 */
export interface ProjectedLiquidationInput {
  basePositionLots: bigint;
  virtualQuotePositionLots: bigint;
  /**
   * Target-market limit-order margin state for the current state (canonical
   * when available).
   */
  limitOrderState: LiquidationLimitOrderState;
  /** The trader's visible resting orders in the target market. */
  visibleOrders: ProjectedLiquidationOrderInput[];
  collateralBalanceQuoteLots: bigint;
  portfolioUnsettledFundingQuoteLots: bigint;
  portfolioDiscountedUnrealizedPnlQuoteLots: bigint;
  portfolioMaintenanceMarginQuoteLots: bigint;
  targetDiscountedUnrealizedPnlQuoteLots: bigint;
  targetMaintenanceMarginQuoteLots: bigint;
  /** Valued spot collateral; target-market assets reprice along the path. */
  spotCollaterals?: readonly SpotCollateralMarginResult[];
}

/**
 * Computes a projected liquidation price by re-solving the static boundary
 * after simulating the trader's own risk-increasing resting orders filling
 * along the adverse price path.
 *
 * This answers a different question than the static/Hawkeye liquidation price.
 * Static liquidation asks where the current state becomes liquidatable if only
 * the mark changes. Projected liquidation asks where the trader would become
 * liquidatable if the mark trades through their position-side resting orders,
 * those orders fill at their limit prices, and each post-fill state is
 * re-evaluated.
 *
 * Use the projected value as a risk/explainer estimate for dashboards,
 * portfolio tools, and pre-trade education when resting orders can materially
 * change the future position. Do not use it as the source of truth for current
 * liquidation eligibility or for parity with Hawkeye/program output; keep the
 * static value visible for that.
 *
 * The path model is deliberately minimal and deterministic:
 * - Only non-reduce-only orders on the position side fill (bids for a long,
 *   asks for a short), fully and exactly at their limit price, most adverse
 *   first. Fees, slippage, contra liquidity, trigger orders, and
 *   opposite-side (risk-reducing) fills are not modeled.
 * - Fills extend the position in its current direction, so no PnL is
 *   realized and collateral stays constant; funding is treated as settled at
 *   the current rate.
 * - Non-target markets stay fixed at their snapshot aggregates, exactly like
 *   the static liquidation search.
 *
 * When no orders would fill before the boundary this returns the static
 * result.
 */
export const computeProjectedLiquidation = (
  input: ProjectedLiquidationInput,
  marketParams: NormalizedMarketParams
): ProjectedLiquidationResult => {
  const fills: ProjectedLiquidationFill[] = [];
  let basePositionLots = input.basePositionLots;
  let virtualQuotePositionLots = input.virtualQuotePositionLots;
  if (basePositionLots === 0n) {
    return {
      liquidationPriceTicks: null,
      staticLiquidationPriceTicks: null,
      fills,
    };
  }
  const isLong = basePositionLots > 0n;
  let simMarkTicks = toBigInt(marketParams.markPriceTicks);
  if (simMarkTicks <= 0n) {
    return {
      liquidationPriceTicks: null,
      staticLiquidationPriceTicks: null,
      fills,
    };
  }

  const consumed = input.visibleOrders.map(() => false);
  const fillOrder = input.visibleOrders
    .map((_, index) => index)
    .filter((index) => {
      const order = input.visibleOrders[index];
      return !order.reduceOnly && (order.side === "bid" ? isLong : !isLong);
    })
    // Most adverse first: highest bid for a long, lowest ask for a short.
    .sort((a, b) => {
      const priceA = input.visibleOrders[a].priceTicks;
      const priceB = input.visibleOrders[b].priceTicks;
      const ordered = isLong ? priceB - priceA : priceA - priceB;
      return ordered > 0n ? 1 : ordered < 0n ? -1 : 0;
    });

  let limitOrderState = input.limitOrderState;
  let staticLiquidationPriceTicks: bigint | null = null;
  let nextFill = 0;
  for (;;) {
    if (
      isProjectedLiquidatableAtTicks(
        simMarkTicks,
        basePositionLots,
        virtualQuotePositionLots,
        limitOrderState,
        input,
        marketParams
      )
    ) {
      return {
        liquidationPriceTicks: simMarkTicks,
        staticLiquidationPriceTicks:
          nextFill === 0 ? simMarkTicks : staticLiquidationPriceTicks,
        fills,
      };
    }

    const boundary = findProjectedLiquidationBoundaryTicks(
      basePositionLots,
      virtualQuotePositionLots,
      limitOrderState,
      simMarkTicks,
      input,
      marketParams
    );
    if (nextFill === 0) {
      staticLiquidationPriceTicks = boundary;
    }
    if (nextFill >= fillOrder.length) {
      return {
        liquidationPriceTicks: boundary,
        staticLiquidationPriceTicks,
        fills,
      };
    }
    const order = input.visibleOrders[fillOrder[nextFill]];
    if (boundary !== null) {
      const boundaryBeforeFill = isLong
        ? boundary >= order.priceTicks
        : boundary <= order.priceTicks;
      if (boundaryBeforeFill) {
        return {
          liquidationPriceTicks: boundary,
          staticLiquidationPriceTicks,
          fills,
        };
      }
    }

    // Fill the order fully at its limit price. Projected fills are always on
    // the position side, so the position grows in place and no PnL is
    // realized.
    const deltaBaseLots =
      order.side === "bid" ? order.baseLotsRemaining : -order.baseLotsRemaining;
    const fillUnitPrice = order.priceTicks * toBigInt(marketParams.tickSize);
    basePositionLots += deltaBaseLots;
    virtualQuotePositionLots -= deltaBaseLots * fillUnitPrice;
    if (
      basePositionLots < I64_MIN ||
      basePositionLots > I64_MAX ||
      virtualQuotePositionLots < I64_MIN ||
      virtualQuotePositionLots > I64_MAX
    ) {
      // Mirror the Rust engines, which report no boundary when the projected
      // position overflows the on-chain integer domain.
      return {
        liquidationPriceTicks: null,
        staticLiquidationPriceTicks,
        fills,
      };
    }
    consumed[fillOrder[nextFill]] = true;
    fills.push({
      side: order.side,
      priceTicks: order.priceTicks,
      baseLots: order.baseLotsRemaining,
    });
    nextFill += 1;
    simMarkTicks = order.priceTicks;
    limitOrderState = aggregateLimitOrderStateFromOrders(
      input.visibleOrders.filter((_, index) => !consumed[index]),
      basePositionLots,
      simMarkTicks
    );
  }
};

const isProjectedLiquidatableAtTicks = (
  ticks: bigint,
  basePositionLots: bigint,
  virtualQuotePositionLots: bigint,
  limitOrderState: LiquidationLimitOrderState,
  input: ProjectedLiquidationInput,
  marketParams: NormalizedMarketParams
): boolean => {
  const tickSize = toBigInt(marketParams.tickSize);
  const rawTargetPnl =
    virtualQuotePositionLots + basePositionLots * ticks * tickSize;
  const upnlRiskFactor = toBigInt(marketParams.upnlRiskFactor);
  const targetDiscountedPnl =
    rawTargetPnl > 0n
      ? applyBpsCeil(rawTargetPnl, upnlRiskFactor)
      : rawTargetPnl;
  const spotCollateral = spotCollateralValueAtTicks(
    ticks,
    marketParams.symbol,
    input.spotCollaterals ?? [],
    tickSize
  );
  const effectiveCollateral =
    input.collateralBalanceQuoteLots +
    input.portfolioUnsettledFundingQuoteLots +
    (input.portfolioDiscountedUnrealizedPnlQuoteLots -
      input.targetDiscountedUnrealizedPnlQuoteLots) +
    targetDiscountedPnl +
    spotCollateral;
  if (effectiveCollateral < 0n) {
    return true;
  }

  const otherMaintenanceMargin = checkedSubtractQuoteLots(
    input.portfolioMaintenanceMarginQuoteLots,
    input.targetMaintenanceMarginQuoteLots,
    "Portfolio maintenance margin underflow in projected liquidation"
  );
  const targetMaintenance = applyBps(
    initialMarginForAssetAtTicks(
      basePositionLots,
      limitOrderState,
      ticks,
      marketParams
    ),
    toBigInt(marketParams.riskFactors.maintenanceMarginFactorBps)
  );
  return effectiveCollateral < otherMaintenanceMargin + targetMaintenance;
};

const findProjectedLiquidationBoundaryTicks = (
  basePositionLots: bigint,
  virtualQuotePositionLots: bigint,
  limitOrderState: LiquidationLimitOrderState,
  currentMarkTicks: bigint,
  input: ProjectedLiquidationInput,
  marketParams: NormalizedMarketParams
): bigint | null => {
  const liquidatable = (ticks: bigint): boolean =>
    isProjectedLiquidatableAtTicks(
      ticks,
      basePositionLots,
      virtualQuotePositionLots,
      limitOrderState,
      input,
      marketParams
    );

  if (basePositionLots > 0n) {
    if (!liquidatable(0n)) {
      return null;
    }

    let low = 0n;
    let high = currentMarkTicks;
    while (low + 1n < high) {
      const mid = low + (high - low) / 2n;
      if (liquidatable(mid)) {
        low = mid;
      } else {
        high = mid;
      }
    }

    return low;
  }

  if (!liquidatable(MAX_HAWKEYE_LIQUIDATION_TICKS)) {
    return null;
  }

  let low = currentMarkTicks;
  let high = MAX_HAWKEYE_LIQUIDATION_TICKS;
  while (low + 1n < high) {
    const mid = low + (high - low) / 2n;
    if (liquidatable(mid)) {
      high = mid;
    } else {
      low = mid;
    }
  }

  return high;
};

/** Per-market projected liquidation result with display USD at the edge. */
export interface MarketProjectedLiquidationResult {
  symbol: string;
  basePositionLots: string;
  liquidationPriceUsd: number | null;
  liquidationPriceTicks: string | null;
  staticLiquidationPriceTicks: string | null;
  fills: ProjectedLiquidationFill[];
}

export interface SubaccountProjectedLiquidationResult {
  subaccountIndex: number;
  positions: MarketProjectedLiquidationResult[];
}

/**
 * Computes the projected liquidation price for one market from computed margin
 * results, mirroring {@link computeMarketLiquidationPriceFromMargin}.
 *
 * Use this alongside the static market liquidation price to show the difference
 * between "current state" and "position-side orders fill first" risk. Returns
 * `null` for markets without a position.
 */
export const computeMarketProjectedLiquidationFromMargin = (
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  marketParams: NormalizedMarketParams,
  compute: (
    input: ProjectedLiquidationInput,
    marketParams: NormalizedMarketParams
  ) => ProjectedLiquidationResult = computeProjectedLiquidation
): MarketProjectedLiquidationResult | null => {
  const input = buildProjectedLiquidationInput(
    market,
    subaccount,
    marketParams
  );
  if (input === null) {
    return null;
  }

  const result = compute(input, marketParams);
  return {
    symbol: market.symbol,
    basePositionLots: market.basePositionLots,
    liquidationPriceUsd:
      result.liquidationPriceTicks === null
        ? null
        : projectedTicksToPriceUsd(result.liquidationPriceTicks, marketParams),
    liquidationPriceTicks: result.liquidationPriceTicks?.toString() ?? null,
    staticLiquidationPriceTicks:
      result.staticLiquidationPriceTicks?.toString() ?? null,
    fills: result.fills,
  };
};

/**
 * Computes projected liquidation prices for every market with a position in
 * the subaccount, mirroring
 * {@link computeSubaccountLiquidationPricesFromMargin}.
 *
 * These values are scenario estimates and are additive to the static prices,
 * not replacements. Pass a memoized `compute` from
 * {@link createProjectedLiquidationCalculator} to reuse results across small
 * target-market mark moves.
 */
export const computeSubaccountProjectedLiquidationFromMargin = (
  subaccount: SubaccountMarginResult,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  compute?: (
    input: ProjectedLiquidationInput,
    marketParams: NormalizedMarketParams
  ) => ProjectedLiquidationResult
): SubaccountProjectedLiquidationResult => ({
  subaccountIndex: subaccount.subaccountIndex,
  positions: subaccount.marketMargins.flatMap((market) => {
    const marketParams = marketsBySymbol[market.symbol];
    if (!marketParams) {
      throw new Error(`Missing market params for symbol ${market.symbol}`);
    }

    const projected = computeMarketProjectedLiquidationFromMargin(
      market,
      subaccount,
      marketParams,
      compute
    );
    return projected ? [projected] : [];
  }),
});

const buildProjectedLiquidationInput = (
  market: MarketMarginResult,
  subaccount: SubaccountMarginResult,
  marketParams: NormalizedMarketParams
): ProjectedLiquidationInput | null => {
  const basePositionLots = toBigInt(market.basePositionLots);
  if (basePositionLots === 0n) {
    return null;
  }

  const markPriceTicks = toBigInt(marketParams.markPriceTicks);
  const visibleOrders: ProjectedLiquidationOrderInput[] = subaccount.limitOrders
    .filter((order) => order.symbol === market.symbol)
    .map((order) => ({
      side: order.side,
      priceTicks: toBigInt(order.priceTicks),
      baseLotsRemaining: toBigInt(order.tradeSizeRemainingLots),
      reduceOnly: order.reduceOnly,
    }));

  return {
    basePositionLots,
    virtualQuotePositionLots: toBigInt(market.virtualQuotePositionLots),
    limitOrderState: aggregateLimitOrderStateFromOrders(
      visibleOrders,
      basePositionLots,
      markPriceTicks
    ),
    visibleOrders,
    collateralBalanceQuoteLots: toBigInt(
      subaccount.margin.collateralBalanceQuoteLots
    ),
    portfolioUnsettledFundingQuoteLots: toBigInt(
      subaccount.margin.unsettledFundingQuoteLots
    ),
    portfolioDiscountedUnrealizedPnlQuoteLots: toBigInt(
      subaccount.margin.discountedUnrealizedPnlQuoteLots
    ),
    portfolioMaintenanceMarginQuoteLots: toBigInt(
      subaccount.margin.maintenanceMarginQuoteLots
    ),
    targetDiscountedUnrealizedPnlQuoteLots: toBigInt(
      market.discountedUnrealizedPnlQuoteLots
    ),
    targetMaintenanceMarginQuoteLots: toBigInt(
      market.maintenanceMarginQuoteLots
    ),
    spotCollaterals: subaccount.spotCollaterals,
  };
};

const projectedTicksToPriceUsd = (
  ticks: bigint,
  marketParams: NormalizedMarketParams
): number => {
  const quoteLotsPerBaseLot = Number(ticks * toBigInt(marketParams.tickSize));
  return (
    (quoteLotsPerBaseLot * Math.pow(10, marketParams.baseLotDecimals)) /
    QUOTE_LOTS_PER_USD
  );
};

/**
 * Creates a memoizing wrapper around {@link computeProjectedLiquidation}.
 *
 * This is useful for live dashboards that display projected liquidation
 * alongside mark updates. It avoids re-running the path simulation while the
 * account state is unchanged and the mark remains on the safe side of the
 * static boundary.
 *
 * The projected boundary is a function of the account state, not of the
 * target market's mark: while the account stays non-liquidatable at the
 * current mark, a target-market mark move changes neither the fill ladder
 * nor any path segment's boundary. Cached results are therefore keyed on
 * every input except the mark and reused while the mark stays on the safe
 * side of the cached static boundary. Any change to positions, orders,
 * collateral, funding, or other-market aggregates changes the key and
 * recomputes.
 */
export const createProjectedLiquidationCalculator = (options?: {
  maxCacheEntries?: number;
}): ((
  input: ProjectedLiquidationInput,
  marketParams: NormalizedMarketParams
) => ProjectedLiquidationResult) => {
  const maxCacheEntries = options?.maxCacheEntries ?? 128;
  const cache = new Map<string, ProjectedLiquidationResult>();

  return (input, marketParams) => {
    const key = projectedLiquidationCacheKey(input, marketParams);
    const cached = cache.get(key);
    if (cached) {
      const markTicks = toBigInt(marketParams.markPriceTicks);
      const boundary = cached.staticLiquidationPriceTicks;
      const markStillSafe =
        boundary === null
          ? markTicks > 0n
          : input.basePositionLots > 0n
            ? markTicks > boundary
            : markTicks < boundary;
      if (markStillSafe) {
        // Refresh LRU position.
        cache.delete(key);
        cache.set(key, cached);
        return cached;
      }
      cache.delete(key);
    }

    const result = computeProjectedLiquidation(input, marketParams);
    cache.set(key, result);
    if (cache.size > maxCacheEntries) {
      const oldest = cache.keys().next().value;
      if (oldest !== undefined) {
        cache.delete(oldest);
      }
    }
    return result;
  };
};

const projectedLiquidationCacheKey = (
  input: ProjectedLiquidationInput,
  marketParams: NormalizedMarketParams
): string => {
  const state = input.limitOrderState;
  const orders = input.visibleOrders
    .map(
      (order) =>
        `${order.side}:${order.priceTicks}:${order.baseLotsRemaining}:${order.reduceOnly ? 1 : 0}`
    )
    .join(",");
  const tiers = marketParams.leverageTiers
    .map(
      (tier) =>
        `${tier.upperBoundSize}:${tier.maxLeverage}:${tier.limitOrderRiskFactorBps}`
    )
    .join(",");
  return [
    marketParams.symbol,
    input.basePositionLots,
    input.virtualQuotePositionLots,
    input.collateralBalanceQuoteLots,
    input.portfolioUnsettledFundingQuoteLots,
    input.portfolioDiscountedUnrealizedPnlQuoteLots,
    input.portfolioMaintenanceMarginQuoteLots,
    input.targetDiscountedUnrealizedPnlQuoteLots,
    input.targetMaintenanceMarginQuoteLots,
    state.totalNonReduceOnlyAskBaseLots,
    state.totalReduceOnlyAskBaseLots,
    state.totalNonReduceOnlyBidBaseLots,
    state.totalReduceOnlyBidBaseLots,
    state.lowestAsk,
    state.highestBid,
    orders,
    marketParams.tickSize,
    marketParams.upnlRiskFactor,
    marketParams.riskFactors.maintenanceMarginFactorBps,
    tiers,
  ].join("|");
};
