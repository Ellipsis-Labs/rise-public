import {
  baseLots,
  quoteLots,
  ticks,
  type BaseLots,
  type QuoteLots,
  type Ticks,
} from "@/primitives/_numberTypes";
import { type Side } from "@/primitives/Side";
import {
  OrderFlags,
  SelfTradeBehavior,
  type ImmediateOrCancelOrderPacket,
  type LimitOrderPacket,
} from "@/primitives/OrderPacket";

const MICRO_USD = 1_000_000n;

const DECIMAL_PATTERN = /^(?:0|[1-9]\d*)(?:\.(\d+))?$/;

const parseNonNegativeDecimal = (
  value: number | string | bigint,
  fieldName: string
): { numerator: bigint; scale: bigint } => {
  const normalized =
    typeof value === "bigint"
      ? value.toString()
      : typeof value === "number"
        ? Number.isFinite(value)
          ? value.toString()
          : ""
        : value.trim();
  const match = DECIMAL_PATTERN.exec(normalized);
  if (!match) {
    throw new Error(
      `${fieldName} must be a non-negative base-10 decimal string, number, or bigint`
    );
  }

  const [whole, fraction = ""] = normalized.split(".");
  const digits = `${whole}${fraction}`;
  const numerator = digits.length > 0 ? BigInt(digits) : 0n;
  const scale = 10n ** BigInt(fraction.length);
  return { numerator, scale };
};

const toBigIntStrict = (
  value: number | string | bigint,
  fieldName: string
): bigint => {
  if (typeof value === "bigint") {
    return value;
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value) || !Number.isSafeInteger(value)) {
      throw new Error(
        `${fieldName} must be a finite safe integer when passed as a number`
      );
    }
    return BigInt(value);
  }
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    throw new Error(`${fieldName} must be a non-negative integer`);
  }
  return BigInt(trimmed);
};

export interface OrderPacketMarketParams {
  tickSize: number | string | bigint;
  baseLotsDecimals: number;
}

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

export const priceUsdToTicksWithMarketParams = (
  priceUsd: number | string | bigint,
  marketParams: OrderPacketMarketParams
): Ticks => {
  const { numerator, scale } = parseNonNegativeDecimal(priceUsd, "priceUsd");
  const priceMicros = (numerator * MICRO_USD) / scale;
  const tickSize = toBigIntStrict(marketParams.tickSize, "tickSize");
  if (tickSize <= 0n) {
    throw new Error("tickSize must be greater than zero");
  }

  const decimals = marketParams.baseLotsDecimals;
  let scaledNumerator = priceMicros;
  let denominator = tickSize;
  const decimalScale = 10n ** BigInt(Math.abs(decimals));

  if (decimals >= 0) {
    denominator *= decimalScale;
  } else {
    scaledNumerator *= decimalScale;
  }

  return ticks(scaledNumerator / denominator);
};

export const baseUnitsToBaseLotsWithMarketParams = (
  baseUnits: number | string | bigint,
  marketParams: Pick<OrderPacketMarketParams, "baseLotsDecimals">
): BaseLots => {
  const { numerator, scale } = parseNonNegativeDecimal(baseUnits, "baseUnits");
  const decimals = marketParams.baseLotsDecimals;
  const decimalScale = 10n ** BigInt(Math.abs(decimals));

  const lots =
    decimals >= 0
      ? (numerator * decimalScale) / scale
      : numerator / (scale * decimalScale);

  return baseLots(lots);
};

export const buildLimitOrderPacketFromMarketParams = (
  params: BuildLimitOrderPacketFromMarketParamsInput,
  marketParams: OrderPacketMarketParams
): LimitOrderPacket => ({
  side: params.side,
  priceInTicks: priceUsdToTicksWithMarketParams(params.priceUsd, marketParams),
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
        : priceUsdToTicksWithMarketParams(params.priceLimitUsd, marketParams),
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
