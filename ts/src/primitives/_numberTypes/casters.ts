import type {
  BaseLots,
  BaseLotsPerTick,
  QuoteLots,
  SignedBaseLots,
  SignedQuoteLots,
  Ticks,
} from "./types";

// Constants for validation
const U64_MIN = 0n;
const U64_MAX = 18446744073709551615n; // 2^64 - 1
const I64_MIN = -9223372036854775808n; // -2^63
const I64_MAX = 9223372036854775807n; // 2^63 - 1

/**
 * Converts input to bigint, accepting number, bigint, or string.
 */
function toBigInt(value: number | bigint | string, typeName: string): bigint {
  try {
    if (typeof value === "bigint") {
      return value;
    }
    if (typeof value === "number") {
      if (!Number.isFinite(value)) {
        throw new Error(`${typeName}: value must be finite`);
      }
      if (!Number.isSafeInteger(value)) {
        throw new Error(
          `${typeName}: number value ${value} is not a safe integer, use bigint or string instead`
        );
      }
      return BigInt(value);
    }
    if (typeof value === "string") {
      return BigInt(value);
    }
    throw new Error(`${typeName}: invalid input type ${typeof value}`);
  } catch (error) {
    if (error instanceof Error) {
      throw error;
    }
    throw new Error(`${typeName}: failed to convert value to bigint`, {
      cause: error,
    });
  }
}

/**
 * Validates that a bigint is within unsigned 64-bit range [0, 2^64-1].
 */
function validateU64(value: bigint, typeName: string): void {
  if (value < U64_MIN || value > U64_MAX) {
    throw new Error(
      `${typeName}: value ${value} is out of u64 range [${U64_MIN}, ${U64_MAX}]`
    );
  }
}

/**
 * Validates that a bigint is within signed 64-bit range [-2^63, 2^63-1].
 */
function validateI64(value: bigint, typeName: string): void {
  if (value < I64_MIN || value > I64_MAX) {
    throw new Error(
      `${typeName}: value ${value} is out of i64 range [${I64_MIN}, ${I64_MAX}]`
    );
  }
}

// Unsigned 64-bit types

export const ticks = (value: number | bigint | string): Ticks => {
  const v = toBigInt(value, "Ticks");
  validateU64(v, "Ticks");
  return v as Ticks;
};

export const baseLots = (value: number | bigint | string): BaseLots => {
  const v = toBigInt(value, "BaseLots");
  validateU64(v, "BaseLots");
  return v as BaseLots;
};

export const baseLotsPerTick = (
  value: number | bigint | string
): BaseLotsPerTick => {
  const v = toBigInt(value, "BaseLotsPerTick");
  validateU64(v, "BaseLotsPerTick");
  return v as BaseLotsPerTick;
};

export const quoteLots = (value: number | bigint | string): QuoteLots => {
  const v = toBigInt(value, "QuoteLots");
  validateU64(v, "QuoteLots");
  return v as QuoteLots;
};

// Signed 64-bit types

export const signedBaseLots = (
  value: number | bigint | string
): SignedBaseLots => {
  const v = toBigInt(value, "SignedBaseLots");
  validateI64(v, "SignedBaseLots");
  return v as SignedBaseLots;
};

export const signedQuoteLots = (
  value: number | bigint | string
): SignedQuoteLots => {
  const v = toBigInt(value, "SignedQuoteLots");
  validateI64(v, "SignedQuoteLots");
  return v as SignedQuoteLots;
};
