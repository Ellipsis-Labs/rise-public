import {
  Direction,
  Side,
  StopLossOrderKind,
  ticks,
  type Ticks,
  type TriggerOrderParams,
} from "@/primitives";

const BPS_DENOMINATOR = 10_000n;
export const TP_SL_MAX_SLIPPAGE_BPS = 1_000;

export type TriggerOrderParamsInput = {
  triggerDirection: Direction;
  tradeSide: Side;
  orderKind?: StopLossOrderKind;
  triggerPrice: Ticks;
  executionPrice?: Ticks | null;
  slippageBps?: number | null;
};

export type TakeProfitStopLossTriggerOrdersInput = {
  primarySide: Side;
  takeProfitPrice?: Ticks | null;
  stopLossPrice?: Ticks | null;
  slippageBps?: number | null;
};

export type TakeProfitStopLossTriggerOrders = {
  greaterTriggerOrder: TriggerOrderParams | null;
  lessTriggerOrder: TriggerOrderParams | null;
};

export const executionPriceFromSlippageBps = (
  triggerPrice: Ticks,
  tradeSide: Side,
  slippageBps: number
): Ticks => {
  if (!Number.isInteger(slippageBps) || slippageBps < 0) {
    throw new Error("slippageBps must be a non-negative integer");
  }

  const bps = BigInt(slippageBps);

  if (tradeSide === Side.Ask) {
    if (slippageBps >= 10_000) {
      throw new Error("sell-side slippageBps must be less than 10000");
    }

    let executionPrice =
      (triggerPrice * (BPS_DENOMINATOR - bps)) / BPS_DENOMINATOR;
    if (slippageBps > 0 && executionPrice === triggerPrice) {
      executionPrice -= 1n;
    }
    if (executionPrice <= 0n) {
      throw new Error(
        "sell-side slippageBps resolves executionPrice to 0 ticks"
      );
    }

    return ticks(executionPrice);
  }

  let executionPrice =
    (triggerPrice * (BPS_DENOMINATOR + bps) + BPS_DENOMINATOR - 1n) /
    BPS_DENOMINATOR;
  if (slippageBps > 0 && executionPrice === triggerPrice) {
    executionPrice += 1n;
  }

  return ticks(executionPrice);
};

export const buildTriggerOrderParams = (
  params: TriggerOrderParamsInput
): TriggerOrderParams => ({
  triggerDirection: params.triggerDirection,
  tradeSide: params.tradeSide,
  orderKind: params.orderKind ?? StopLossOrderKind.IOC,
  triggerPrice: params.triggerPrice,
  executionPrice: resolveExecutionPrice(params),
});

export const normalizeTriggerOrderParams = (
  params: TriggerOrderParamsInput | null | undefined
): TriggerOrderParams | null => {
  if (params == null) {
    return null;
  }
  return buildTriggerOrderParams(params);
};

export const buildTakeProfitStopLossTriggerOrders = (
  params: TakeProfitStopLossTriggerOrdersInput
): TakeProfitStopLossTriggerOrders => {
  const tradeSide = params.primarySide === Side.Bid ? Side.Ask : Side.Bid;
  const slippageBps = params.slippageBps ?? TP_SL_MAX_SLIPPAGE_BPS;
  const takeProfit =
    params.takeProfitPrice == null
      ? null
      : buildTriggerOrderParams({
          triggerDirection:
            params.primarySide === Side.Bid
              ? Direction.GreaterThan
              : Direction.LessThan,
          tradeSide,
          orderKind: StopLossOrderKind.Limit,
          triggerPrice: params.takeProfitPrice,
          executionPrice: params.takeProfitPrice,
        });
  const stopLoss =
    params.stopLossPrice == null
      ? null
      : buildTriggerOrderParams({
          triggerDirection:
            params.primarySide === Side.Bid
              ? Direction.LessThan
              : Direction.GreaterThan,
          tradeSide,
          orderKind: StopLossOrderKind.IOC,
          triggerPrice: params.stopLossPrice,
          slippageBps,
        });

  return params.primarySide === Side.Bid
    ? { greaterTriggerOrder: takeProfit, lessTriggerOrder: stopLoss }
    : { greaterTriggerOrder: stopLoss, lessTriggerOrder: takeProfit };
};

const resolveExecutionPrice = (params: TriggerOrderParamsInput): Ticks => {
  if (params.executionPrice != null && params.slippageBps != null) {
    throw new Error("Provide executionPrice or slippageBps, not both");
  }

  if (params.executionPrice != null) {
    return params.executionPrice;
  }

  if (params.slippageBps != null) {
    return executionPriceFromSlippageBps(
      params.triggerPrice,
      params.tradeSide,
      params.slippageBps
    );
  }

  if ((params.orderKind ?? StopLossOrderKind.IOC) === StopLossOrderKind.Limit) {
    return params.triggerPrice;
  }

  return executionPriceFromSlippageBps(
    params.triggerPrice,
    params.tradeSide,
    TP_SL_MAX_SLIPPAGE_BPS
  );
};
