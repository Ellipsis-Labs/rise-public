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
import {
  computeSubaccountLiquidationPricesFromMargin,
  computeTraderLiquidationPricesFromMargin,
  type MarketLiquidationPriceResult,
  type SubaccountLiquidationPricesResult,
  type TraderLiquidationPricesResult,
} from "./liquidation";
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
  computeTraderLiquidationPricesFromInputs: (
    inputs: TraderMarginInputs
  ) => TraderLiquidationPricesResult;
  computeSubaccountLiquidationPricesFromInputs: (
    inputs: SubaccountMarginInputs
  ) => SubaccountLiquidationPricesResult;
  simulateMargin: (inputs: SimulateMarginInput) => SimulateMarginResult;
  simulateMarginScenarios: (
    inputs: SimulateMarginScenariosInput
  ) => SimulateMarginScenariosResult;
  simulatePositionFill: (
    inputs: SimulatePositionFillInput
  ) => SimulatedPositionFillResult;
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
    computeTraderLiquidationPricesFromInputs: (inputs) =>
      computeTraderLiquidationPricesFromInputs(inputs, normalized),
    computeSubaccountLiquidationPricesFromInputs: (inputs) =>
      computeSubaccountLiquidationPricesFromInputs(inputs, normalized),
    simulateMargin: (inputs) => simulateMarginFromInputs(inputs, normalized),
    simulateMarginScenarios: (inputs) =>
      simulateMarginScenariosFromInputs(inputs, normalized),
    simulatePositionFill: (inputs) =>
      simulatePositionFillFromInputs(inputs, normalized),
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

export const computeTraderLiquidationPricesFromInputs = (
  inputs: TraderMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol
): TraderLiquidationPricesResult =>
  computeTraderLiquidationPricesFromMargin(
    computeTraderMarginFromInputs(inputs, marketsBySymbol),
    marketsBySymbol
  );

export const computeSubaccountLiquidationPricesFromInputs = (
  inputs: SubaccountMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol
): SubaccountLiquidationPricesResult =>
  computeSubaccountLiquidationPricesFromMargin(
    computeSubaccountMarginFromInputs(inputs, marketsBySymbol),
    marketsBySymbol
  );

export type MarginSimulationMode = "cross" | "isolated";

export interface SimulatePositionFillInput {
  subaccount: SubaccountMarginInputs;
  symbol: string;
  side: "bid" | "ask";
  baseLots: string;
  priceTicks: string;
  feeQuoteLots?: string;
  marginMode?: MarginSimulationMode;
  isolatedCollateralBalanceQuoteLots?: string;
}

export interface SimulatedPositionFillResult {
  marginMode: MarginSimulationMode;
  symbol: string;
  priceTicks: string;
  priceUsd: number;
  realizedPnlQuoteLots: string;
  feeQuoteLots: string;
  projectedPosition: MarginPositionState | null;
  projectedSubaccountInput: SubaccountMarginInputs;
  margin: SubaccountMarginResult;
  liquidationPrice: MarketLiquidationPriceResult | null;
}

export type MarginSimulationAction =
  | {
      type: "fillPosition";
      symbol: string;
      side: "bid" | "ask";
      baseLots: string;
      priceTicks: string;
      feeQuoteLots?: string;
    }
  | {
      /**
       * Close part or all of the current position with a simulated fill. The
       * close size is `|position| * fractionBps / 10_000` (floor, default the
       * full position) and the side opposes the current position. When
       * `priceTicks` is omitted, the current scenario mark price is used.
       */
      type: "closePosition";
      symbol: string;
      fractionBps?: string;
      priceTicks?: string;
      feeQuoteLots?: string;
    }
  | {
      type: "placeLimitOrder";
      symbol: string;
      orderSequenceNumber: string;
      side: "bid" | "ask";
      priceTicks: string;
      sizeLots: string;
      initialSizeLots?: string;
      reduceOnly?: boolean;
      isStopLoss?: boolean;
      isStopLossDirection?: boolean;
    }
  | {
      type: "cancelOrder";
      symbol: string;
      orderSequenceNumber: string;
    }
  | {
      type: "cancelAllOrders";
      symbol?: string;
    }
  | {
      type: "adjustCollateral";
      quoteLotsDelta: string;
    }
  | {
      type: "setCollateral";
      quoteLots: string;
    }
  | {
      type: "settleFunding";
      symbol?: string;
    }
  | {
      type: "applyFundingPayment";
      symbol: string;
      quoteLots: string;
    }
  | {
      type: "setMarkPrice";
      symbol: string;
      markPriceTicks: string;
    }
  | {
      /**
       * Move the mark price by a signed basis-point delta relative to the
       * current scenario mark price: `new = current * (10_000 + delta) /
       * 10_000` (truncated). The result must stay a positive valid tick price.
       */
      type: "moveMarkPrice";
      symbol: string;
      percentDeltaBps: string;
    };

export type MarginSimulationActionReport =
  | {
      type: "fillPosition";
      symbol: string;
      priceTicks: string;
      priceUsd: number;
      realizedPnlQuoteLots: string;
      feeQuoteLots: string;
      settledFundingQuoteLots: string;
      projectedPosition: MarginPositionState | null;
    }
  | {
      type: "closePosition";
      symbol: string;
      closedBaseLots: string;
      priceTicks: string;
      priceUsd: number;
      realizedPnlQuoteLots: string;
      feeQuoteLots: string;
      settledFundingQuoteLots: string;
      projectedPosition: MarginPositionState | null;
    }
  | {
      type: "placeLimitOrder";
      symbol: string;
      orderSequenceNumber: string;
      settledFundingQuoteLots: string;
    }
  | {
      type: "cancelOrder";
      symbol: string;
      orderSequenceNumber: string;
      removed: boolean;
      settledFundingQuoteLots: string;
    }
  | {
      type: "cancelAllOrders";
      symbol?: string;
      removed: number | null;
      settledFundingQuoteLots: string;
    }
  | {
      type: "adjustCollateral";
      quoteLotsDelta: string;
    }
  | {
      type: "setCollateral";
      previousQuoteLots: string;
      newQuoteLots: string;
    }
  | {
      type: "settleFunding";
      symbol?: string;
      settledFundingQuoteLots: string;
    }
  | {
      type: "applyFundingPayment";
      symbol: string;
      fundingPaymentQuoteLots: string;
    }
  | {
      type: "setMarkPrice";
      symbol: string;
      previousMarkPriceTicks: string;
      newMarkPriceTicks: string;
    }
  | {
      type: "moveMarkPrice";
      symbol: string;
      percentDeltaBps: string;
      previousMarkPriceTicks: string;
      newMarkPriceTicks: string;
    };

export interface MarginSimulationDelta {
  collateralBalanceQuoteLots: string;
  effectiveCollateralQuoteLots: string;
  initialMarginQuoteLots: string;
  maintenanceMarginQuoteLots: string;
  unrealizedPnlQuoteLots: string;
  unsettledFundingQuoteLots: string;
}

export interface SimulateMarginInput {
  subaccount: SubaccountMarginInputs;
  actions: MarginSimulationAction[];
  marginMode?: MarginSimulationMode;
  isolatedSymbol?: string;
  isolatedCollateralBalanceQuoteLots?: string;
  /**
   * Mark prices (in ticks) to use instead of the market-parameter marks for
   * the whole simulation, including the `before` margin. Use this to compute
   * against caller-supplied prices (e.g. API price overrides). To model a mark
   * move as part of the scenario itself, use the `setMarkPrice` or
   * `moveMarkPrice` actions instead.
   */
  markPriceOverrides?: { [symbol: string]: string };
}

export interface SimulateMarginResult {
  marginMode: MarginSimulationMode;
  before: SubaccountMarginResult;
  after: SubaccountMarginResult;
  delta: MarginSimulationDelta;
  projectedSubaccountInput: SubaccountMarginInputs;
  liquidationPrices: SubaccountLiquidationPricesResult;
  actionReports: MarginSimulationActionReport[];
}

/**
 * A labeled action list applied on top of the shared base state in
 * `simulateMarginScenariosFromInputs`.
 */
export interface MarginScenario {
  label: string;
  actions: MarginSimulationAction[];
}

export interface SimulateMarginScenariosInput {
  subaccount: SubaccountMarginInputs;
  /** Actions applied once to form the shared baseline every scenario starts
   * from. */
  baseActions?: MarginSimulationAction[];
  scenarios: MarginScenario[];
  marginMode?: MarginSimulationMode;
  isolatedSymbol?: string;
  isolatedCollateralBalanceQuoteLots?: string;
  /** Mark prices (in ticks) to use instead of the market-parameter marks for
   * the whole batch, including the `before` and base margins. */
  markPriceOverrides?: { [symbol: string]: string };
}

/**
 * One scenario outcome. `simulation.before` is the shared post-base-action
 * margin, so `simulation.delta` isolates the scenario's own effect.
 */
export interface MarginScenarioResult {
  label: string;
  simulation: SimulateMarginResult;
}

export interface SimulateMarginScenariosResult {
  marginMode: MarginSimulationMode;
  /** Margin before base actions and scenarios (after mark-price overrides). */
  before: SubaccountMarginResult;
  /** Margin after base actions; every scenario branches from this state. */
  base: SubaccountMarginResult;
  baseActionReports: MarginSimulationActionReport[];
  scenarios: MarginScenarioResult[];
}

/**
 * Simulates a fully filled position change at `priceTicks` and recomputes the
 * resulting margin and liquidation price. The provided price is treated as the
 * exact fill price; slippage is intentionally ignored. Pass `feeQuoteLots`
 * when projecting a fill path that deducts trading fees from collateral.
 */
export const simulatePositionFillFromInputs = (
  input: SimulatePositionFillInput,
  marketsBySymbol: NormalizedMarketParamsBySymbol
): SimulatedPositionFillResult => {
  const marketParams = marketsBySymbol[input.symbol];
  if (!marketParams) {
    throw new Error(`Missing market params for symbol ${input.symbol}`);
  }
  const marginMode =
    input.marginMode ?? (marketParams.isolatedOnly ? "isolated" : "cross");

  const simulation = simulateMarginFromInputs(
    {
      subaccount: input.subaccount,
      marginMode,
      isolatedSymbol: input.symbol,
      isolatedCollateralBalanceQuoteLots:
        input.isolatedCollateralBalanceQuoteLots,
      actions: [
        {
          type: "fillPosition",
          symbol: input.symbol,
          side: input.side,
          baseLots: input.baseLots,
          priceTicks: input.priceTicks,
          feeQuoteLots: input.feeQuoteLots,
        },
      ],
    },
    marketsBySymbol
  );

  const fillReport = simulation.actionReports.find(
    (
      report
    ): report is Extract<
      MarginSimulationActionReport,
      { type: "fillPosition" }
    > => report.type === "fillPosition" && report.symbol === input.symbol
  );
  if (!fillReport) {
    throw new Error(
      `Missing fill simulation report for symbol ${input.symbol}`
    );
  }

  return {
    marginMode: simulation.marginMode,
    symbol: input.symbol,
    priceTicks: fillReport.priceTicks,
    priceUsd: fillReport.priceUsd,
    realizedPnlQuoteLots: fillReport.realizedPnlQuoteLots,
    feeQuoteLots: fillReport.feeQuoteLots,
    projectedPosition: fillReport.projectedPosition,
    projectedSubaccountInput: simulation.projectedSubaccountInput,
    margin: simulation.after,
    liquidationPrice:
      simulation.liquidationPrices.positions.find(
        (position) => position.symbol === input.symbol
      ) ?? null,
  };
};

export const simulateMarginFromInputs = (
  input: SimulateMarginInput,
  marketsBySymbol: NormalizedMarketParamsBySymbol
): SimulateMarginResult => {
  const scope = resolveSimulationScope(input, marketsBySymbol);
  const scenarioMarketsBySymbol =
    cloneNormalizedMarketsBySymbol(marketsBySymbol);
  applyMarkPriceOverrides(scenarioMarketsBySymbol, input.markPriceOverrides);
  const projectedSubaccountInput = buildScopedSubaccountInput(
    input.subaccount,
    scope
  );
  const before = computeSubaccountMarginFromInputs(
    projectedSubaccountInput,
    scenarioMarketsBySymbol
  );
  const actionReports: MarginSimulationActionReport[] = [];

  for (const action of input.actions) {
    applySimulationAction(
      projectedSubaccountInput,
      scenarioMarketsBySymbol,
      scope,
      action,
      actionReports
    );
  }

  const after = computeSubaccountMarginFromInputs(
    projectedSubaccountInput,
    scenarioMarketsBySymbol
  );

  return {
    marginMode: scope.marginMode,
    before,
    after,
    delta: computeMarginSimulationDelta(before, after),
    projectedSubaccountInput,
    liquidationPrices: computeSubaccountLiquidationPricesFromMargin(
      after,
      scenarioMarketsBySymbol
    ),
    actionReports,
  };
};

/**
 * Simulates independent labeled scenarios that all branch from one shared
 * baseline (`baseActions` applied to the subaccount). Each scenario's
 * `simulation.before` is the shared post-base margin, so scenario deltas
 * isolate the scenario's own effect. This maps 1:1 onto batched what-if API
 * requests (e.g. price ladders or order previews).
 */
export const simulateMarginScenariosFromInputs = (
  input: SimulateMarginScenariosInput,
  marketsBySymbol: NormalizedMarketParamsBySymbol
): SimulateMarginScenariosResult => {
  const baseActions = input.baseActions ?? [];
  const combinedActions = [
    ...baseActions,
    ...input.scenarios.flatMap((scenario) => scenario.actions),
  ];
  const scope = resolveSimulationScope(
    {
      subaccount: input.subaccount,
      actions: combinedActions,
      marginMode: input.marginMode,
      isolatedSymbol: input.isolatedSymbol,
      isolatedCollateralBalanceQuoteLots:
        input.isolatedCollateralBalanceQuoteLots,
    },
    marketsBySymbol
  );
  const baseMarketsBySymbol = cloneNormalizedMarketsBySymbol(marketsBySymbol);
  applyMarkPriceOverrides(baseMarketsBySymbol, input.markPriceOverrides);
  const baseSubaccountInput = buildScopedSubaccountInput(
    input.subaccount,
    scope
  );
  const before = computeSubaccountMarginFromInputs(
    baseSubaccountInput,
    baseMarketsBySymbol
  );

  const baseActionReports: MarginSimulationActionReport[] = [];
  for (const action of baseActions) {
    applySimulationAction(
      baseSubaccountInput,
      baseMarketsBySymbol,
      scope,
      action,
      baseActionReports
    );
  }
  const base = computeSubaccountMarginFromInputs(
    baseSubaccountInput,
    baseMarketsBySymbol
  );

  const scenarios: MarginScenarioResult[] = input.scenarios.map((scenario) => {
    const scenarioSubaccountInput = cloneSubaccountInput(baseSubaccountInput);
    const scenarioMarketsBySymbol =
      cloneNormalizedMarketsBySymbol(baseMarketsBySymbol);
    const actionReports: MarginSimulationActionReport[] = [];
    for (const action of scenario.actions) {
      applySimulationAction(
        scenarioSubaccountInput,
        scenarioMarketsBySymbol,
        scope,
        action,
        actionReports
      );
    }
    const after = computeSubaccountMarginFromInputs(
      scenarioSubaccountInput,
      scenarioMarketsBySymbol
    );

    return {
      label: scenario.label,
      simulation: {
        marginMode: scope.marginMode,
        before: base,
        after,
        delta: computeMarginSimulationDelta(base, after),
        projectedSubaccountInput: scenarioSubaccountInput,
        liquidationPrices: computeSubaccountLiquidationPricesFromMargin(
          after,
          scenarioMarketsBySymbol
        ),
        actionReports,
      },
    };
  });

  return {
    marginMode: scope.marginMode,
    before,
    base,
    baseActionReports,
    scenarios,
  };
};

const QUOTE_LOTS_PER_USD = 1_000_000;

interface SimulationScope {
  marginMode: MarginSimulationMode;
  isolatedSymbol?: string;
  isolatedCollateralBalanceQuoteLots?: string;
}

const resolveSimulationScope = (
  input: SimulateMarginInput,
  marketsBySymbol: NormalizedMarketParamsBySymbol
): SimulationScope => {
  const isolatedSymbol = input.isolatedSymbol ?? inferSingleActionSymbol(input);
  const inferredMarket = isolatedSymbol
    ? marketsBySymbol[isolatedSymbol]
    : undefined;
  const marginMode =
    input.marginMode ?? (inferredMarket?.isolatedOnly ? "isolated" : "cross");

  if (marginMode === "isolated" && !isolatedSymbol) {
    throw new Error(
      "isolatedSymbol is required when simulating isolated margin"
    );
  }

  for (const symbol of actionSymbols(input.actions)) {
    const marketParams = marketsBySymbol[symbol];
    if (!marketParams) {
      throw new Error(`Missing market params for symbol ${symbol}`);
    }
    if (marginMode === "cross" && marketParams.isolatedOnly) {
      throw new Error(`Market ${symbol} only supports isolated margin`);
    }
    if (marginMode === "isolated" && symbol !== isolatedSymbol) {
      throw new Error(
        `Isolated margin simulation for ${isolatedSymbol} cannot include action for ${symbol}`
      );
    }
  }

  return {
    marginMode,
    isolatedSymbol,
    isolatedCollateralBalanceQuoteLots:
      input.isolatedCollateralBalanceQuoteLots,
  };
};

const inferSingleActionSymbol = (
  input: SimulateMarginInput
): string | undefined => {
  const symbols = [...actionSymbols(input.actions)];
  return symbols.length === 1 ? symbols[0] : undefined;
};

const actionSymbols = (
  actions: readonly MarginSimulationAction[]
): Set<string> => {
  const symbols = new Set<string>();
  for (const action of actions) {
    switch (action.type) {
      case "adjustCollateral":
      case "cancelAllOrders":
      case "setCollateral":
      case "settleFunding":
        if ("symbol" in action && action.symbol) {
          symbols.add(action.symbol);
        }
        break;
      case "applyFundingPayment":
      case "cancelOrder":
      case "closePosition":
      case "fillPosition":
      case "moveMarkPrice":
      case "placeLimitOrder":
      case "setMarkPrice":
        symbols.add(action.symbol);
        break;
      default:
        assertNever(action);
    }
  }
  return symbols;
};

const cloneNormalizedMarketsBySymbol = (
  marketsBySymbol: NormalizedMarketParamsBySymbol
): NormalizedMarketParamsBySymbol => {
  const clone: NormalizedMarketParamsBySymbol = {};
  for (const [symbol, marketParams] of Object.entries(marketsBySymbol)) {
    clone[symbol] = {
      ...marketParams,
      leverageTiers: marketParams.leverageTiers.map((tier) => ({ ...tier })),
      riskFactors: { ...marketParams.riskFactors },
    };
  }
  return clone;
};

const buildScopedSubaccountInput = (
  subaccount: SubaccountMarginInputs,
  scope: SimulationScope
): SubaccountMarginInputs => {
  if (scope.marginMode === "isolated") {
    const symbol = requireIsolatedSymbol(scope);
    const currentMarket = subaccount.markets.find(
      (marketInput) => marketInput.symbol === symbol
    );
    return {
      subaccountIndex: subaccount.subaccountIndex,
      collateralBalanceQuoteLots:
        scope.isolatedCollateralBalanceQuoteLots ??
        subaccount.collateralBalanceQuoteLots,
      markets: [cloneMarketInput(currentMarket ?? { symbol })],
    };
  }

  return {
    subaccountIndex: subaccount.subaccountIndex,
    collateralBalanceQuoteLots: subaccount.collateralBalanceQuoteLots,
    markets: subaccount.markets.map((marketInput) =>
      cloneMarketInput(marketInput)
    ),
  };
};

const applySimulationAction = (
  subaccount: SubaccountMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  scope: SimulationScope,
  action: MarginSimulationAction,
  actionReports: MarginSimulationActionReport[]
): void => {
  assertActionAllowedInScope(action, scope);

  switch (action.type) {
    case "fillPosition":
      actionReports.push(
        applyFillPositionAction(subaccount, marketsBySymbol, scope, action)
      );
      return;
    case "closePosition":
      actionReports.push(
        applyClosePositionAction(subaccount, marketsBySymbol, scope, action)
      );
      return;
    case "placeLimitOrder":
      actionReports.push(applyPlaceLimitOrderAction(subaccount, scope, action));
      return;
    case "cancelOrder":
      actionReports.push(applyCancelOrderAction(subaccount, scope, action));
      return;
    case "cancelAllOrders":
      actionReports.push(applyCancelAllOrdersAction(subaccount, scope, action));
      return;
    case "adjustCollateral":
      actionReports.push(applyAdjustCollateralAction(subaccount, action));
      return;
    case "setCollateral":
      actionReports.push(applySetCollateralAction(subaccount, action));
      return;
    case "settleFunding":
      actionReports.push(applySettleFundingAction(subaccount, scope, action));
      return;
    case "applyFundingPayment":
      actionReports.push(applyFundingPaymentAction(subaccount, action));
      return;
    case "setMarkPrice":
      actionReports.push(applySetMarkPriceAction(marketsBySymbol, action));
      return;
    case "moveMarkPrice":
      actionReports.push(applyMoveMarkPriceAction(marketsBySymbol, action));
      return;
    default:
      assertNever(action);
  }
};

const applyFillPositionAction = (
  subaccount: SubaccountMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  scope: SimulationScope,
  action: Extract<MarginSimulationAction, { type: "fillPosition" }>
): Extract<MarginSimulationActionReport, { type: "fillPosition" }> => {
  const marketParams = requireMarketParams(marketsBySymbol, action.symbol);
  const fillBaseLots = requirePositiveBigInt(
    action.baseLots,
    "Simulated fill baseLots must be positive"
  );
  const fillPriceTicks = requirePositiveBigInt(
    action.priceTicks,
    "Simulated fill priceTicks must be positive"
  );
  const feeQuoteLots = requireNonNegativeBigInt(
    action.feeQuoteLots ?? "0",
    "Simulated fill feeQuoteLots must be non-negative"
  );
  const settledFundingQuoteLots = settleFundingForActionScope(
    subaccount,
    scope
  );
  const marketInput = getOrCreateMarketInput(subaccount, action.symbol);
  const currentPosition = marketInput.position;
  const currentBaseLots = currentPosition
    ? toBigInt(currentPosition.basePositionLots)
    : 0n;
  const currentVirtualQuoteLots = currentPosition
    ? toBigInt(currentPosition.virtualQuotePositionLots)
    : 0n;
  const signedDeltaBaseLots =
    action.side === "bid" ? fillBaseLots : -fillBaseLots;
  const projection = projectPositionAfterFill(
    currentBaseLots,
    currentVirtualQuoteLots,
    signedDeltaBaseLots,
    fillPriceTicks * marketParams.tickSize
  );
  const projectedPosition = buildProjectedPosition(
    projection.projectedBaseLots,
    projection.projectedVirtualQuoteLots,
    projection.resetAccumulatedFunding,
    currentPosition,
    marketParams
  );

  subaccount.collateralBalanceQuoteLots = (
    toBigInt(subaccount.collateralBalanceQuoteLots) +
    projection.realizedPnlQuoteLots -
    feeQuoteLots
  ).toString();
  marketInput.position = projectedPosition ?? undefined;

  return {
    type: "fillPosition",
    symbol: action.symbol,
    priceTicks: fillPriceTicks.toString(),
    priceUsd: priceTicksToUsd(fillPriceTicks, marketParams),
    realizedPnlQuoteLots: projection.realizedPnlQuoteLots.toString(),
    feeQuoteLots: feeQuoteLots.toString(),
    settledFundingQuoteLots: settledFundingQuoteLots.toString(),
    projectedPosition,
  };
};

const BPS_DENOMINATOR = 10_000n;
const MAX_TICKS = 4_294_967_295n; // u32::MAX, mirrors the Rust Ticks bounds.

const applyClosePositionAction = (
  subaccount: SubaccountMarginInputs,
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  scope: SimulationScope,
  action: Extract<MarginSimulationAction, { type: "closePosition" }>
): Extract<MarginSimulationActionReport, { type: "closePosition" }> => {
  const marketParams = requireMarketParams(marketsBySymbol, action.symbol);
  const position = getMarketInput(subaccount, action.symbol)?.position;
  if (!position) {
    throw new Error(
      `Cannot close position for ${action.symbol} without an open position`
    );
  }

  const fractionBps = requirePositiveBigInt(
    action.fractionBps ?? BPS_DENOMINATOR.toString(),
    "Close position fractionBps must be positive"
  );
  if (fractionBps > BPS_DENOMINATOR) {
    throw new Error(
      `Close position fractionBps must be at most ${BPS_DENOMINATOR}`
    );
  }
  const currentBaseLots = toBigInt(position.basePositionLots);
  const closedBaseLots =
    (absBigInt(currentBaseLots) * fractionBps) / BPS_DENOMINATOR;
  if (closedBaseLots === 0n) {
    throw new Error(
      `Close fraction ${fractionBps} bps of position for ${action.symbol} rounds to zero base lots`
    );
  }

  const fillReport = applyFillPositionAction(
    subaccount,
    marketsBySymbol,
    scope,
    {
      type: "fillPosition",
      symbol: action.symbol,
      side: currentBaseLots > 0n ? "ask" : "bid",
      baseLots: closedBaseLots.toString(),
      priceTicks: action.priceTicks ?? marketParams.markPriceTicks.toString(),
      feeQuoteLots: action.feeQuoteLots,
    }
  );

  return {
    type: "closePosition",
    symbol: fillReport.symbol,
    closedBaseLots: closedBaseLots.toString(),
    priceTicks: fillReport.priceTicks,
    priceUsd: fillReport.priceUsd,
    realizedPnlQuoteLots: fillReport.realizedPnlQuoteLots,
    feeQuoteLots: fillReport.feeQuoteLots,
    settledFundingQuoteLots: fillReport.settledFundingQuoteLots,
    projectedPosition: fillReport.projectedPosition,
  };
};

const applyMoveMarkPriceAction = (
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  action: Extract<MarginSimulationAction, { type: "moveMarkPrice" }>
): Extract<MarginSimulationActionReport, { type: "moveMarkPrice" }> => {
  const marketParams = requireMarketParams(marketsBySymbol, action.symbol);
  const percentDeltaBps = toBigInt(action.percentDeltaBps);
  const newMarkPriceTicks =
    (marketParams.markPriceTicks * (BPS_DENOMINATOR + percentDeltaBps)) /
    BPS_DENOMINATOR;
  if (newMarkPriceTicks <= 0n || newMarkPriceTicks > MAX_TICKS) {
    throw new Error(
      `Mark price move of ${percentDeltaBps} bps for ${action.symbol} does not produce a positive in-range tick price`
    );
  }

  const setReport = applySetMarkPriceAction(marketsBySymbol, {
    type: "setMarkPrice",
    symbol: action.symbol,
    markPriceTicks: newMarkPriceTicks.toString(),
  });

  return {
    type: "moveMarkPrice",
    symbol: setReport.symbol,
    percentDeltaBps: percentDeltaBps.toString(),
    previousMarkPriceTicks: setReport.previousMarkPriceTicks,
    newMarkPriceTicks: setReport.newMarkPriceTicks,
  };
};

const applyMarkPriceOverrides = (
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  markPriceOverrides: { [symbol: string]: string } | undefined
): void => {
  for (const [symbol, markPriceTicks] of Object.entries(
    markPriceOverrides ?? {}
  )) {
    const marketParams = requireMarketParams(marketsBySymbol, symbol);
    marketParams.markPriceTicks = requirePositiveBigInt(
      markPriceTicks,
      `Mark price override for ${symbol} must be positive`
    );
  }
};

const applyPlaceLimitOrderAction = (
  subaccount: SubaccountMarginInputs,
  scope: SimulationScope,
  action: Extract<MarginSimulationAction, { type: "placeLimitOrder" }>
): Extract<MarginSimulationActionReport, { type: "placeLimitOrder" }> => {
  const settledFundingQuoteLots = settleFundingForActionScope(
    subaccount,
    scope
  );
  const marketInput = getOrCreateMarketInput(subaccount, action.symbol);
  ensureOrderListCanBeMutated(marketInput, "place limit order");
  const orders = marketInput.limitOrders ?? [];
  const filteredOrders = orders.filter(
    (order) => order.orderSequenceNumber !== action.orderSequenceNumber
  );
  filteredOrders.push({
    orderSequenceNumber: action.orderSequenceNumber,
    side: action.side,
    priceTicks: requirePositiveBigInt(
      action.priceTicks,
      "Limit order priceTicks must be positive"
    ).toString(),
    sizeRemainingLots: requirePositiveBigInt(
      action.sizeLots,
      "Limit order sizeLots must be positive"
    ).toString(),
    initialSizeLots: requirePositiveBigInt(
      action.initialSizeLots ?? action.sizeLots,
      "Limit order initialSizeLots must be positive"
    ).toString(),
    reduceOnly: action.reduceOnly ?? false,
    isStopLoss: action.isStopLoss,
    isStopLossDirection: action.isStopLossDirection,
    status: "active",
  });
  marketInput.limitOrders = filteredOrders;
  marketInput.limitOrderMargin = undefined;

  return {
    type: "placeLimitOrder",
    symbol: action.symbol,
    orderSequenceNumber: action.orderSequenceNumber,
    settledFundingQuoteLots: settledFundingQuoteLots.toString(),
  };
};

const applyCancelOrderAction = (
  subaccount: SubaccountMarginInputs,
  scope: SimulationScope,
  action: Extract<MarginSimulationAction, { type: "cancelOrder" }>
): Extract<MarginSimulationActionReport, { type: "cancelOrder" }> => {
  const settledFundingQuoteLots = settleFundingForActionScope(
    subaccount,
    scope
  );
  const marketInput = getMarketInput(subaccount, action.symbol);
  if (!marketInput) {
    return {
      type: "cancelOrder",
      symbol: action.symbol,
      orderSequenceNumber: action.orderSequenceNumber,
      removed: false,
      settledFundingQuoteLots: settledFundingQuoteLots.toString(),
    };
  }

  ensureOrderListCanBeMutated(marketInput, "cancel order");
  const orders = marketInput.limitOrders ?? [];
  const filteredOrders = orders.filter(
    (order) => order.orderSequenceNumber !== action.orderSequenceNumber
  );
  marketInput.limitOrders = filteredOrders;
  marketInput.limitOrderMargin = undefined;

  return {
    type: "cancelOrder",
    symbol: action.symbol,
    orderSequenceNumber: action.orderSequenceNumber,
    removed: filteredOrders.length !== orders.length,
    settledFundingQuoteLots: settledFundingQuoteLots.toString(),
  };
};

const applyCancelAllOrdersAction = (
  subaccount: SubaccountMarginInputs,
  scope: SimulationScope,
  action: Extract<MarginSimulationAction, { type: "cancelAllOrders" }>
): Extract<MarginSimulationActionReport, { type: "cancelAllOrders" }> => {
  const settledFundingQuoteLots = settleFundingForActionScope(
    subaccount,
    scope
  );
  let removed = 0;
  let removedCountKnown = true;
  const markets = action.symbol
    ? subaccount.markets.filter(
        (marketInput) => marketInput.symbol === action.symbol
      )
    : subaccount.markets;

  for (const marketInput of markets) {
    if (marketInput.limitOrders) {
      removed += marketInput.limitOrders.length;
    } else if (marketInput.limitOrderMargin) {
      removedCountKnown = false;
    }
    marketInput.limitOrders = [];
    marketInput.limitOrderMargin = undefined;
  }

  return {
    type: "cancelAllOrders",
    symbol: action.symbol,
    removed: removedCountKnown ? removed : null,
    settledFundingQuoteLots: settledFundingQuoteLots.toString(),
  };
};

const applyAdjustCollateralAction = (
  subaccount: SubaccountMarginInputs,
  action: Extract<MarginSimulationAction, { type: "adjustCollateral" }>
): Extract<MarginSimulationActionReport, { type: "adjustCollateral" }> => {
  const quoteLotsDelta = toBigInt(action.quoteLotsDelta);
  subaccount.collateralBalanceQuoteLots = (
    toBigInt(subaccount.collateralBalanceQuoteLots) + quoteLotsDelta
  ).toString();

  return {
    type: "adjustCollateral",
    quoteLotsDelta: quoteLotsDelta.toString(),
  };
};

const applySetCollateralAction = (
  subaccount: SubaccountMarginInputs,
  action: Extract<MarginSimulationAction, { type: "setCollateral" }>
): Extract<MarginSimulationActionReport, { type: "setCollateral" }> => {
  const previousQuoteLots = subaccount.collateralBalanceQuoteLots;
  const newQuoteLots = toBigInt(action.quoteLots).toString();
  subaccount.collateralBalanceQuoteLots = newQuoteLots;

  return {
    type: "setCollateral",
    previousQuoteLots,
    newQuoteLots,
  };
};

const applySettleFundingAction = (
  subaccount: SubaccountMarginInputs,
  scope: SimulationScope,
  action: Extract<MarginSimulationAction, { type: "settleFunding" }>
): Extract<MarginSimulationActionReport, { type: "settleFunding" }> => ({
  type: "settleFunding",
  symbol: action.symbol,
  settledFundingQuoteLots: settleFundingForActionScope(
    subaccount,
    scope,
    action.symbol
  ).toString(),
});

const applyFundingPaymentAction = (
  subaccount: SubaccountMarginInputs,
  action: Extract<MarginSimulationAction, { type: "applyFundingPayment" }>
): Extract<MarginSimulationActionReport, { type: "applyFundingPayment" }> => {
  const marketInput = getMarketInput(subaccount, action.symbol);
  if (!marketInput?.position) {
    throw new Error(
      `Cannot apply funding payment for ${action.symbol} without an open position`
    );
  }
  const fundingPayment = toBigInt(action.quoteLots);
  marketInput.position = {
    ...marketInput.position,
    unsettledFundingQuoteLots: (
      toBigInt(marketInput.position.unsettledFundingQuoteLots) + fundingPayment
    ).toString(),
  };

  return {
    type: "applyFundingPayment",
    symbol: action.symbol,
    fundingPaymentQuoteLots: fundingPayment.toString(),
  };
};

const applySetMarkPriceAction = (
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  action: Extract<MarginSimulationAction, { type: "setMarkPrice" }>
): Extract<MarginSimulationActionReport, { type: "setMarkPrice" }> => {
  const marketParams = requireMarketParams(marketsBySymbol, action.symbol);
  const previousMarkPriceTicks = marketParams.markPriceTicks.toString();
  marketParams.markPriceTicks = requirePositiveBigInt(
    action.markPriceTicks,
    "markPriceTicks must be positive"
  );

  return {
    type: "setMarkPrice",
    symbol: action.symbol,
    previousMarkPriceTicks,
    newMarkPriceTicks: marketParams.markPriceTicks.toString(),
  };
};

const computeMarginSimulationDelta = (
  before: SubaccountMarginResult,
  after: SubaccountMarginResult
): MarginSimulationDelta => ({
  collateralBalanceQuoteLots: subtractQuoteLotStrings(
    after.margin.collateralBalanceQuoteLots,
    before.margin.collateralBalanceQuoteLots
  ),
  effectiveCollateralQuoteLots: subtractQuoteLotStrings(
    after.margin.effectiveCollateralQuoteLots,
    before.margin.effectiveCollateralQuoteLots
  ),
  initialMarginQuoteLots: subtractQuoteLotStrings(
    after.margin.initialMarginQuoteLots,
    before.margin.initialMarginQuoteLots
  ),
  maintenanceMarginQuoteLots: subtractQuoteLotStrings(
    after.margin.maintenanceMarginQuoteLots,
    before.margin.maintenanceMarginQuoteLots
  ),
  unrealizedPnlQuoteLots: subtractQuoteLotStrings(
    after.margin.unrealizedPnlQuoteLots,
    before.margin.unrealizedPnlQuoteLots
  ),
  unsettledFundingQuoteLots: subtractQuoteLotStrings(
    after.margin.unsettledFundingQuoteLots,
    before.margin.unsettledFundingQuoteLots
  ),
});

const settleFundingForActionScope = (
  subaccount: SubaccountMarginInputs,
  scope: SimulationScope,
  symbol?: string
): bigint => {
  const targetSymbol =
    scope.marginMode === "isolated" ? requireIsolatedSymbol(scope) : symbol;
  const markets =
    targetSymbol === undefined
      ? subaccount.markets
      : subaccount.markets.filter(
          (marketInput) => marketInput.symbol === targetSymbol
        );
  let settledFundingQuoteLots = 0n;
  for (const marketInput of markets) {
    if (!marketInput.position) {
      continue;
    }
    const unsettledFundingQuoteLots = toBigInt(
      marketInput.position.unsettledFundingQuoteLots
    );
    settledFundingQuoteLots += unsettledFundingQuoteLots;
    marketInput.position = {
      ...marketInput.position,
      unsettledFundingQuoteLots: "0",
    };
  }

  subaccount.collateralBalanceQuoteLots = (
    toBigInt(subaccount.collateralBalanceQuoteLots) + settledFundingQuoteLots
  ).toString();
  return settledFundingQuoteLots;
};

const getOrCreateMarketInput = (
  subaccount: SubaccountMarginInputs,
  symbol: string
): MarketMarginInputs => {
  const existing = getMarketInput(subaccount, symbol);
  if (existing) {
    return existing;
  }
  const created: MarketMarginInputs = { symbol };
  subaccount.markets.push(created);
  return created;
};

const getMarketInput = (
  subaccount: SubaccountMarginInputs,
  symbol: string
): MarketMarginInputs | undefined =>
  subaccount.markets.find((marketInput) => marketInput.symbol === symbol);

const ensureOrderListCanBeMutated = (
  marketInput: MarketMarginInputs,
  action: string
): void => {
  if (marketInput.limitOrderMargin && marketInput.limitOrders === undefined) {
    throw new Error(
      `Cannot ${action} for ${marketInput.symbol} when only aggregate limitOrderMargin is provided`
    );
  }
};

const assertActionAllowedInScope = (
  action: MarginSimulationAction,
  scope: SimulationScope
): void => {
  if (scope.marginMode !== "isolated") {
    return;
  }
  const isolatedSymbol = requireIsolatedSymbol(scope);
  if ("symbol" in action && action.symbol && action.symbol !== isolatedSymbol) {
    throw new Error(
      `Isolated margin simulation for ${isolatedSymbol} cannot include action for ${action.symbol}`
    );
  }
};

const requireIsolatedSymbol = (scope: SimulationScope): string => {
  if (!scope.isolatedSymbol) {
    throw new Error(
      "isolatedSymbol is required for isolated margin simulation"
    );
  }
  return scope.isolatedSymbol;
};

const requireMarketParams = (
  marketsBySymbol: NormalizedMarketParamsBySymbol,
  symbol: string
): NormalizedMarketParams => {
  const marketParams = marketsBySymbol[symbol];
  if (!marketParams) {
    throw new Error(`Missing market params for symbol ${symbol}`);
  }
  return marketParams;
};

const requirePositiveBigInt = (value: string, errorMessage: string): bigint => {
  const parsed = toBigInt(value);
  if (parsed <= 0n) {
    throw new Error(errorMessage);
  }
  return parsed;
};

const requireNonNegativeBigInt = (
  value: string,
  errorMessage: string
): bigint => {
  const parsed = toBigInt(value);
  if (parsed < 0n) {
    throw new Error(errorMessage);
  }
  return parsed;
};

const subtractQuoteLotStrings = (lhs: string, rhs: string): string =>
  (toBigInt(lhs) - toBigInt(rhs)).toString();

const assertNever = (value: never): never => {
  throw new Error(
    `Unexpected margin simulation action: ${JSON.stringify(value)}`
  );
};

const projectPositionAfterFill = (
  currentBaseLots: bigint,
  currentVirtualQuoteLots: bigint,
  deltaBaseLots: bigint,
  fillQuoteLotsPerBaseLot: bigint
): {
  projectedBaseLots: bigint;
  projectedVirtualQuoteLots: bigint;
  realizedPnlQuoteLots: bigint;
  resetAccumulatedFunding: boolean;
} => {
  const fillVirtualQuoteLots = -deltaBaseLots * fillQuoteLotsPerBaseLot;

  if (
    currentBaseLots === 0n ||
    signBigInt(currentBaseLots) === signBigInt(deltaBaseLots)
  ) {
    return {
      projectedBaseLots: currentBaseLots + deltaBaseLots,
      projectedVirtualQuoteLots: currentVirtualQuoteLots + fillVirtualQuoteLots,
      realizedPnlQuoteLots: 0n,
      resetAccumulatedFunding: currentBaseLots === 0n,
    };
  }

  const currentAbsBaseLots = absBigInt(currentBaseLots);
  const deltaAbsBaseLots = absBigInt(deltaBaseLots);
  const baseLotsToClose =
    deltaAbsBaseLots > currentAbsBaseLots ? -currentBaseLots : deltaBaseLots;
  const baseLotsRemainingFromFill = deltaBaseLots - baseLotsToClose;
  const quoteLotsToClosePosition =
    (fillVirtualQuoteLots * baseLotsToClose) / deltaBaseLots;
  const quoteLotsRemainingFromFill =
    fillVirtualQuoteLots - quoteLotsToClosePosition;
  const quoteLotsToEnterPosition =
    (currentVirtualQuoteLots * baseLotsToClose) / currentBaseLots;
  let projectedBaseLots = currentBaseLots + baseLotsToClose;
  let projectedVirtualQuoteLots =
    currentVirtualQuoteLots + quoteLotsToEnterPosition;
  let realizedPnlQuoteLots =
    quoteLotsToClosePosition - quoteLotsToEnterPosition;
  const flipped = projectedBaseLots === 0n;

  if (flipped) {
    realizedPnlQuoteLots += projectedVirtualQuoteLots;
    projectedBaseLots = baseLotsRemainingFromFill;
    projectedVirtualQuoteLots = quoteLotsRemainingFromFill;
  }

  return {
    projectedBaseLots,
    projectedVirtualQuoteLots,
    realizedPnlQuoteLots,
    resetAccumulatedFunding: flipped,
  };
};

const buildProjectedPosition = (
  projectedBaseLots: bigint,
  projectedVirtualQuoteLots: bigint,
  resetAccumulatedFunding: boolean,
  currentPosition: MarginPositionState | undefined,
  marketParams: NormalizedMarketParams
): MarginPositionState | null => {
  if (projectedBaseLots === 0n) {
    return null;
  }

  return {
    basePositionLots: projectedBaseLots.toString(),
    virtualQuotePositionLots: projectedVirtualQuoteLots.toString(),
    entryPriceTicks: entryPriceTicksForPosition(
      projectedBaseLots,
      projectedVirtualQuoteLots,
      marketParams
    ).toString(),
    unsettledFundingQuoteLots: "0",
    accumulatedFundingQuoteLots: resetAccumulatedFunding
      ? "0"
      : (currentPosition?.accumulatedFundingQuoteLots ?? "0"),
  };
};

const cloneSubaccountInput = (
  subaccount: SubaccountMarginInputs
): SubaccountMarginInputs => ({
  subaccountIndex: subaccount.subaccountIndex,
  collateralBalanceQuoteLots: subaccount.collateralBalanceQuoteLots,
  markets: subaccount.markets.map((marketInput) =>
    cloneMarketInput(marketInput)
  ),
});

const cloneMarketInput = (
  marketInput: MarketMarginInputs
): MarketMarginInputs => ({
  symbol: marketInput.symbol,
  position: marketInput.position ? { ...marketInput.position } : undefined,
  limitOrderMargin: marketInput.limitOrderMargin
    ? { ...marketInput.limitOrderMargin }
    : undefined,
  limitOrders: marketInput.limitOrders?.map((order) => ({ ...order })),
});

const priceTicksToUsd = (
  priceTicks: bigint,
  marketParams: NormalizedMarketParams
): number =>
  (Number(priceTicks * marketParams.tickSize) *
    Math.pow(10, marketParams.baseLotDecimals)) /
  QUOTE_LOTS_PER_USD;

const entryPriceTicksForPosition = (
  basePositionLots: bigint,
  virtualQuotePositionLots: bigint,
  marketParams: NormalizedMarketParams
): bigint => {
  const absBasePositionLots = absBigInt(basePositionLots);
  if (absBasePositionLots === 0n) {
    return 0n;
  }

  return roundDivBigInt(
    absBigInt(virtualQuotePositionLots),
    absBasePositionLots * marketParams.tickSize
  );
};

const roundDivBigInt = (numerator: bigint, denominator: bigint): bigint => {
  if (denominator <= 0n) {
    throw new Error("Cannot divide by a non-positive denominator");
  }

  return (numerator + denominator / 2n) / denominator;
};

const signBigInt = (value: bigint): bigint => {
  if (value > 0n) {
    return 1n;
  }
  if (value < 0n) {
    return -1n;
  }
  return 0n;
};

const filterActiveOrders = (
  orders: LimitOrderMarginInput[]
): LimitOrderMarginInput[] =>
  orders.filter((order) => (order.status ? order.status === "active" : true));

const resolveLimitOrderMarginState = (
  orders: LimitOrderMarginInput[],
  limitOrderMargin: LimitOrderMarginState | undefined,
  basePositionLots: bigint
): LimitOrderMarginState | undefined => {
  if (limitOrderMargin) {
    return limitOrderMargin;
  }
  if (orders.length === 0) {
    return undefined;
  }
  return buildLimitOrderMarginStateFromOrders(orders, basePositionLots);
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
    input.limitOrderMargin,
    basePositionLots
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

  // Reduce-only orders still rest on the book, so the fill-loss reserve is
  // computed over the total base lots on each side, using the best resting price.
  const fillLossInputs: FillLossInputs = limitOrderState
    ? {
        totalBidBaseLots:
          totalBid + toBigInt(limitOrderState.totalReduceOnlyBidBaseLots),
        totalAskBaseLots:
          totalAsk + toBigInt(limitOrderState.totalReduceOnlyAskBaseLots),
        highestBid: toBigInt(limitOrderState.highestBid),
        lowestAsk: toBigInt(limitOrderState.lowestAsk),
        tickSize: marketParams.tickSize,
      }
    : {
        totalBidBaseLots: 0n,
        totalAskBaseLots: 0n,
        highestBid: 0n,
        lowestAsk: 0n,
        tickSize: marketParams.tickSize,
      };

  const initialMargin = initialMarginForAsset(
    basePositionLots,
    totalBid,
    totalAsk,
    assetUnitPrice,
    leverageTiers,
    false,
    fillLossInputs
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
          fillLossInputs,
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
    true,
    fillLossInputs
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

/**
 * Inputs for the adverse-fill-loss reserve. `totalBid/AskBaseLots` include
 * reduce-only orders (they still rest on the book); the tick prices are the best
 * resting price on each side (0 when there are no orders that side).
 */
interface FillLossInputs {
  totalBidBaseLots: bigint;
  totalAskBaseLots: bigint;
  highestBid: bigint;
  lowestAsk: bigint;
  tickSize: bigint;
}

/**
 * Reserve for the immediate mark-to-market loss if resting orders on `side` were
 * to fill at their limit price. A resting bid above mark (or ask below mark)
 * fills in-the-money for the counterparty, so reserve `size * |limitPrice -
 * markPrice|`. Orders resting on the passive side of mark contribute nothing.
 * Mirrors `limit_order_fill_loss` in program-core/exchange/src/margin.rs.
 */
const limitOrderFillLoss = (
  side: "bid" | "ask",
  size: bigint,
  limitPriceTicks: bigint,
  markPrice: bigint,
  tickSize: bigint
): bigint => {
  if (limitPriceTicks === 0n) {
    return 0n;
  }
  const limitPrice = limitPriceTicks * tickSize;
  let priceDifference: bigint;
  if (side === "bid") {
    if (limitPrice <= markPrice) {
      return 0n;
    }
    priceDifference = limitPrice - markPrice;
  } else {
    if (limitPrice >= markPrice) {
      return 0n;
    }
    priceDifference = markPrice - limitPrice;
  }
  return size * priceDifference;
};

const initialMarginForAsset = (
  position: bigint,
  totalBid: bigint,
  totalAsk: bigint,
  assetUnitPrice: bigint,
  tiers: LeverageTier[],
  bypassRiskFactor: boolean,
  fillLoss?: FillLossInputs,
  orderLeverageLimit?: bigint
): bigint => {
  const totalBidAll = fillLoss?.totalBidBaseLots ?? 0n;
  const totalAskAll = fillLoss?.totalAskBaseLots ?? 0n;
  if (
    position === 0n &&
    totalBid === 0n &&
    totalAsk === 0n &&
    totalBidAll === 0n &&
    totalAskAll === 0n
  ) {
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

  if (fillLoss) {
    const bidFillLoss =
      totalBidAll > 0n
        ? limitOrderFillLoss(
            "bid",
            totalBidAll,
            fillLoss.highestBid,
            assetUnitPrice,
            fillLoss.tickSize
          )
        : 0n;
    const askFillLoss =
      totalAskAll > 0n
        ? limitOrderFillLoss(
            "ask",
            totalAskAll,
            fillLoss.lowestAsk,
            assetUnitPrice,
            fillLoss.tickSize
          )
        : 0n;
    collateralRequired += bidFillLoss + askFillLoss;
  }

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
