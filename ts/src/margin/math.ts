const BPS_DENOMINATOR = 10_000n;

export const toBigInt = (value: string | number | bigint): bigint => {
  if (typeof value === "bigint") {
    return value;
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error("value is not finite");
    }
    if (!Number.isSafeInteger(value)) {
      throw new Error("value is not a safe integer");
    }
    return BigInt(value);
  }
  return BigInt(value);
};

export const absBigInt = (value: bigint): bigint =>
  value < 0n ? -value : value;

export const maxBigInt = (a: bigint, b: bigint): bigint => (a > b ? a : b);

export const minBigInt = (a: bigint, b: bigint): bigint => (a < b ? a : b);

export const divCeil = (numerator: bigint, denominator: bigint): bigint => {
  if (denominator === 0n) {
    throw new Error("division by zero");
  }
  if (numerator === 0n) {
    return 0n;
  }
  return (numerator + denominator - 1n) / denominator;
};

export const applyBps = (value: bigint, bps: bigint): bigint =>
  (value * bps) / BPS_DENOMINATOR;

export const applyBpsCeil = (value: bigint, bps: bigint): bigint =>
  divCeil(value * bps, BPS_DENOMINATOR);

export interface LeverageTier {
  upperBoundSize: bigint;
  maxLeverage: bigint;
  limitOrderRiskFactorBps: bigint;
}

const interpolateU64 = (
  x1: bigint,
  y1: bigint,
  x2: bigint,
  y2: bigint,
  x: bigint
): bigint => {
  if (x1 === x2 || y1 === y2) {
    return y1;
  }

  const xRange = Number(x2) - Number(x1);
  if (xRange <= 0) {
    return y1;
  }

  const yRange = Number(y2) - Number(y1);
  const xOffset = Number(x) - Number(x1);
  const percentOfXRange = Math.min(Math.max(xOffset / xRange, 0), 1);
  const interpolatedValue = Number(y1) + percentOfXRange * yRange;
  return BigInt(Math.trunc(interpolatedValue));
};

export const getLeverageConstant = (
  tiers: LeverageTier[],
  positionSize: bigint
): bigint => {
  for (let i = 0; i < tiers.length; i += 1) {
    const tier = tiers[i];
    if (positionSize <= tier.upperBoundSize) {
      if (i === 0) {
        return tier.maxLeverage;
      }
      const prev = tiers[i - 1];
      return interpolateU64(
        prev.upperBoundSize,
        prev.maxLeverage,
        tier.upperBoundSize,
        tier.maxLeverage,
        positionSize
      );
    }
  }
  return 1n;
};

export const getLimitOrderRiskFactor = (
  tiers: LeverageTier[],
  positionSize: bigint
): bigint => {
  for (let i = 0; i < tiers.length; i += 1) {
    const tier = tiers[i];
    if (positionSize <= tier.upperBoundSize) {
      if (i === 0) {
        return tier.limitOrderRiskFactorBps;
      }
      const prev = tiers[i - 1];
      return interpolateU64(
        prev.upperBoundSize,
        prev.limitOrderRiskFactorBps,
        tier.upperBoundSize,
        tier.limitOrderRiskFactorBps,
        positionSize
      );
    }
  }
  return BPS_DENOMINATOR;
};
