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
} from "./math";
import { buildNormalizedMarketParamsBySymbol } from "./normalize";
import type {
  NormalizedMarketParams,
  NormalizedMarketParamsBySymbol,
} from "./normalize";
import { buildLimitOrderMarginStateFromOrders } from "./inputs";
import type {
  LimitOrderMarginInput,
  LimitOrderMarginState,
  MarginCalculationOptions,
  MarginTotals,
  MarginPositionState,
  MarginRiskState,
  MarginRiskTier,
  MarketMarginInputs,
  MarketMarginResult,
  MarketParams,
  OrderMarginResult,
  SubaccountMarginInputs,
  SubaccountMarginResult,
  TraderMarginInputs,
  TraderMarginResult,
} from "./types";

// Source of truth for these calculations lives in Rust (line numbers as of 2026-01-15):
// - Initial margin + limit order margin: program-core/exchange/src/margin.rs:308-385
// - Initial margin (withdrawal strictness path): program-core/exchange/src/margin.rs:394-533
// - Risk tier assessment: program-core/exchange/src/margin.rs:886-945
// - Risk state (healthy/unhealthy/underwater): program-core/exchange/src/margin.rs:121-142
// - Effective collateral (collateral + discounted uPnL + funding): program-core/exchange/src/risk_view/mod.rs:135-182
// - Risk tier wrapper (RiskView -> assess_risk_tier): program-core/exchange/src/risk_view/mod.rs:289-316
// - Leverage tier interpolation: program-core/exchange/src/accounts/perp_asset_map/leverage_tier.rs:69-112
// - SDK mirror of risk tier thresholds: sdk/phoenix-state/src/margin.rs:209-232

export interface MarketParamsBySymbol {
  [symbol: string]: MarketParams;
}

export const buildMarketParamsBySymbol = (
  markets: MarketParams[]
): MarketParamsBySymbol => {
  const map: MarketParamsBySymbol = {};
  for (const market of markets) {
    if (map[market.symbol]) {
      throw new Error(`Duplicate market params for symbol ${market.symbol}`);
    }
    map[market.symbol] = market;
  }
  return map;
};

export interface MarginCalculator {
  markets: NormalizedMarketParamsBySymbol;
  computeTraderMargin: (
    inputs: TraderMarginInputs,
    options?: MarginCalculationOptions
  ) => TraderMarginResult;
  computeSubaccountMargin: (
    inputs: SubaccountMarginInputs,
    options?: MarginCalculationOptions
  ) => SubaccountMarginResult;
  computeTraderMarginFromInputs: (
    inputs: TraderMarginInputs,
    options?: MarginCalculationOptions
  ) => TraderMarginResult;
  computeSubaccountMarginFromInputs: (
    inputs: SubaccountMarginInputs,
    options?: MarginCalculationOptions
  ) => SubaccountMarginResult;
}

export const createMarginCalculator = (
  markets: MarketParams[]
): MarginCalculator => {
  const normalized = buildNormalizedMarketParamsBySymbol(markets);
  return {
    markets: normalized,
    computeTraderMargin: (inputs, callOptions) =>
      computeTraderMargin(inputs, normalized, callOptions),
    computeSubaccountMargin: (inputs, callOptions) =>
      computeSubaccountMargin(inputs, normalized, callOptions),
    computeTraderMarginFromInputs: (inputs, callOptions) =>
      computeTraderMarginFromInputs(inputs, normalized, callOptions),
    computeSubaccountMarginFromInputs: (inputs, callOptions) =>
      computeSubaccountMarginFromInputs(inputs, normalized, callOptions),
  };
};

export const computeTraderMargin = (
  inputs: TraderMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  options?: MarginCalculationOptions
): TraderMarginResult => {
  return computeTraderMarginFromInputs(inputs, marketsBySymbol, options);
};

export const computeSubaccountMargin = (
  inputs: SubaccountMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  options?: MarginCalculationOptions
): SubaccountMarginResult => {
  return computeSubaccountMarginFromInputs(inputs, marketsBySymbol, options);
};

export const computeTraderMarginFromInputs = (
  inputs: TraderMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  options?: MarginCalculationOptions
): TraderMarginResult => ({
  authority: inputs.authority,
  traderPdaIndex: inputs.traderPdaIndex,
  subaccounts: inputs.subaccounts.map((subaccount) =>
    computeSubaccountMarginFromInputs(subaccount, marketsBySymbol, options)
  ),
});

export const computeSubaccountMarginFromInputs = (
  inputs: SubaccountMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  options?: MarginCalculationOptions
): SubaccountMarginResult => {
  const marketMargins: MarketMarginResult[] = [];
  const limitOrders: OrderMarginResult[] = [];

  let totalInitialMargin = 0n;
  let hasOrderLeverageAdjustedInitialMargin = false;
  let totalOrderLeverageAdjustedInitialMargin = 0n;
  let totalInitialMarginForWithdrawals = 0n;
  let totalMaintenanceMargin = 0n;
  let totalCancelMargin = 0n;
  let totalBackstopMargin = 0n;
  let totalHighRiskMargin = 0n;
  let totalLimitOrderMargin = 0n;
  let totalUnrealizedPnl = 0n;
  let totalDiscountedUnrealizedPnl = 0n;
  let totalDiscountedPnlForWithdrawals = 0n;
  let totalUnsettledFunding = 0n;
  let totalAccumulatedFunding = 0n;

  const seenSymbols = new Set<string>();
  for (const marketInput of inputs.markets) {
    if (seenSymbols.has(marketInput.symbol)) {
      throw new Error(
        `Duplicate margin inputs for symbol ${marketInput.symbol}`
      );
    }
    seenSymbols.add(marketInput.symbol);

    const symbol = marketInput.symbol;
    const marketParams = marketsBySymbol[symbol];
    if (!marketParams) {
      throw new Error(`Missing market params for symbol ${symbol}`);
    }

    const result = computeMarketMarginFromInputs(
      marketInput,
      marketParams,
      options
    );

    marketMargins.push(result.market);
    limitOrders.push(...result.limitOrders);

    totalInitialMargin += result.margin.initialMargin;
    hasOrderLeverageAdjustedInitialMargin =
      hasOrderLeverageAdjustedInitialMargin ||
      result.margin.orderLeverageAdjustedInitialMargin !== undefined;
    totalOrderLeverageAdjustedInitialMargin +=
      result.margin.orderLeverageAdjustedInitialMargin ??
      result.margin.initialMargin;
    totalInitialMarginForWithdrawals +=
      result.margin.initialMarginForWithdrawals;
    totalMaintenanceMargin += result.margin.maintenanceMargin;
    totalCancelMargin += result.margin.cancelMargin;
    totalBackstopMargin += result.margin.backstopMargin;
    totalHighRiskMargin += result.margin.highRiskMargin;
    totalLimitOrderMargin += result.margin.limitOrderMargin;
    totalUnrealizedPnl += result.margin.unrealizedPnl;
    totalDiscountedUnrealizedPnl += result.margin.discountedUnrealizedPnl;
    totalDiscountedPnlForWithdrawals +=
      result.margin.discountedPnlForWithdrawals;
    totalUnsettledFunding += result.margin.unsettledFunding;
    totalAccumulatedFunding += result.margin.accumulatedFunding;
  }

  marketMargins.sort((a, b) => a.symbol.localeCompare(b.symbol));
  limitOrders.sort((a, b) => {
    const symbolCmp = a.symbol.localeCompare(b.symbol);
    if (symbolCmp !== 0) {
      return symbolCmp;
    }
    return a.orderSequenceNumber.localeCompare(b.orderSequenceNumber);
  });

  const collateralBalance = toBigInt(inputs.collateralBalanceQuoteLots ?? "0");
  const effectiveCollateral =
    collateralBalance + totalDiscountedUnrealizedPnl + totalUnsettledFunding;
  const effectiveCollateralForWithdrawals =
    collateralBalance +
    totalDiscountedPnlForWithdrawals +
    totalUnsettledFunding;
  const portfolioValue = collateralBalance + totalUnrealizedPnl;

  const riskState = computeRiskState(totalInitialMargin, effectiveCollateral);
  const riskTier = computeRiskTier(
    totalHighRiskMargin,
    totalBackstopMargin,
    totalMaintenanceMargin,
    totalCancelMargin,
    totalInitialMargin,
    effectiveCollateral
  );

  const margin: MarginTotals = {
    collateralBalanceQuoteLots: collateralBalance.toString(),
    effectiveCollateralQuoteLots: effectiveCollateral.toString(),
    effectiveCollateralForWithdrawalsQuoteLots:
      effectiveCollateralForWithdrawals.toString(),
    portfolioValueQuoteLots: portfolioValue.toString(),
    initialMarginQuoteLots: totalInitialMargin.toString(),
    initialMarginForWithdrawalsQuoteLots:
      totalInitialMarginForWithdrawals.toString(),
    maintenanceMarginQuoteLots: totalMaintenanceMargin.toString(),
    cancelMarginQuoteLots: totalCancelMargin.toString(),
    backstopMarginQuoteLots: totalBackstopMargin.toString(),
    highRiskMarginQuoteLots: totalHighRiskMargin.toString(),
    limitOrderMarginQuoteLots: totalLimitOrderMargin.toString(),
    unrealizedPnlQuoteLots: totalUnrealizedPnl.toString(),
    discountedUnrealizedPnlQuoteLots: totalDiscountedUnrealizedPnl.toString(),
    discountedPnlForWithdrawalsQuoteLots:
      totalDiscountedPnlForWithdrawals.toString(),
    unsettledFundingQuoteLots: totalUnsettledFunding.toString(),
    accumulatedFundingQuoteLots: totalAccumulatedFunding.toString(),
    riskState,
    riskTier,
  };
  if (
    hasOrderLeverageAdjustedInitialMargin &&
    totalOrderLeverageAdjustedInitialMargin !== totalInitialMargin
  ) {
    margin.orderLeverageAdjustedInitialMarginQuoteLots =
      totalOrderLeverageAdjustedInitialMargin.toString();
  }

  return {
    subaccountIndex: inputs.subaccountIndex,
    margin,
    marketMargins,
    limitOrders,
  };
};

const filterActiveOrders = (
  orders: LimitOrderMarginInput[]
): LimitOrderMarginInput[] =>
  orders.filter((order) => (order.status ? order.status === "active" : true));

const resolveLimitOrderMarginState = (
  orders: LimitOrderMarginInput[],
  limitOrderMargin: LimitOrderMarginState | undefined
): LimitOrderMarginState | undefined => {
  if (limitOrderMargin) {
    return limitOrderMargin;
  }
  if (orders.length === 0) {
    return undefined;
  }
  return buildLimitOrderMarginStateFromOrders(orders);
};

const getOrderLeverageLimitForSymbol = (
  symbol: string,
  options?: MarginCalculationOptions
): bigint | undefined => {
  // User/product preferences are intentionally permissive: fractional values
  // floor to safe integer leverage, while invalid values silently fall back to
  // protocol leverage.
  const rawLimit = options?.orderLeverageLimitsBySymbol?.[symbol];
  if (rawLimit === undefined || !Number.isFinite(rawLimit)) {
    return undefined;
  }

  const flooredLimit = Math.floor(rawLimit);
  if (flooredLimit < 1 || !Number.isSafeInteger(flooredLimit)) {
    return undefined;
  }

  return BigInt(flooredLimit);
};

const getEffectiveLeverageConstant = (
  tiers: LeverageTier[],
  positionSize: bigint,
  orderLeverageLimit?: bigint
): bigint => {
  const protocolMaxLeverage = getLeverageConstant(tiers, positionSize);
  return orderLeverageLimit !== undefined &&
    orderLeverageLimit < protocolMaxLeverage
    ? orderLeverageLimit
    : protocolMaxLeverage;
};

const computeMarketMarginFromInputs = (
  input: MarketMarginInputs,
  marketParams: NormalizedMarketParams,
  options?: MarginCalculationOptions
): {
  market: MarketMarginResult;
  limitOrders: OrderMarginResult[];
  margin: {
    initialMargin: bigint;
    orderLeverageAdjustedInitialMargin?: bigint;
    initialMarginForWithdrawals: bigint;
    maintenanceMargin: bigint;
    backstopMargin: bigint;
    highRiskMargin: bigint;
    cancelMargin: bigint;
    limitOrderMargin: bigint;
    unrealizedPnl: bigint;
    discountedUnrealizedPnl: bigint;
    discountedPnlForWithdrawals: bigint;
    unsettledFunding: bigint;
    accumulatedFunding: bigint;
    positionValue: bigint;
  };
} => {
  const assetUnitPrice = marketParams.markPriceTicks * marketParams.tickSize;

  const position: MarginPositionState | undefined = input.position;
  const basePositionLots = position ? toBigInt(position.basePositionLots) : 0n;
  const virtualQuotePositionLots = position
    ? toBigInt(position.virtualQuotePositionLots)
    : 0n;
  const entryPriceTicks = position ? toBigInt(position.entryPriceTicks) : 0n;

  const unsettledFunding = position
    ? toBigInt(position.unsettledFundingQuoteLots)
    : 0n;
  const accumulatedFunding = position
    ? toBigInt(position.accumulatedFundingQuoteLots)
    : 0n;

  const positionValue = assetUnitPrice * basePositionLots;
  const unrealizedPnl = virtualQuotePositionLots + positionValue;

  const upnlRiskFactor = marketParams.upnlRiskFactor;
  const upnlRiskFactorForWithdrawals =
    marketParams.upnlRiskFactorForWithdrawals;

  const discountedUnrealizedPnl =
    unrealizedPnl > 0n
      ? applyBpsCeil(unrealizedPnl, upnlRiskFactor)
      : unrealizedPnl;

  const discountedPnlForWithdrawals =
    unrealizedPnl > 0n
      ? applyBpsCeil(unrealizedPnl, upnlRiskFactorForWithdrawals)
      : unrealizedPnl;

  const activeOrders = filterActiveOrders(input.limitOrders ?? []);
  const limitOrderState = resolveLimitOrderMarginState(
    activeOrders,
    input.limitOrderMargin
  );
  const totalBid = limitOrderState
    ? toBigInt(limitOrderState.totalNonReduceOnlyBidBaseLots)
    : 0n;
  const totalAsk = limitOrderState
    ? toBigInt(limitOrderState.totalNonReduceOnlyAskBaseLots)
    : 0n;
  const leverageTiers = marketParams.leverageTiers;
  const orderLeverageLimit = getOrderLeverageLimitForSymbol(
    input.symbol,
    options
  );

  const initialMargin = initialMarginForAsset(
    basePositionLots,
    totalBid,
    totalAsk,
    assetUnitPrice,
    leverageTiers,
    false
  );

  const orderLeverageAdjustedInitialMargin =
    orderLeverageLimit === undefined
      ? initialMargin
      : initialMarginForAsset(
          basePositionLots,
          totalBid,
          totalAsk,
          assetUnitPrice,
          leverageTiers,
          false,
          orderLeverageLimit
        );

  const positionOnlyInitialMargin = initialMarginForAsset(
    basePositionLots,
    0n,
    0n,
    assetUnitPrice,
    leverageTiers,
    false
  );

  const initialMarginForWithdrawals = initialMarginForAsset(
    basePositionLots,
    totalBid,
    totalAsk,
    assetUnitPrice,
    leverageTiers,
    true
  );

  const limitOrderMargin = maxBigInt(
    initialMargin - positionOnlyInitialMargin,
    0n
  );
  const orderLeverageAdjustedInitialMarginOverride =
    orderLeverageAdjustedInitialMargin !== initialMargin
      ? orderLeverageAdjustedInitialMargin
      : undefined;

  const maintenanceMargin = applyBps(
    initialMargin,
    marketParams.riskFactors.maintenanceMarginFactorBps
  );
  const backstopMargin = applyBps(
    initialMargin,
    marketParams.riskFactors.backstopMarginFactorBps
  );
  const highRiskMargin = applyBps(
    initialMargin,
    marketParams.riskFactors.highRiskMarginFactorBps
  );
  const cancelMargin = applyBps(
    initialMargin,
    marketParams.cancelOrderRiskFactorBps
  );

  const protocolLimitOrders = computeLimitOrderMargins(
    input.symbol,
    basePositionLots,
    activeOrders,
    assetUnitPrice,
    leverageTiers
  );
  const limitOrders =
    orderLeverageLimit === undefined
      ? protocolLimitOrders
      : withOrderLeverageAdjustedLimitOrderMargins(
          protocolLimitOrders,
          computeLimitOrderMargins(
            input.symbol,
            basePositionLots,
            activeOrders,
            assetUnitPrice,
            leverageTiers,
            orderLeverageLimit
          )
        );

  const market: MarketMarginResult = {
    symbol: input.symbol,
    basePositionLots: basePositionLots.toString(),
    virtualQuotePositionLots: virtualQuotePositionLots.toString(),
    entryPriceTicks: entryPriceTicks.toString(),
    unrealizedPnlQuoteLots: unrealizedPnl.toString(),
    discountedUnrealizedPnlQuoteLots: discountedUnrealizedPnl.toString(),
    positionInitialMarginQuoteLots: positionOnlyInitialMargin.toString(),
    initialMarginQuoteLots: initialMargin.toString(),
    maintenanceMarginQuoteLots: maintenanceMargin.toString(),
    cancelMarginQuoteLots: cancelMargin.toString(),
    backstopMarginQuoteLots: backstopMargin.toString(),
    highRiskMarginQuoteLots: highRiskMargin.toString(),
    limitOrderMarginQuoteLots: limitOrderMargin.toString(),
    positionValueQuoteLots: positionValue.toString(),
    unsettledFundingQuoteLots: unsettledFunding.toString(),
    accumulatedFundingQuoteLots: accumulatedFunding.toString(),
  };

  return {
    market,
    limitOrders,
    margin: {
      initialMargin,
      orderLeverageAdjustedInitialMargin:
        orderLeverageAdjustedInitialMarginOverride,
      initialMarginForWithdrawals,
      maintenanceMargin,
      backstopMargin,
      highRiskMargin,
      cancelMargin,
      limitOrderMargin,
      unrealizedPnl,
      discountedUnrealizedPnl,
      discountedPnlForWithdrawals,
      unsettledFunding,
      accumulatedFunding,
      positionValue,
    },
  };
};

const initialMarginForAsset = (
  position: bigint,
  totalBid: bigint,
  totalAsk: bigint,
  assetUnitPrice: bigint,
  tiers: LeverageTier[],
  bypassRiskFactor: boolean,
  orderLeverageLimit?: bigint
): bigint => {
  if (position === 0n && totalBid === 0n && totalAsk === 0n) {
    return 0n;
  }

  let collateralRequired = 0n;
  let existingPositionMarginOffset = 0n;

  if (position !== 0n) {
    const absolutePositionSize = absBigInt(position);
    const absoluteBookValue = assetUnitPrice * absolutePositionSize;
    const leverage = getEffectiveLeverageConstant(
      tiers,
      absolutePositionSize,
      orderLeverageLimit
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
          tiers,
          existingPositionMarginOffset,
          bypassRiskFactor,
          orderLeverageLimit
        )
      : 0n;

  const marginAsk =
    totalAsk > 0n
      ? marginIncreaseForAsks(
          position,
          totalAsk,
          assetUnitPrice,
          tiers,
          existingPositionMarginOffset,
          bypassRiskFactor,
          orderLeverageLimit
        )
      : 0n;

  collateralRequired += maxBigInt(marginBid, marginAsk);

  return collateralRequired;
};

const marginIncreaseForBids = (
  position: bigint,
  bidSize: bigint,
  assetUnitPrice: bigint,
  tiers: LeverageTier[],
  existingPositionMarginOffset: bigint,
  bypassRiskFactor: boolean,
  orderLeverageLimit?: bigint
): bigint => {
  const newExposureSigned = bidSize + position - absBigInt(position);
  if (newExposureSigned <= 0n) {
    return 0n;
  }

  const totalExposureSigned = position + bidSize;
  const totalExposure = absBigInt(totalExposureSigned);
  const totalGrossValue = assetUnitPrice * totalExposure;
  const totalLeverage = getEffectiveLeverageConstant(
    tiers,
    totalExposure,
    orderLeverageLimit
  );
  const totalMargin = divCeil(totalGrossValue, totalLeverage);
  const incrementalMargin = maxBigInt(
    totalMargin - existingPositionMarginOffset,
    0n
  );

  if (bypassRiskFactor) {
    return incrementalMargin;
  }

  const riskFactor = getLimitOrderRiskFactor(tiers, totalExposure);
  return applyBpsCeil(incrementalMargin, riskFactor);
};

const marginIncreaseForAsks = (
  position: bigint,
  askSize: bigint,
  assetUnitPrice: bigint,
  tiers: LeverageTier[],
  existingPositionMarginOffset: bigint,
  bypassRiskFactor: boolean,
  orderLeverageLimit?: bigint
): bigint => {
  const newExposureSigned = askSize - position - absBigInt(position);
  if (newExposureSigned <= 0n) {
    return 0n;
  }

  const totalExposureSigned = position - askSize;
  const totalExposure = absBigInt(totalExposureSigned);
  const totalGrossValue = assetUnitPrice * totalExposure;
  const totalLeverage = getEffectiveLeverageConstant(
    tiers,
    totalExposure,
    orderLeverageLimit
  );
  const totalMargin = divCeil(totalGrossValue, totalLeverage);
  const incrementalMargin = maxBigInt(
    totalMargin - existingPositionMarginOffset,
    0n
  );

  if (bypassRiskFactor) {
    return incrementalMargin;
  }

  const riskFactor = getLimitOrderRiskFactor(tiers, totalExposure);
  return applyBpsCeil(incrementalMargin, riskFactor);
};

const withOrderLeverageAdjustedLimitOrderMargins = (
  protocolOrders: OrderMarginResult[],
  orderLeverageAdjustedOrders: OrderMarginResult[]
): OrderMarginResult[] =>
  protocolOrders.map((order, index) => {
    const orderLeverageAdjustedOrder = orderLeverageAdjustedOrders[index];
    if (
      !orderLeverageAdjustedOrder ||
      orderLeverageAdjustedOrder.marginRequirementQuoteLots ===
        order.marginRequirementQuoteLots
    ) {
      return order;
    }

    return {
      ...order,
      orderLeverageAdjustedMarginRequirementQuoteLots:
        orderLeverageAdjustedOrder.marginRequirementQuoteLots,
    };
  });

const computeLimitOrderMargins = (
  symbol: string,
  position: bigint,
  orders: LimitOrderMarginInput[],
  assetUnitPrice: bigint,
  tiers: LeverageTier[],
  orderLeverageLimit?: bigint
): OrderMarginResult[] => {
  const result: OrderMarginResult[] = [];
  const existingPositionMarginOffset =
    position === 0n
      ? 0n
      : divCeil(
          assetUnitPrice * absBigInt(position),
          getEffectiveLeverageConstant(
            tiers,
            absBigInt(position),
            orderLeverageLimit
          )
        );

  for (const order of orders) {
    const remainingBaseLots = order.reduceOnly
      ? 0n
      : toBigInt(order.sizeRemainingLots);
    const orderSize = remainingBaseLots;

    const marginRequirement =
      order.side === "bid"
        ? marginIncreaseForBids(
            position,
            orderSize,
            assetUnitPrice,
            tiers,
            existingPositionMarginOffset,
            false,
            orderLeverageLimit
          )
        : marginIncreaseForAsks(
            position,
            orderSize,
            assetUnitPrice,
            tiers,
            existingPositionMarginOffset,
            false,
            orderLeverageLimit
          );

    const marginFactor =
      marginRequirement === 0n
        ? 0n
        : getLimitOrderRiskFactor(
            tiers,
            order.side === "bid"
              ? absBigInt(position + remainingBaseLots)
              : absBigInt(position - remainingBaseLots)
          );

    result.push({
      symbol,
      orderSequenceNumber: order.orderSequenceNumber,
      side: order.side,
      priceTicks: order.priceTicks,
      tradeSizeRemainingLots: order.sizeRemainingLots,
      initialSizeLots: order.initialSizeLots,
      reduceOnly: order.reduceOnly,
      isStopLoss: order.isStopLoss ?? false,
      isStopLossDirection: order.isStopLossDirection ?? false,
      marginRequirementQuoteLots: marginRequirement.toString(),
      marginFactorBps: marginFactor.toString(),
    });
  }

  return result;
};

const computeRiskState = (
  initialMargin: bigint,
  effectiveCollateral: bigint
): MarginRiskState => {
  if (effectiveCollateral < 0n) {
    return "underwater";
  }
  if (effectiveCollateral === 0n) {
    return initialMargin === 0n ? "zeroCollateralNoPositions" : "underwater";
  }
  return effectiveCollateral >= initialMargin ? "healthy" : "unhealthy";
};

const computeRiskTier = (
  highRiskMargin: bigint,
  backstopMargin: bigint,
  maintenanceMargin: bigint,
  cancelMargin: bigint,
  initialMargin: bigint,
  effectiveCollateral: bigint
): MarginRiskTier => {
  if (effectiveCollateral < 0n) {
    return "highRisk";
  }

  const effective = effectiveCollateral;

  if (effective < highRiskMargin) {
    return "highRisk";
  }
  if (effective < backstopMargin) {
    return "backstopLiquidatable";
  }
  if (effective < maintenanceMargin) {
    return "liquidatable";
  }
  if (effective < cancelMargin) {
    return "cancellable";
  }
  if (effective < initialMargin) {
    return "atRisk";
  }
  return "safe";
};
