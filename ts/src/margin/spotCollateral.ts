/**
 * Spot collateral valuation.
 *
 * Spot collateral is collateral held in an asset other than the quote token.
 * Native SOL is the only spot asset today. Its margin value is the balance
 * marked at the bound perp market's **index** price (not its mark price),
 * haircut by a balance-dependent discount curve:
 *
 * ```text
 * nativeUnitsPerBaseLot = 10^(decimals - baseLotDecimals)
 * priceQuoteLotsPerBaseLot = indexPriceTicks * tickSize
 * notional   = floor(balance * priceQuoteLotsPerBaseLot / nativeUnitsPerBaseLot)
 * retainedBps(b) = (10000 - minMarginDiscountBps)
 *                - floor((maxMarginDiscountBps - minMarginDiscountBps)
 *                        * min(b, maxGlobalBalance) / maxGlobalBalance)
 * discounted = floor(notional * retainedBps / 10000)
 * ```
 *
 * Both floors are load bearing, and `retainedBps` floors the interpolation
 * term, which rounds at most 1 bps in the trader's favour. The discount
 * denominator is the exchange-wide cap while the numerator is the trader's own
 * balance, so another trader's deposit never changes your haircut.
 *
 * Spot collateral adds to effective collateral and portfolio value. It never
 * adds a margin requirement, and it is excluded from effective collateral for
 * *quote* withdrawals because it cannot back one.
 */

/**
 * Index of native SOL's entry in the trader position map (`0xFFFF0000`).
 *
 * Native SOL predates the dedicated spot-position index range and lives in the
 * trader header extension, so it has its own reserved key.
 */
export const NATIVE_SOL_ASSET_INDEX = 0xffff0000;

import type { SpotCollateralParams } from "./types";
import { applyBps } from "./math";

const BPS_DENOMINATOR = 10_000n;

/** Why a spot collateral balance could not be valued. */
export type SpotCollateralValuationFailure =
  /** No index price was supplied, or it was zero. Rise cannot derive the index
   * price itself — it is a median over the market's oracles — so it must be
   * provided, and must never silently fall back to the mark price. */
  | "missingIndexPrice"
  /** The asset has fewer decimals than the perp market's base lot, so native
   * units cannot be converted into base lots. */
  | "negativeDecimalsDifference"
  /** A zero global cap, or a minimum discount above the maximum. */
  | "invalidDiscountCurve"
  /** The perp market that prices this asset is not in the market set. */
  | "unknownPerpMarket";

export interface SpotCollateralPrice {
  /** Index price in quote lots per base lot. */
  priceQuoteLotsPerBaseLot: bigint;
  /** Native units in one base lot. */
  nativeUnitsPerBaseLot: bigint;
}

type SpotCollateralPriceParams = Pick<SpotCollateralParams, "decimals">;
type SpotCollateralDiscountParams = Pick<
  SpotCollateralParams,
  "maxGlobalBalance" | "minMarginDiscountBps" | "maxMarginDiscountBps"
>;

export class SpotCollateralValuationError extends Error {
  constructor(readonly failure: SpotCollateralValuationFailure) {
    super(`spot collateral valuation failed: ${failure}`);
    this.name = "SpotCollateralValuationError";
  }
}

/**
 * Resolve the index price of one base lot and the native units it contains.
 */
export const spotCollateralPrice = (
  params: SpotCollateralPriceParams,
  indexPriceTicks: bigint | undefined,
  tickSize: bigint,
  baseLotDecimals: number
): SpotCollateralPrice => {
  if (indexPriceTicks === undefined || indexPriceTicks === 0n) {
    throw new SpotCollateralValuationError("missingIndexPrice");
  }

  const difference = params.decimals - baseLotDecimals;
  if (difference < 0) {
    throw new SpotCollateralValuationError("negativeDecimalsDifference");
  }

  return {
    priceQuoteLotsPerBaseLot: indexPriceTicks * tickSize,
    nativeUnitsPerBaseLot: 10n ** BigInt(difference),
  };
};

/**
 * Undiscounted value of a balance, in quote lots.
 *
 * A single floor over the whole balance, so sub-base-lot dust keeps its value
 * instead of being truncated away.
 */
export const notionalSpotCollateral = (
  price: SpotCollateralPrice,
  balance: bigint
): bigint => {
  if (balance === 0n) {
    return 0n;
  }
  if (price.nativeUnitsPerBaseLot === 0n) {
    throw new SpotCollateralValuationError("negativeDecimalsDifference");
  }
  return (
    (balance * price.priceQuoteLotsPerBaseLot) / price.nativeUnitsPerBaseLot
  );
};

/**
 * Fraction of notional value retained after the margin haircut, in basis
 * points, for a trader holding `balance`.
 *
 * This is a *retained* multiplier, not a haircut: a configured
 * `minMarginDiscountBps` of 500 means 9,500 bps of value is retained at a zero
 * balance.
 */
export const marginRetainedBps = (
  params: SpotCollateralDiscountParams,
  balance: bigint
): bigint => {
  const maxBalance = params.maxGlobalBalance;
  const minDiscount = BigInt(params.minMarginDiscountBps);
  const maxDiscount = BigInt(params.maxMarginDiscountBps);

  if (
    maxBalance === 0n ||
    minDiscount > maxDiscount ||
    maxDiscount > BPS_DENOMINATOR
  ) {
    throw new SpotCollateralValuationError("invalidDiscountCurve");
  }

  const upper = BPS_DENOMINATOR - minDiscount;
  const lower = BPS_DENOMINATOR - maxDiscount;
  const target = balance < maxBalance ? balance : maxBalance;

  if (target === 0n) {
    return upper;
  }
  if (target === maxBalance) {
    return lower;
  }
  // Line through (0, upper) and (maxBalance, lower), evaluated at `target`. The
  // subtracted term floors, so the result rounds up: at most 1 bps in the
  // trader's favour.
  return upper - ((upper - lower) * target) / maxBalance;
};

/**
 * Value of a balance after the margin haircut, in quote lots. This is the term
 * spot collateral contributes to effective collateral.
 */
export const discountedSpotCollateral = (
  params: SpotCollateralParams,
  price: SpotCollateralPrice,
  balance: bigint
): bigint =>
  applyBps(
    notionalSpotCollateral(price, balance),
    marginRetainedBps(params, balance)
  );

/**
 * Maximum spot collateral (native units) withdrawable while post-withdrawal
 * effective collateral still covers the initial margin.
 *
 * Positions are fixed across the withdrawal, so the initial margin is unchanged
 * and the binding constraint reduces to
 * `effectiveCollateral >= initialMargin`. Returns zero when the account is not
 * currently healthy, matching the on-chain rule that forbids reducing
 * collateral from an unhealthy or underwater state.
 */
export const maxWithdrawableSpotCollateral = (
  nonSpotEffectiveCollateral: bigint,
  initialMargin: bigint,
  balance: bigint,
  params: SpotCollateralParams,
  price: SpotCollateralPrice
): bigint => {
  // Non-spot collateral alone covers the requirement: the whole balance is free.
  if (nonSpotEffectiveCollateral >= initialMargin) {
    return balance;
  }
  // Even the full balance cannot cover it, so the account is unhealthy and
  // nothing may be withdrawn.
  if (
    nonSpotEffectiveCollateral +
      discountedSpotCollateral(params, price, balance) <
    initialMargin
  ) {
    return 0n;
  }

  // Binary-search the smallest balance to retain such that
  // `nonSpotEc + discounted(retained) >= initialMargin`. The discounted value is
  // monotonic non-decreasing in balance under any valid curve.
  let lo = 0n;
  let hi = balance;
  while (lo < hi) {
    const mid = lo + (hi - lo) / 2n;
    if (
      nonSpotEffectiveCollateral +
        discountedSpotCollateral(params, price, mid) >=
      initialMargin
    ) {
      hi = mid;
    } else {
      lo = mid + 1n;
    }
  }
  return balance - lo;
};
