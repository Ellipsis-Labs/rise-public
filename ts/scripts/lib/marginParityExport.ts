import {
  computeSubaccountMarginFromInputs,
  createMarginCalculator,
} from "../../src/margin/compute";
import {
  computeProjectedLiquidation,
  type ProjectedLiquidationInput,
} from "../../src/margin/liquidation";
import { normalizeMarketParams } from "../../src/margin/normalize";
import {
  absBigInt,
  applyBps,
  applyBpsCeil,
  divCeil,
  getLeverageConstant,
  getLimitOrderRiskFactor,
  maxBigInt,
  toBigInt,
  type LeverageTier,
} from "../../src/margin/math";
import type {
  MarginTotals,
  MarketMarginInputs,
  MarketMarginResult,
  OrderMarginResult,
  SubaccountMarginInputs,
} from "../../src/margin/types";

export type MarginParitySnapshotFile = {
  slot: number;
  markets: Array<{
    symbol: string;
    assetId: number;
    markPriceTicks: string;
    bestBidTicks?: string;
    bestAskTicks?: string;
    tickSize: string;
    baseLotDecimals: number;
    leverageTiers: Array<{
      upperBoundSize: string;
      maxLeverage: string;
      limitOrderRiskFactorBps: string;
    }>;
    riskFactors: {
      maintenanceMarginFactorBps: string;
      backstopMarginFactorBps: string;
      highRiskMarginFactorBps: string;
    };
    cancelOrderRiskFactorBps: string;
    upnlRiskFactor: string;
    upnlRiskFactorForWithdrawals: string;
    isolatedOnly: boolean;
  }>;
  traders: Array<{
    authority: string;
    traderPdaIndex: number;
    subaccounts: Array<{
      traderKey: string;
      traderSubaccountIndex: number;
      input: {
        collateralBalanceQuoteLots: string;
        markets: Array<{
          symbol: string;
          position?: {
            basePositionLots: string;
            virtualQuotePositionLots: string;
            entryPriceTicks: string;
            unsettledFundingQuoteLots: string;
            accumulatedFundingQuoteLots: string;
          };
          limitOrderState: {
            numAskOrders: number;
            numBidOrders: number;
            lowestAsk: string;
            highestBid: string;
            totalNonReduceOnlyAskBaseLots: string;
            totalReduceOnlyAskBaseLots: string;
            totalNonReduceOnlyBidBaseLots: string;
            totalReduceOnlyBidBaseLots: string;
          };
          visibleLimitOrders: Array<{
            orderSequenceNumber: string;
            side: "bid" | "ask";
            priceTicks: string;
            sizeRemainingLots: string;
            initialSizeLots: string;
            reduceOnly: boolean;
            isStopLoss: boolean;
            isStopLossDirection: boolean;
            isConditionalOrder: boolean;
          }>;
        }>;
      };
      expected: {
        marketMargins: Array<{
          symbol: string;
          cancelMarginQuoteLots: string;
          highRiskMarginQuoteLots: string;
        }>;
      };
    }>;
  }>;
};

export type MarginParityOutputFile = {
  schemaVersion: number;
  generatedAt: string;
  slot: number;
  engine: string;
  traders: Array<{
    authority: string;
    traderPdaIndex: number;
    subaccounts: Array<{
      traderKey: string;
      traderSubaccountIndex: number;
      margin: MarginTotals & {
        riskScore: number;
      };
      marketMargins: Array<
        MarketMarginResult & {
          liquidationPriceTicks?: string;
          projectedLiquidationPriceTicks?: string;
          cancelMarginQuoteLots: string;
          highRiskMarginQuoteLots: string;
          maxLimitBidBaseLots: string;
          maxLimitAskBaseLots: string;
          maxMarketBuyBaseLotsEstimate: string;
          maxMarketSellBaseLotsEstimate: string;
        }
      >;
      limitOrders: Array<
        OrderMarginResult & {
          isConditionalOrder: boolean;
        }
      >;
    }>;
  }>;
};

type GeneratedOutputOptions = {
  generatedAt?: string;
};

const computeRiskScore = (
  effectiveCollateral: bigint,
  maintenanceMargin: bigint
): number => {
  if (maintenanceMargin === 0n) {
    return 0;
  }
  if (effectiveCollateral > 0n) {
    return Number(
      BigInt(
        Math.round(
          (Number(maintenanceMargin) / Number(effectiveCollateral)) * 1000
        )
      )
    );
  }
  const underwater = maintenanceMargin - effectiveCollateral;
  return (
    1000 +
    Number(
      BigInt(
        Math.round((Number(underwater) / Number(maintenanceMargin)) * 1000)
      )
    )
  );
};

const MAX_HAWKEYE_LIQUIDATION_TICKS = 0xffff_ffffn;
const emptyLimitOrderState: NonNullable<
  MarketMarginInputs["limitOrderMargin"]
> = {
  numAskOrders: 0,
  numBidOrders: 0,
  lowestAsk: "0",
  highestBid: "0",
  totalNonReduceOnlyAskBaseLots: "0",
  totalReduceOnlyAskBaseLots: "0",
  totalNonReduceOnlyBidBaseLots: "0",
  totalReduceOnlyBidBaseLots: "0",
};

const parseLeverageTiers = (
  market: MarginParitySnapshotFile["markets"][number]
): LeverageTier[] =>
  market.leverageTiers.map((tier) => ({
    upperBoundSize: toBigInt(tier.upperBoundSize),
    maxLeverage: toBigInt(tier.maxLeverage),
    limitOrderRiskFactorBps: toBigInt(tier.limitOrderRiskFactorBps),
  }));

const computeLiquidationPriceTicks = (
  market: MarketMarginResult,
  totals: MarginTotals,
  marketParams: MarginParitySnapshotFile["markets"][number],
  marketInput: MarketMarginInputs
): string | undefined => {
  const basePositionLots = toBigInt(market.basePositionLots);
  if (basePositionLots === 0n) {
    return undefined;
  }
  const currentMarkTicks = toBigInt(marketParams.markPriceTicks);
  if (
    isLiquidatableAtTicks(
      currentMarkTicks,
      market,
      totals,
      marketParams,
      marketInput
    )
  ) {
    return currentMarkTicks.toString();
  }
  const liquidationPriceTicks = findLiquidationBoundaryTicks(
    market,
    totals,
    marketParams,
    marketInput
  );
  return liquidationPriceTicks?.toString();
};

/**
 * Computes the projected (path-dependent) liquidation boundary through the
 * rise TypeScript SDK implementation so the exported value exercises the same
 * code integrators call.
 */
const computeProjectedLiquidationPriceTicks = (
  market: MarketMarginResult,
  totals: MarginTotals,
  marketParams: MarginParitySnapshotFile["markets"][number],
  marketInput: MarketMarginInputs
): string | undefined => {
  const basePositionLots = toBigInt(market.basePositionLots);
  if (basePositionLots === 0n) {
    return undefined;
  }

  const state = marketInput.limitOrderMargin ?? emptyLimitOrderState;
  const input: ProjectedLiquidationInput = {
    basePositionLots,
    virtualQuotePositionLots: toBigInt(market.virtualQuotePositionLots),
    limitOrderState: {
      totalNonReduceOnlyAskBaseLots: toBigInt(
        state.totalNonReduceOnlyAskBaseLots
      ),
      totalReduceOnlyAskBaseLots: toBigInt(state.totalReduceOnlyAskBaseLots),
      totalNonReduceOnlyBidBaseLots: toBigInt(
        state.totalNonReduceOnlyBidBaseLots
      ),
      totalReduceOnlyBidBaseLots: toBigInt(state.totalReduceOnlyBidBaseLots),
      lowestAsk: toBigInt(state.lowestAsk),
      highestBid: toBigInt(state.highestBid),
    },
    visibleOrders: (marketInput.limitOrders ?? []).map((order) => ({
      side: order.side,
      priceTicks: toBigInt(order.priceTicks),
      baseLotsRemaining: toBigInt(order.sizeRemainingLots),
      reduceOnly: order.reduceOnly,
    })),
    collateralBalanceQuoteLots: toBigInt(
      String(totals.collateralBalanceQuoteLots)
    ),
    portfolioUnsettledFundingQuoteLots: toBigInt(
      String(totals.unsettledFundingQuoteLots)
    ),
    portfolioDiscountedUnrealizedPnlQuoteLots: toBigInt(
      String(totals.discountedUnrealizedPnlQuoteLots)
    ),
    portfolioMaintenanceMarginQuoteLots: toBigInt(
      String(totals.maintenanceMarginQuoteLots)
    ),
    targetDiscountedUnrealizedPnlQuoteLots: toBigInt(
      market.discountedUnrealizedPnlQuoteLots
    ),
    targetMaintenanceMarginQuoteLots: toBigInt(
      market.maintenanceMarginQuoteLots
    ),
  };

  const result = computeProjectedLiquidation(
    input,
    normalizeMarketParams(marketParams)
  );
  return result.liquidationPriceTicks?.toString();
};

const otherMarketMaintenanceMarginQuoteLots = (
  market: MarketMarginResult,
  totals: MarginTotals
): bigint => {
  const portfolioMaintenanceMargin = toBigInt(
    String(totals.maintenanceMarginQuoteLots)
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

const findLiquidationBoundaryTicks = (
  market: MarketMarginResult,
  totals: MarginTotals,
  marketParams: MarginParitySnapshotFile["markets"][number],
  marketInput: MarketMarginInputs
): bigint | undefined => {
  const basePositionLots = toBigInt(market.basePositionLots);
  if (basePositionLots === 0n) {
    return undefined;
  }

  const currentMarkTicks = toBigInt(marketParams.markPriceTicks);
  if (currentMarkTicks <= 0n) {
    return undefined;
  }

  if (basePositionLots > 0n) {
    if (!isLiquidatableAtTicks(0n, market, totals, marketParams, marketInput)) {
      return undefined;
    }

    let low = 0n;
    let high = currentMarkTicks;
    while (low + 1n < high) {
      const mid = low + (high - low) / 2n;
      if (
        isLiquidatableAtTicks(mid, market, totals, marketParams, marketInput)
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
      totals,
      marketParams,
      marketInput
    )
  ) {
    return undefined;
  }

  let low = currentMarkTicks;
  let high = MAX_HAWKEYE_LIQUIDATION_TICKS;
  while (low + 1n < high) {
    const mid = low + (high - low) / 2n;
    if (isLiquidatableAtTicks(mid, market, totals, marketParams, marketInput)) {
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
  totals: MarginTotals,
  marketParams: MarginParitySnapshotFile["markets"][number],
  marketInput: MarketMarginInputs
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
  const effectiveCollateral =
    toBigInt(String(totals.collateralBalanceQuoteLots)) +
    toBigInt(String(totals.unsettledFundingQuoteLots)) +
    (toBigInt(String(totals.discountedUnrealizedPnlQuoteLots)) -
      toBigInt(market.discountedUnrealizedPnlQuoteLots)) +
    targetDiscountedPnl;
  if (effectiveCollateral < 0n) {
    return true;
  }

  const maintenance =
    otherMarketMaintenanceMarginQuoteLots(market, totals) +
    maintenanceMarginAtTicks(ticks, market, marketParams, marketInput);
  return effectiveCollateral < maintenance;
};

const maintenanceMarginAtTicks = (
  ticks: bigint,
  market: MarketMarginResult,
  marketParams: MarginParitySnapshotFile["markets"][number],
  marketInput: MarketMarginInputs
): bigint => {
  const initialMargin = initialMarginForAssetAtTicks(
    toBigInt(market.basePositionLots),
    marketInput.limitOrderMargin ?? emptyLimitOrderState,
    ticks,
    marketParams
  );
  const maintenanceBps = toBigInt(
    marketParams.riskFactors.maintenanceMarginFactorBps
  );
  return applyBps(initialMargin, maintenanceBps);
};

const initialMarginForAssetAtTicks = (
  position: bigint,
  limitOrderState: NonNullable<MarketMarginInputs["limitOrderMargin"]>,
  ticks: bigint,
  marketParams: MarginParitySnapshotFile["markets"][number]
): bigint => {
  const totalBid = toBigInt(limitOrderState.totalNonReduceOnlyBidBaseLots);
  const totalAsk = toBigInt(limitOrderState.totalNonReduceOnlyAskBaseLots);
  const totalBidAll =
    totalBid + toBigInt(limitOrderState.totalReduceOnlyBidBaseLots);
  const totalAskAll =
    totalAsk + toBigInt(limitOrderState.totalReduceOnlyAskBaseLots);
  if (
    position === 0n &&
    totalBid === 0n &&
    totalAsk === 0n &&
    totalBidAll === 0n &&
    totalAskAll === 0n
  ) {
    return 0n;
  }

  const tiers = parseLeverageTiers(marketParams);
  const tickSize = toBigInt(marketParams.tickSize);
  const assetUnitPrice = ticks * tickSize;
  let collateralRequired = 0n;
  let existingPositionMarginOffset = 0n;

  if (position !== 0n) {
    const absolutePositionSize = absBigInt(position);
    const absoluteBookValue = assetUnitPrice * absolutePositionSize;
    const leverage = getLeverageConstant(tiers, absolutePositionSize);
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
          tiers,
          existingPositionMarginOffset
        )
      : 0n;
  const marginAsk =
    totalAsk > 0n
      ? marginIncreaseForAsks(
          position,
          totalAsk,
          assetUnitPrice,
          tiers,
          existingPositionMarginOffset
        )
      : 0n;

  collateralRequired += maxBigInt(marginBid, marginAsk);
  collateralRequired +=
    totalBidAll > 0n
      ? limitOrderFillLoss(
          "bid",
          totalBidAll,
          toBigInt(limitOrderState.highestBid),
          assetUnitPrice,
          tickSize
        )
      : 0n;
  collateralRequired +=
    totalAskAll > 0n
      ? limitOrderFillLoss(
          "ask",
          totalAskAll,
          toBigInt(limitOrderState.lowestAsk),
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
  tiers: LeverageTier[],
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
  tiers: LeverageTier[],
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

const ceilDiv = (numerator: bigint, denominator: bigint): bigint => {
  if (denominator <= 0n) {
    throw new Error("ceilDiv denominator must be positive");
  }
  return (numerator + denominator - 1n) / denominator;
};

const analyticalSolutionForMaxPosition = (
  effectiveCollateral: bigint,
  assetUnitPrice: bigint,
  l2: bigint,
  l1: bigint,
  q2: bigint,
  q1: bigint
): bigint => {
  const deltaL = l1 - l2;
  const deltaQ = q2 - q1;
  const numerator = effectiveCollateral * (deltaQ * l1 + deltaL * q1);
  const denominator = deltaQ * assetUnitPrice + effectiveCollateral * deltaL;
  if (denominator === 0n) {
    return 0n;
  }
  return numerator / denominator;
};

const maxPositionForCollateralAtPrice = (
  effectiveCollateral: bigint,
  priceTicks: bigint,
  market: MarginParitySnapshotFile["markets"][number]
): bigint => {
  const assetUnitPrice = priceTicks * toBigInt(market.tickSize);
  if (effectiveCollateral <= 0n || assetUnitPrice === 0n) {
    return 0n;
  }

  const tiers = parseLeverageTiers(market);
  for (let index = 0; index < tiers.length; index += 1) {
    const tier = tiers[index];
    const tierPositionValue = assetUnitPrice * tier.upperBoundSize;
    const tierMarginRequired = divCeil(tierPositionValue, tier.maxLeverage);
    if (tierMarginRequired > effectiveCollateral) {
      if (index === 0) {
        const maxPosition =
          (effectiveCollateral * tier.maxLeverage) / assetUnitPrice;
        return maxPosition < tier.upperBoundSize
          ? maxPosition
          : tier.upperBoundSize;
      }
      const previous = tiers[index - 1];
      return analyticalSolutionForMaxPosition(
        effectiveCollateral,
        assetUnitPrice,
        tier.maxLeverage,
        previous.maxLeverage,
        tier.upperBoundSize,
        previous.upperBoundSize
      );
    }
  }

  const lastTier = tiers[tiers.length - 1];
  if (!lastTier) {
    return 0n;
  }
  const lastTierLeverage = getLeverageConstant(tiers, (1n << 63n) - 1n);
  const maxPosition = (effectiveCollateral * lastTierLeverage) / assetUnitPrice;
  return maxPosition > lastTier.upperBoundSize
    ? maxPosition
    : lastTier.upperBoundSize;
};

const computeMaxAdditionalLotsAtPrice = (
  effectiveCollateral: bigint,
  currentPosition: bigint,
  priceTicks: bigint,
  market: MarginParitySnapshotFile["markets"][number]
): { maxBidLots: bigint; maxAskLots: bigint } => {
  const maxAbsPosition = maxPositionForCollateralAtPrice(
    effectiveCollateral,
    priceTicks,
    market
  );
  const maxBidLotsRaw = maxAbsPosition - currentPosition;
  const maxAskLotsRaw = maxAbsPosition + currentPosition;
  return {
    maxBidLots: maxBidLotsRaw > 0n ? maxBidLotsRaw : 0n,
    maxAskLots: maxAskLotsRaw > 0n ? maxAskLotsRaw : 0n,
  };
};

export const buildMarginParityOutput = (
  snapshot: MarginParitySnapshotFile,
  options: GeneratedOutputOptions = {}
): MarginParityOutputFile => {
  const calculator = createMarginCalculator(snapshot.markets);
  const marketsBySymbol = new Map(
    snapshot.markets.map((market) => [market.symbol, market])
  );

  const traders = snapshot.traders.map((trader) => ({
    authority: trader.authority,
    traderPdaIndex: trader.traderPdaIndex,
    subaccounts: trader.subaccounts.map((subaccount) => {
      const markets: MarketMarginInputs[] = subaccount.input.markets.map(
        (market) => ({
          symbol: market.symbol,
          position: market.position
            ? {
                basePositionLots: market.position.basePositionLots,
                virtualQuotePositionLots:
                  market.position.virtualQuotePositionLots,
                entryPriceTicks: market.position.entryPriceTicks,
                unsettledFundingQuoteLots:
                  market.position.unsettledFundingQuoteLots,
                accumulatedFundingQuoteLots:
                  market.position.accumulatedFundingQuoteLots,
              }
            : undefined,
          limitOrderMargin: {
            numAskOrders: market.limitOrderState.numAskOrders ?? 0,
            numBidOrders: market.limitOrderState.numBidOrders ?? 0,
            lowestAsk: market.limitOrderState.lowestAsk ?? "0",
            highestBid: market.limitOrderState.highestBid ?? "0",
            totalNonReduceOnlyAskBaseLots:
              market.limitOrderState.totalNonReduceOnlyAskBaseLots ?? "0",
            totalReduceOnlyAskBaseLots:
              market.limitOrderState.totalReduceOnlyAskBaseLots ?? "0",
            totalNonReduceOnlyBidBaseLots:
              market.limitOrderState.totalNonReduceOnlyBidBaseLots ?? "0",
            totalReduceOnlyBidBaseLots:
              market.limitOrderState.totalReduceOnlyBidBaseLots ?? "0",
          },
          limitOrders: (market.visibleLimitOrders ?? []).map((order) => ({
            orderSequenceNumber: order.orderSequenceNumber,
            side: order.side,
            priceTicks: order.priceTicks,
            sizeRemainingLots: order.sizeRemainingLots,
            initialSizeLots: order.initialSizeLots,
            reduceOnly: order.reduceOnly,
            isStopLoss: order.isStopLoss,
            isStopLossDirection: order.isStopLossDirection,
          })),
        })
      );

      const inputs: SubaccountMarginInputs = {
        subaccountIndex: subaccount.traderSubaccountIndex,
        collateralBalanceQuoteLots: subaccount.input.collateralBalanceQuoteLots,
        markets,
      };
      const result = computeSubaccountMarginFromInputs(
        inputs,
        calculator.markets
      );
      const effectiveCollateral = BigInt(
        result.margin.effectiveCollateralQuoteLots
      );
      const maintenanceMargin = BigInt(
        result.margin.maintenanceMarginQuoteLots
      );
      const marketMargins = result.marketMargins.map((market) => {
        const marketParams = marketsBySymbol.get(market.symbol);
        if (!marketParams) {
          throw new Error(
            `missing snapshot market params for ${market.symbol}`
          );
        }
        const marketInput = inputs.markets.find(
          (input) => input.symbol === market.symbol
        );
        if (!marketInput) {
          throw new Error(`missing market input for ${market.symbol}`);
        }
        const currentPosition = toBigInt(market.basePositionLots);
        const markPriceTicks = toBigInt(marketParams.markPriceTicks);
        const bestBidTicks = marketParams.bestBidTicks
          ? toBigInt(marketParams.bestBidTicks)
          : markPriceTicks;
        const bestAskTicks = marketParams.bestAskTicks
          ? toBigInt(marketParams.bestAskTicks)
          : markPriceTicks;
        const maxLimit = computeMaxAdditionalLotsAtPrice(
          effectiveCollateral,
          currentPosition,
          markPriceTicks,
          marketParams
        );
        const maxMarketBuy = computeMaxAdditionalLotsAtPrice(
          effectiveCollateral,
          currentPosition,
          bestAskTicks,
          marketParams
        );
        const maxMarketSell = computeMaxAdditionalLotsAtPrice(
          effectiveCollateral,
          currentPosition,
          bestBidTicks,
          marketParams
        );

        return {
          ...market,
          liquidationPriceTicks:
            computeLiquidationPriceTicks(
              market,
              result.margin,
              marketParams,
              marketInput
            ) ?? undefined,
          projectedLiquidationPriceTicks:
            computeProjectedLiquidationPriceTicks(
              market,
              result.margin,
              marketParams,
              marketInput
            ) ?? undefined,
          maxLimitBidBaseLots: maxLimit.maxBidLots.toString(),
          maxLimitAskBaseLots: maxLimit.maxAskLots.toString(),
          maxMarketBuyBaseLotsEstimate: maxMarketBuy.maxBidLots.toString(),
          maxMarketSellBaseLotsEstimate: maxMarketSell.maxAskLots.toString(),
        };
      });

      return {
        traderKey: subaccount.traderKey,
        traderSubaccountIndex: subaccount.traderSubaccountIndex,
        margin: {
          ...result.margin,
          riskScore: computeRiskScore(effectiveCollateral, maintenanceMargin),
        },
        marketMargins,
        limitOrders: result.limitOrders.map((order) => ({
          ...order,
          isConditionalOrder:
            subaccount.input.markets
              .flatMap((market) => market.visibleLimitOrders ?? [])
              .find(
                (candidate) =>
                  candidate.orderSequenceNumber === order.orderSequenceNumber &&
                  candidate.side === order.side &&
                  candidate.priceTicks === order.priceTicks
              )?.isConditionalOrder ?? false,
        })),
      };
    }),
  }));

  return {
    schemaVersion: 1,
    generatedAt: options.generatedAt ?? new Date().toISOString(),
    slot: snapshot.slot,
    engine: "rise-ts",
    traders,
  };
};
