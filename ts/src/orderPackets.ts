import { quoteLots, type QuoteLots } from "@/primitives/_numberTypes";
import { type Side } from "@/primitives/Side";
import {
  OrderFlags,
  SelfTradeBehavior,
  type ImmediateOrCancelOrderPacket,
  type LimitOrderPacket,
} from "@/primitives/OrderPacket";
import {
  baseUnitsToBaseLotsWithMarketParams,
  orderPriceUsdToTicksWithMarketParams,
  type OrderPacketMarketParams,
} from "@/units";

export interface BuildLimitOrderPacketFromMarketParamsInput {
  side: Side;
  priceUsd: number | string | bigint;
  baseUnits: number | string | bigint;
  selfTradeBehavior?: SelfTradeBehavior;
  matchLimit?: bigint | null;
  clientOrderId?: bigint;
  lastValidSlot?: bigint | null;
  orderFlags?: OrderFlags;
  cancelExisting?: boolean;
}

export interface BuildMarketOrderPacketFromMarketParamsInput {
  side: Side;
  baseUnits: number | string | bigint;
  priceLimitUsd?: number | string | bigint | null;
  numQuoteLots?: QuoteLots | null;
  minBaseUnitsToFill?: number | string | bigint;
  minQuoteLotsToFill?: QuoteLots | null;
  selfTradeBehavior?: SelfTradeBehavior;
  matchLimit?: bigint | null;
  clientOrderId?: bigint;
  lastValidSlot?: bigint | null;
  orderFlags?: OrderFlags;
  cancelExisting?: boolean;
}

export interface PhoenixOrderPacketBuilders {
  buildLimitOrderPacket: (
    params: { symbol: string } & BuildLimitOrderPacketFromMarketParamsInput
  ) => Promise<LimitOrderPacket>;
  buildMarketOrderPacket: (
    params: { symbol: string } & BuildMarketOrderPacketFromMarketParamsInput
  ) => Promise<ImmediateOrCancelOrderPacket>;
}

export const buildLimitOrderPacketFromMarketParams = (
  params: BuildLimitOrderPacketFromMarketParamsInput,
  marketParams: OrderPacketMarketParams
): LimitOrderPacket => ({
  side: params.side,
  priceInTicks: orderPriceUsdToTicksWithMarketParams(
    params.priceUsd,
    marketParams,
    params.side
  ),
  numBaseLots: baseUnitsToBaseLotsWithMarketParams(
    params.baseUnits,
    marketParams
  ),
  selfTradeBehavior:
    params.selfTradeBehavior ?? SelfTradeBehavior.CancelProvide,
  matchLimit: params.matchLimit ?? null,
  clientOrderId: params.clientOrderId ?? 0n,
  lastValidSlot: params.lastValidSlot ?? null,
  orderFlags: params.orderFlags ?? OrderFlags.None,
  cancelExisting: params.cancelExisting ?? false,
});

export const buildMarketOrderPacketFromMarketParams = (
  params: BuildMarketOrderPacketFromMarketParamsInput,
  marketParams: OrderPacketMarketParams
): ImmediateOrCancelOrderPacket => {
  const numBaseLots = baseUnitsToBaseLotsWithMarketParams(
    params.baseUnits,
    marketParams
  );
  const minBaseLotsToFill =
    params.minBaseUnitsToFill === undefined ||
    params.minBaseUnitsToFill === null
      ? numBaseLots
      : baseUnitsToBaseLotsWithMarketParams(
          params.minBaseUnitsToFill,
          marketParams
        );

  return {
    side: params.side,
    priceInTicks:
      params.priceLimitUsd === undefined || params.priceLimitUsd === null
        ? null
        : orderPriceUsdToTicksWithMarketParams(
            params.priceLimitUsd,
            marketParams,
            params.side
          ),
    numBaseLots,
    numQuoteLots: params.numQuoteLots ?? null,
    minBaseLotsToFill,
    minQuoteLotsToFill: params.minQuoteLotsToFill ?? quoteLots(1n),
    selfTradeBehavior: params.selfTradeBehavior ?? SelfTradeBehavior.Abort,
    matchLimit: params.matchLimit ?? null,
    clientOrderId: params.clientOrderId ?? 0n,
    lastValidSlot: params.lastValidSlot ?? null,
    orderFlags: params.orderFlags ?? OrderFlags.None,
    cancelExisting: params.cancelExisting ?? false,
  };
};
