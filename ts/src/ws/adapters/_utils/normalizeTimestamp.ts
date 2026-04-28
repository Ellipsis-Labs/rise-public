import { createInvalidTimestampError } from "@/ws/errorHandling/errors";

export const normalizeTimestamp = (
  timestamp: number | string | bigint,
  unit: "s" | "ms"
): number => {
  if (unit !== "s" && unit !== "ms") {
    throw createInvalidTimestampError(unit, {
      operation: "normalize_timestamp",
      detail: "unit must be 's' or 'ms'",
    });
  }

  const maxSafe = BigInt(Number.MAX_SAFE_INTEGER);
  const minSafe = BigInt(Number.MIN_SAFE_INTEGER);
  let parsedFromIso = false;

  const parseIsoTimestamp = (value: string): bigint => {
    const parsed = Date.parse(value);
    if (Number.isNaN(parsed)) {
      throw createInvalidTimestampError(unit, {
        operation: "normalize_timestamp",
        rawMessage: value,
        detail: "timestamp string is neither integer nor ISO-8601",
      });
    }
    const parsedBigInt = BigInt(parsed);
    if (parsedBigInt > maxSafe || parsedBigInt < minSafe) {
      throw createInvalidTimestampError(unit, {
        operation: "normalize_timestamp",
        rawMessage: value,
        detail: "timestamp exceeds JS safe integer range after parsing",
      });
    }
    return parsedBigInt;
  };

  const toBigInt = () => {
    if (typeof timestamp === "bigint") return timestamp;
    if (typeof timestamp === "string") {
      if (!/^-?\d+$/.test(timestamp)) {
        parsedFromIso = true;
        return parseIsoTimestamp(timestamp);
      }
      return BigInt(timestamp);
    }
    if (!Number.isFinite(timestamp)) {
      throw createInvalidTimestampError(unit, {
        operation: "normalize_timestamp",
        rawMessage: timestamp,
        detail: "timestamp is not a finite number",
      });
    }
    if (!Number.isSafeInteger(timestamp)) {
      throw createInvalidTimestampError(unit, {
        operation: "normalize_timestamp",
        rawMessage: timestamp,
        detail: "timestamp exceeds JS safe integer range",
      });
    }
    return BigInt(timestamp);
  };

  const ts = toBigInt();
  const scaled = unit === "s" && !parsedFromIso ? ts * 1000n : ts;
  if (scaled > maxSafe || scaled < minSafe) {
    throw createInvalidTimestampError(unit, {
      operation: "normalize_timestamp",
      rawMessage: timestamp,
      detail: "timestamp exceeds JS safe integer range after scaling",
    });
  }
  return Number(scaled);
};
