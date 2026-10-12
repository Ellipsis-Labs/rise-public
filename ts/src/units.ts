import {
  baseLots,
  quoteLots,
  ticks,
  type BaseLots,
  type QuoteLots,
  type Ticks,
} from "@/primitives/_numberTypes";
import { Side } from "@/primitives/Side";

export const QUOTE_LOTS_PER_USD = 1_000_000n;
export const QUOTE_DECIMALS = 6;
export const BPS_DENOMINATOR = 10_000n;
export const FEE_MICRO_MULTIPLIER = 1_000_000;

export const pow10 = (exponent: number): bigint => 10n ** BigInt(exponent);

export const divCeil = (numerator: bigint, denominator: bigint): bigint => {
  if (denominator === 0n) {
    throw new Error("division by zero");
  }
  return (numerator + denominator - 1n) / denominator;
};

export const divRoundNearest = (
  numerator: bigint,
  denominator: bigint
): bigint => (numerator + denominator / 2n) / denominator;

export const applyBps = (value: bigint, bps: bigint): bigint =>
  (value * bps) / BPS_DENOMINATOR;

export const applyBpsCeil = (value: bigint, bps: bigint): bigint =>
  divCeil(value * bps, BPS_DENOMINATOR);

export type Rounding = "floor" | "ceil" | "nearest";

const divRound = (
  numerator: bigint,
  denominator: bigint,
  rounding: Rounding
): bigint =>
  rounding === "ceil"
    ? divCeil(numerator, denominator)
    : rounding === "nearest"
      ? divRoundNearest(numerator, denominator)
      : numerator / denominator;

const DECIMAL_PATTERN = /^(?:0|[1-9]\d*)(?:\.(\d+))?$/;
const EXPONENT_PATTERN = /^(\d+)(?:\.(\d+))?e([+-]\d+)$/;

/**
 * Parse a non-negative base-10 decimal into an exact `numerator / scale`
 * ratio. Numbers are read through their shortest round-trip representation.
 */
export const parseNonNegativeDecimal = (
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

  const exponentMatch =
    typeof value === "number" ? EXPONENT_PATTERN.exec(normalized) : null;
  if (exponentMatch) {
    const [, whole, fraction = "", exponentText] = exponentMatch;
    const exponent = Number(exponentText) - fraction.length;
    const digits = BigInt(`${whole}${fraction}`);
    return exponent >= 0
      ? { numerator: digits * pow10(exponent), scale: 1n }
      : { numerator: digits, scale: pow10(-exponent) };
  }

  if (!DECIMAL_PATTERN.test(normalized)) {
    throw new Error(
      `${fieldName} must be a non-negative base-10 decimal string, number, or bigint`
    );
  }

  const [whole, fraction = ""] = normalized.split(".");
  return {
    numerator: BigInt(`${whole}${fraction}`),
    scale: pow10(fraction.length),
  };
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

/** Convert a USD price to ticks. Rounds down by default. */
export const priceUsdToTicksWithMarketParams = (
  priceUsd: number | string | bigint,
  marketParams: OrderPacketMarketParams,
  rounding: Rounding = "floor"
): Ticks => {
  const { numerator, scale } = parseNonNegativeDecimal(priceUsd, "priceUsd");
  const tickSize = toBigIntStrict(marketParams.tickSize, "tickSize");
  if (tickSize <= 0n) {
    throw new Error("tickSize must be greater than zero");
  }

  const decimals = marketParams.baseLotsDecimals;
  const decimalScale = pow10(Math.abs(decimals));
  let scaledNumerator = numerator * QUOTE_LOTS_PER_USD;
  let denominator = scale * tickSize;

  if (decimals >= 0) {
    denominator *= decimalScale;
  } else {
    scaledNumerator *= decimalScale;
  }

  return ticks(divRound(scaledNumerator, denominator, rounding));
};

/**
 * Convert a USD order price to ticks: bids round down, asks round up.
 * Throws if the price resolves to zero ticks.
 */
export const orderPriceUsdToTicksWithMarketParams = (
  priceUsd: number | string | bigint,
  marketParams: OrderPacketMarketParams,
  side: Side
): Ticks => {
  const priceInTicks = priceUsdToTicksWithMarketParams(
    priceUsd,
    marketParams,
    side === Side.Bid ? "floor" : "ceil"
  );
  if (priceInTicks === 0n) {
    throw new Error("order price must be at least one tick");
  }
  return priceInTicks;
};

/**
 * Inverse of {@link priceUsdToTicksWithMarketParams}: convert an integer tick
 * price back to a human-readable USD price for display/preview. Returns the
 * USD value of the tick boundary itself (not the original pre-snap price).
 * Returned as a `number` for display; the exact tick value remains the source
 * of truth.
 */
export const ticksToUsdWithMarketParams = (
  priceInTicks: number | string | bigint,
  marketParams: OrderPacketMarketParams
): number => {
  const tickValue = toBigIntStrict(priceInTicks, "priceInTicks");
  const tickSize = toBigIntStrict(marketParams.tickSize, "tickSize");
  if (tickSize <= 0n) {
    throw new Error("tickSize must be greater than zero");
  }

  const decimals = marketParams.baseLotsDecimals;
  const decimalScale = pow10(Math.abs(decimals));

  if (decimals >= 0) {
    return (
      Number(tickValue * tickSize * decimalScale) / Number(QUOTE_LOTS_PER_USD)
    );
  }
  return (
    Number(tickValue * tickSize) /
    (Number(decimalScale) * Number(QUOTE_LOTS_PER_USD))
  );
};

/** Convert human base units to base lots, rounding down. */
export const baseUnitsToBaseLotsWithMarketParams = (
  baseUnits: number | string | bigint,
  marketParams: Pick<OrderPacketMarketParams, "baseLotsDecimals">
): BaseLots => {
  const { numerator, scale } = parseNonNegativeDecimal(baseUnits, "baseUnits");
  const decimals = marketParams.baseLotsDecimals;
  const decimalScale = pow10(Math.abs(decimals));

  const lots =
    decimals >= 0
      ? (numerator * decimalScale) / scale
      : numerator / (scale * decimalScale);

  return baseLots(lots);
};

export const baseLotsToBaseUnits = (
  lots: number | bigint,
  baseLotsDecimals: number
): number => Number(lots) / 10 ** baseLotsDecimals;

/** Convert a USD amount to quote lots, rounding down. */
export const usdToQuoteLots = (value: number | string | bigint): QuoteLots => {
  const { numerator, scale } = parseNonNegativeDecimal(value, "value");
  return quoteLots((numerator * QUOTE_LOTS_PER_USD) / scale);
};

export const quoteLotsToUsd = (lots: number | bigint): number =>
  Number(lots) / Number(QUOTE_LOTS_PER_USD);

export interface TickSizeDisplayParams {
  baseLotsDecimals: number;
  tickSizeInQuoteLotsPerBaseLot: number;
  quoteDecimals?: number;
}

/** Number of decimal places needed to display a price on the tick grid. */
export const priceDecimalsFromTickSize = (
  params: TickSizeDisplayParams
): number => {
  const quoteDecimals = params.quoteDecimals ?? QUOTE_DECIMALS;
  const decimalShift = Math.max(0, quoteDecimals - params.baseLotsDecimals);
  let normalizedTickSize = Math.abs(params.tickSizeInQuoteLotsPerBaseLot);
  let trailingZeroCount = 0;

  while (normalizedTickSize !== 0 && normalizedTickSize % 10 === 0) {
    normalizedTickSize /= 10;
    trailingZeroCount += 1;
  }

  const decimals = Math.max(0, Math.trunc(decimalShift - trailingZeroCount));
  return decimals === 1 ? 2 : decimals;
};

/** Tick size in USD, rounded to its display precision. */
export const displayTickSize = (params: TickSizeDisplayParams): number => {
  const quoteDecimals = params.quoteDecimals ?? QUOTE_DECIMALS;
  const tickSize =
    (10 ** params.baseLotsDecimals / 10 ** quoteDecimals) *
    params.tickSizeInQuoteLotsPerBaseLot;
  return Number(tickSize.toFixed(priceDecimalsFromTickSize(params)));
};

export const feeRateToMicro = (feeRate: number): number =>
  Math.round(feeRate * FEE_MICRO_MULTIPLIER);

export const riskFactorPercentToBps = (value: number): number => {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.round(value * 100);
};
