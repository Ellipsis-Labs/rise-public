import { Side } from "../primitives/Side";
import {
  cloneSubaccountInput,
  computeSubaccountMarginFromInputs,
  ensureOrderListCanBeMutated,
  getOrCreateMarketInput,
} from "./compute";
import { absBigInt, toBigInt } from "./math";
import { buildNormalizedMarketParamsBySymbol } from "./normalize";
import type { NormalizedMarketParamsBySymbol } from "./normalize";
import {
  buildSubaccountMarginInputsFromSnapshot,
  type MarginSnapshotSubaccount,
} from "./snapshot";
import type {
  LimitOrderMarginInput,
  MarginCalculationOptions,
  MarketParams,
  SubaccountMarginInputs,
  SubaccountMarginResult,
} from "./types";

export type MarginMarketsInput =
  | readonly MarketParams[]
  | NormalizedMarketParamsBySymbol;

export type DraftOrderMarginInput = {
  symbol: string;
  side: Side | "bid" | "ask";
  orderType: "market" | "limit";
  priceTicks: string | number | bigint;
  sizeBaseLots: string | number | bigint;
  reduceOnly?: boolean;
};

export type DraftOrderMarginRequirementResult = {
  /**
   * The value product surfaces should show/use.
   * This is adjusted when order leverage prefs apply, otherwise protocol.
   */
  marginRequirementQuoteLots: string;
  /** Protocol margin without order-leverage display adjustment. */
  protocolMarginRequirementQuoteLots: string;
  /** Present only when order leverage prefs change the result. */
  orderLeverageAdjustedMarginRequirementQuoteLots?: string;
};

const DRAFT_ORDER_SEQUENCE_NUMBER = "__rise_draft_order__";

const isMarketParamsArray = (
  markets: MarginMarketsInput
): markets is readonly MarketParams[] => Array.isArray(markets);

const normalizeMarginMarketsInput = (
  markets: MarginMarketsInput
): NormalizedMarketParamsBySymbol =>
  isMarketParamsArray(markets)
    ? buildNormalizedMarketParamsBySymbol(markets)
    : markets;

const normalizeDraftSide = (
  side: DraftOrderMarginInput["side"]
): "bid" | "ask" => {
  if (side === "bid" || side === Side.Bid) {
    return "bid";
  }
  if (side === "ask" || side === Side.Ask) {
    return "ask";
  }
  throw new Error('Invalid draft order side. Expected "bid" or "ask".');
};

const requirePositiveDraftPriceTicks = (
  priceTicks: DraftOrderMarginInput["priceTicks"]
): void => {
  if (toBigInt(priceTicks) <= 0n) {
    throw new Error("Draft order priceTicks must be positive");
  }
};

const buildSubaccountInputsWithDraftLimitOrder = (
  subaccount: SubaccountMarginInputs,
  draftOrder: DraftOrderMarginInput,
  sizeBaseLots: bigint
): SubaccountMarginInputs => {
  const projected = cloneSubaccountInput(subaccount);
  const marketInput = getOrCreateMarketInput(projected, draftOrder.symbol);
  ensureOrderListCanBeMutated(marketInput, "project a draft limit order");
  const order: LimitOrderMarginInput = {
    orderSequenceNumber: DRAFT_ORDER_SEQUENCE_NUMBER,
    side: normalizeDraftSide(draftOrder.side),
    priceTicks: toBigInt(draftOrder.priceTicks).toString(),
    sizeRemainingLots: sizeBaseLots.toString(),
    initialSizeLots: sizeBaseLots.toString(),
    reduceOnly: draftOrder.reduceOnly ?? false,
    isStopLoss: false,
    isStopLossDirection: false,
    status: "active",
  };

  marketInput.limitOrders = [...(marketInput.limitOrders ?? []), order];
  // The canonical calculator rebuilds this aggregate from complete order rows,
  // the current position, and the current mark price.
  marketInput.limitOrderMargin = undefined;
  return projected;
};

/**
 * Draft order sizing only needs the initial-margin delta, so market drafts
 * intentionally project a base-lot delta without modeling realized PnL,
 * funding settlement, or virtual quote changes. Use the action simulator for
 * full post-fill account-state projections.
 */
const buildSubaccountInputsWithDraftMarketFill = (
  subaccount: SubaccountMarginInputs,
  draftOrder: DraftOrderMarginInput,
  sizeBaseLots: bigint
): SubaccountMarginInputs => {
  const projected = cloneSubaccountInput(subaccount);
  const marketInput = getOrCreateMarketInput(projected, draftOrder.symbol);
  ensureOrderListCanBeMutated(marketInput, "project a draft market fill");
  const signedBaseLots =
    normalizeDraftSide(draftOrder.side) === "bid"
      ? sizeBaseLots
      : -sizeBaseLots;

  marketInput.position = marketInput.position
    ? {
        ...marketInput.position,
        basePositionLots: (
          toBigInt(marketInput.position.basePositionLots) + signedBaseLots
        ).toString(),
      }
    : {
        basePositionLots: signedBaseLots.toString(),
        virtualQuotePositionLots: "0",
        entryPriceTicks: toBigInt(draftOrder.priceTicks).toString(),
        unsettledFundingQuoteLots: "0",
        accumulatedFundingQuoteLots: "0",
      };
  // A position change can change the executable cap for reduce-only orders.
  // Rebuild their aggregate from the complete order rows.
  marketInput.limitOrderMargin = undefined;

  return projected;
};

/**
 * Initial margin to display: leverage-adjusted when computed, base otherwise.
 *
 * `orderLeverageAdjustedInitialMarginQuoteLots` is only present when
 * `MarginCalculationOptions.orderLeverageLimitsBySymbol` changed the result,
 * so this returns the protocol `initialMarginQuoteLots` in every other case.
 */
const getDisplayInitialMarginQuoteLots = (
  margin: SubaccountMarginResult["margin"]
): string =>
  margin.orderLeverageAdjustedInitialMarginQuoteLots ??
  margin.initialMarginQuoteLots;

const displayInitialMargin = (result: SubaccountMarginResult): bigint =>
  toBigInt(getDisplayInitialMarginQuoteLots(result.margin));

const positiveDelta = (postValue: bigint, preValue: bigint): bigint => {
  const delta = postValue - preValue;
  return delta > 0n ? delta : 0n;
};

const buildInitialMarginDeltaResult = (
  preOrderMargin: SubaccountMarginResult,
  postOrderMargin: SubaccountMarginResult
): DraftOrderMarginRequirementResult => {
  const protocolMargin = positiveDelta(
    toBigInt(postOrderMargin.margin.initialMarginQuoteLots),
    toBigInt(preOrderMargin.margin.initialMarginQuoteLots)
  );
  const adjustedMargin = positiveDelta(
    displayInitialMargin(postOrderMargin),
    displayInitialMargin(preOrderMargin)
  );
  return buildDraftOrderMarginRequirementResult(
    protocolMargin,
    adjustedMargin !== protocolMargin ? adjustedMargin : undefined
  );
};

const findPositionBaseLots = (
  subaccount: SubaccountMarginInputs,
  symbol: string
): bigint | undefined => {
  const position = subaccount.markets.find(
    (marketInput) => marketInput.symbol === symbol
  )?.position;
  return position ? toBigInt(position.basePositionLots) : undefined;
};

const executableReduceOnlyMarketSizeBaseLots = (
  subaccount: SubaccountMarginInputs,
  draftOrder: Pick<DraftOrderMarginInput, "reduceOnly" | "side" | "symbol">,
  requestedSizeBaseLots: bigint
): bigint => {
  if (!draftOrder.reduceOnly) {
    return requestedSizeBaseLots;
  }

  const basePositionLots = findPositionBaseLots(subaccount, draftOrder.symbol);
  if (basePositionLots === undefined || basePositionLots === 0n) {
    return 0n;
  }

  const side = normalizeDraftSide(draftOrder.side);
  const reducesPosition =
    (side === "bid" && basePositionLots < 0n) ||
    (side === "ask" && basePositionLots > 0n);
  if (!reducesPosition) {
    return 0n;
  }

  const reducibleSizeBaseLots = absBigInt(basePositionLots);
  return requestedSizeBaseLots < reducibleSizeBaseLots
    ? requestedSizeBaseLots
    : reducibleSizeBaseLots;
};

const buildDraftOrderMarginRequirementResult = (
  protocolMarginRequirementQuoteLots: bigint | string,
  orderLeverageAdjustedMarginRequirementQuoteLots?: bigint | string
): DraftOrderMarginRequirementResult => {
  const protocolMargin = protocolMarginRequirementQuoteLots.toString();
  const adjustedMargin =
    orderLeverageAdjustedMarginRequirementQuoteLots?.toString();
  return adjustedMargin === undefined || adjustedMargin === protocolMargin
    ? {
        marginRequirementQuoteLots: protocolMargin,
        protocolMarginRequirementQuoteLots: protocolMargin,
      }
    : {
        marginRequirementQuoteLots: adjustedMargin,
        protocolMarginRequirementQuoteLots: protocolMargin,
        orderLeverageAdjustedMarginRequirementQuoteLots: adjustedMargin,
      };
};

/**
 * Computes the incremental initial margin a draft order would require, from
 * already-built margin inputs.
 *
 * `marginRequirementQuoteLots` already incorporates the order-leverage
 * adjustment whenever `options.orderLeverageLimitsBySymbol` changes the
 * result (see `DraftOrderMarginRequirementResult`); the unadjusted value is
 * always available as `protocolMarginRequirementQuoteLots`.
 */
export const computeDraftOrderMarginRequirementFromInputs = (params: {
  subaccount: SubaccountMarginInputs;
  markets: MarginMarketsInput;
  draftOrder: DraftOrderMarginInput;
  options?: MarginCalculationOptions;
}): DraftOrderMarginRequirementResult => {
  requirePositiveDraftPriceTicks(params.draftOrder.priceTicks);
  const markets = normalizeMarginMarketsInput(params.markets);
  const sizeBaseLots = toBigInt(params.draftOrder.sizeBaseLots);
  if (sizeBaseLots <= 0n) {
    return buildDraftOrderMarginRequirementResult(0n);
  }

  // Keep this path separate from simulateMarginFromInputs: the simulator models
  // instruction-like state changes, while draft order sizing needs the display
  // initial-margin delta under optional order-leverage limits.
  const preOrderMargin = computeSubaccountMarginFromInputs(
    params.subaccount,
    markets,
    params.options
  );

  if (params.draftOrder.orderType === "limit") {
    const postOrderMargin = computeSubaccountMarginFromInputs(
      buildSubaccountInputsWithDraftLimitOrder(
        params.subaccount,
        params.draftOrder,
        sizeBaseLots
      ),
      markets,
      params.options
    );
    return buildInitialMarginDeltaResult(preOrderMargin, postOrderMargin);
  }

  if (params.draftOrder.orderType === "market") {
    const executableSizeBaseLots = executableReduceOnlyMarketSizeBaseLots(
      params.subaccount,
      params.draftOrder,
      sizeBaseLots
    );
    if (executableSizeBaseLots <= 0n) {
      return buildDraftOrderMarginRequirementResult(0n);
    }

    const postOrderMargin = computeSubaccountMarginFromInputs(
      buildSubaccountInputsWithDraftMarketFill(
        params.subaccount,
        params.draftOrder,
        executableSizeBaseLots
      ),
      markets,
      params.options
    );
    return buildInitialMarginDeltaResult(preOrderMargin, postOrderMargin);
  }

  throw new Error('Invalid draft order type. Expected "market" or "limit".');
};

/**
 * Snapshot-shaped variant of `computeDraftOrderMarginRequirementFromInputs`.
 */
export const computeDraftOrderMarginRequirementFromSnapshot = (params: {
  subaccount: MarginSnapshotSubaccount;
  markets: MarginMarketsInput;
  draftOrder: DraftOrderMarginInput;
  options?: MarginCalculationOptions;
}): DraftOrderMarginRequirementResult =>
  computeDraftOrderMarginRequirementFromInputs({
    subaccount: buildSubaccountMarginInputsFromSnapshot(params.subaccount),
    markets: params.markets,
    draftOrder: params.draftOrder,
    options: params.options,
  });

/**
 * Finds the largest draft order size (in base lots, up to `maxSizeBaseLots`)
 * whose incremental margin requirement fits within
 * `availableMarginQuoteLots`, from already-built margin inputs.
 *
 * Reduce-only market drafts are first capped to the closable position size.
 * Negative available margin is treated as zero for those drafts so a close
 * with no incremental margin requirement remains available.
 */
export const computeMaxDraftOrderSizeForAvailableMarginFromInputs = (params: {
  subaccount: SubaccountMarginInputs;
  markets: MarginMarketsInput;
  draftOrder: Omit<DraftOrderMarginInput, "sizeBaseLots">;
  maxSizeBaseLots: string | number | bigint;
  availableMarginQuoteLots: string | number | bigint;
  options?: MarginCalculationOptions;
}): bigint => {
  requirePositiveDraftPriceTicks(params.draftOrder.priceTicks);
  const requestedMaxSizeBaseLots = toBigInt(params.maxSizeBaseLots);
  const maxSizeBaseLots =
    params.draftOrder.orderType === "market"
      ? executableReduceOnlyMarketSizeBaseLots(
          params.subaccount,
          params.draftOrder,
          requestedMaxSizeBaseLots
        )
      : requestedMaxSizeBaseLots;
  if (maxSizeBaseLots <= 0n) {
    return 0n;
  }

  const rawAvailableMarginQuoteLots = toBigInt(params.availableMarginQuoteLots);
  const availableMarginQuoteLots =
    rawAvailableMarginQuoteLots < 0n &&
    params.draftOrder.orderType === "market" &&
    params.draftOrder.reduceOnly
      ? 0n
      : rawAvailableMarginQuoteLots;
  if (availableMarginQuoteLots < 0n) {
    return 0n;
  }

  const markets = normalizeMarginMarketsInput(params.markets);
  const marginForSize = (sizeBaseLots: bigint): bigint =>
    toBigInt(
      computeDraftOrderMarginRequirementFromInputs({
        subaccount: params.subaccount,
        markets,
        draftOrder: {
          ...params.draftOrder,
          sizeBaseLots,
        },
        options: params.options,
      }).marginRequirementQuoteLots
    );

  if (marginForSize(maxSizeBaseLots) <= availableMarginQuoteLots) {
    return maxSizeBaseLots;
  }

  // Initial-margin requirements are monotonic in draft base size under the
  // current margin formulas; the positive-delta clamp preserves that for
  // reducing and flip-through-zero market drafts.
  let low = 0n;
  let high = maxSizeBaseLots;
  let best = 0n;

  while (low <= high) {
    const mid = (low + high) / 2n;
    if (marginForSize(mid) <= availableMarginQuoteLots) {
      best = mid;
      low = mid + 1n;
    } else {
      high = mid - 1n;
    }
  }

  return best;
};

/**
 * Snapshot-shaped variant of
 * `computeMaxDraftOrderSizeForAvailableMarginFromInputs`.
 */
export const computeMaxDraftOrderSizeForAvailableMarginFromSnapshot = (params: {
  subaccount: MarginSnapshotSubaccount;
  markets: MarginMarketsInput;
  draftOrder: Omit<DraftOrderMarginInput, "sizeBaseLots">;
  maxSizeBaseLots: string | number | bigint;
  availableMarginQuoteLots: string | number | bigint;
  options?: MarginCalculationOptions;
}): bigint =>
  computeMaxDraftOrderSizeForAvailableMarginFromInputs({
    subaccount: buildSubaccountMarginInputsFromSnapshot(params.subaccount),
    markets: params.markets,
    draftOrder: params.draftOrder,
    maxSizeBaseLots: params.maxSizeBaseLots,
    availableMarginQuoteLots: params.availableMarginQuoteLots,
    options: params.options,
  });
