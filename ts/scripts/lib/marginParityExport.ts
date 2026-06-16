import {
  computeSubaccountMarginFromInputs,
  createMarginCalculator,
} from "../../src/margin/compute";
import {
  absBigInt,
  applyBps,
  divCeil,
  getLeverageConstant,
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
            numBidOrders: number;
            numAskOrders: number;
            totalNonReduceOnlyBidBaseLots: string;
            totalNonReduceOnlyAskBaseLots: string;
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

const quoteLotsToUsd = (quoteLots: bigint): number =>
  Number(quoteLots) / 1_000_000;

const baseLotsToUnits = (baseLots: bigint, baseLotDecimals: number): number =>
  Number(baseLots) / Math.pow(10, baseLotDecimals);

const priceToTicks = (
  price: number,
  market: MarginParitySnapshotFile["markets"][number]
): bigint | undefined => {
  if (!Number.isFinite(price) || price <= 0) {
    return undefined;
  }
  const numerator = price * 1_000_000;
  const denominator =
    Number(market.tickSize) * Math.pow(10, market.baseLotDecimals);
  if (!Number.isFinite(denominator) || denominator <= 0) {
    return undefined;
  }
  const ticks = Math.round(numerator / denominator);
  if (!Number.isFinite(ticks) || ticks < 0) {
    return undefined;
  }
  return BigInt(ticks);
};

const parseLeverageTiers = (
  market: MarginParitySnapshotFile["markets"][number]
): LeverageTier[] =>
  market.leverageTiers.map((tier) => ({
    upperBoundSize: toBigInt(tier.upperBoundSize),
    maxLeverage: toBigInt(tier.maxLeverage),
    limitOrderRiskFactorBps: toBigInt(tier.limitOrderRiskFactorBps),
  }));

const solveLiquidationPrice = (
  positionSize: number,
  entryPrice: number,
  leverage: bigint,
  maintenanceBps: bigint,
  rawCollateral: number,
  otherAssetUnrealizedPnl: number,
  otherAssetMaintenanceMargin: number
): number | undefined => {
  if (positionSize === 0 || leverage === 0n || maintenanceBps <= 0n) {
    return undefined;
  }
  const leverageNumber = Number(leverage);
  const maintenanceCoefficient =
    (Math.abs(positionSize) * Number(maintenanceBps)) / 10_000 / leverageNumber;
  const denom = maintenanceCoefficient - positionSize;
  if (!Number.isFinite(denom) || Math.abs(denom) < 1e-10) {
    return undefined;
  }
  const numerator =
    rawCollateral +
    otherAssetUnrealizedPnl -
    otherAssetMaintenanceMargin -
    entryPrice * positionSize;
  const price = numerator / denom;
  if (!Number.isFinite(price) || price <= 0) {
    return undefined;
  }
  return price;
};

const computeLiquidationPriceTicks = (
  market: MarketMarginResult,
  totals: MarginTotals,
  marketParams: MarginParitySnapshotFile["markets"][number]
): string | undefined => {
  const basePositionLots = toBigInt(market.basePositionLots);
  if (basePositionLots === 0n) {
    return undefined;
  }
  const basePosition = baseLotsToUnits(
    basePositionLots,
    marketParams.baseLotDecimals
  );
  if (basePosition === 0) {
    return undefined;
  }

  const entryPrice =
    quoteLotsToUsd(absBigInt(toBigInt(market.virtualQuotePositionLots))) /
    Math.abs(basePosition);
  const rawCollateral = quoteLotsToUsd(
    toBigInt(String(totals.collateralBalanceQuoteLots))
  );
  const otherAssetUnrealizedPnl = quoteLotsToUsd(
    toBigInt(String(totals.discountedUnrealizedPnlQuoteLots)) -
      toBigInt(market.discountedUnrealizedPnlQuoteLots)
  );
  const assetPositionMaintenanceMargin = applyBps(
    toBigInt(market.positionInitialMarginQuoteLots),
    toBigInt(marketParams.riskFactors.maintenanceMarginFactorBps)
  );
  const otherAssetMaintenanceMargin = quoteLotsToUsd(
    toBigInt(String(totals.maintenanceMarginQuoteLots)) -
      assetPositionMaintenanceMargin
  );
  const leverage = getLeverageConstant(
    parseLeverageTiers(marketParams),
    absBigInt(basePositionLots)
  );
  const maintenanceBps = toBigInt(
    marketParams.riskFactors.maintenanceMarginFactorBps
  );
  const liquidationPrice = solveLiquidationPrice(
    basePosition,
    entryPrice,
    leverage,
    maintenanceBps,
    rawCollateral,
    otherAssetUnrealizedPnl,
    otherAssetMaintenanceMargin
  );
  const liquidationPriceTicks =
    liquidationPrice === undefined
      ? undefined
      : priceToTicks(liquidationPrice, marketParams);
  return liquidationPriceTicks?.toString();
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
            numBidOrders: market.limitOrderState.numBidOrders,
            numAskOrders: market.limitOrderState.numAskOrders,
            totalNonReduceOnlyBidBaseLots:
              market.limitOrderState.totalNonReduceOnlyBidBaseLots,
            totalNonReduceOnlyAskBaseLots:
              market.limitOrderState.totalNonReduceOnlyAskBaseLots,
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
            computeLiquidationPriceTicks(market, result.margin, marketParams) ??
            undefined,
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
