import type { TraderStateMarketLimitOrderRow } from "@/api/traders/traderState";
import {
  priceUsdToTicksWithMarketParams,
  ticksToUsdWithMarketParams,
  type OrderPacketMarketParams,
} from "@/orderPackets";
import type { CancelId } from "@/primitives/CancelId";
import { ticks, u64 } from "@/primitives/_numberTypes";
import { Side } from "@/primitives/Side";
import {
  CondensedOrderFlags,
  type MultipleOrderPacket,
  type MultipleOrderPacketV2,
} from "@/primitives/OrderPacket";

/**
 * Bounds for the number of sub-orders in a scale (multi-limit) order. The
 * on-chain matching engine allows at most 64 resting limit orders per trader
 * per market per side (`MAX_LIMIT_ORDERS`); a scale batch is one-sided, so the
 * order count is capped there.
 */
export const MIN_SCALE_ORDERS = 2;
export const MAX_SCALE_ORDERS = 64;

/** Distribution bias slider range (maps to the `-100% … +100%` UI). */
export const MIN_SCALE_BIAS = -1;
export const MAX_SCALE_BIAS = 1;

/** Per-order floor in base lots; an order may not rest below this. */
export const DEFAULT_MIN_BASE_LOTS_PER_ORDER = 1;

/**
 * Conservative default for how many sub-orders to pack into a single
 * `place_multi_limit_order` transaction. A full 64-order side does not fit one
 * legacy transaction (~39 max) and only ~54 fit a v0 transaction with an
 * Address Lookup Table, before compute-unit limits. 30 leaves headroom for a
 * blockhash, signatures, a compute-budget instruction and a client order id.
 * Callers using v0 + ALT can raise this via {@link chunkScaleLevelsForTx}.
 */
export const DEFAULT_MAX_ORDERS_PER_TX = 30;

/**
 * Same, for `place_multi_limit_order_v2`. A V2 leg is 25 wire bytes against
 * the legacy no-expiry leg's 17, so 30 no longer fits a legacy transaction;
 * 24 keeps the headroom the V1 default has.
 */
export const DEFAULT_MAX_ORDERS_PER_TX_V2 = 24;

export interface ScaleOrderInput {
  /** Lower (min) price bound of the ladder, in USD. */
  lowerPrice: number;
  /** Upper (max) price bound of the ladder, in USD. */
  upperPrice: number;
  /** Requested number of sub-orders to spread across the range. */
  orderCount: number;
  /** Total order size in base lots (integer). */
  totalBaseLots: number;
  /**
   * Price distribution bias in `[-1, 1]`. Negative clusters price levels toward
   * the lower bound, positive toward the upper bound, `0` spaces them evenly.
   */
  priceBias: number;
  /**
   * Amount (base size) distribution bias in `[-1, 1]`. Negative assigns more
   * base size to lower prices, positive to higher prices, `0` splits size
   * uniformly. Weighting is by base size, not notional.
   */
  sizeBias: number;
  baseLotsDecimals: number;
  /** Market tick size, as accepted by {@link OrderPacketMarketParams}. */
  tickSize: number | string | bigint;
  /** Per-order floor in base lots; defaults to {@link DEFAULT_MIN_BASE_LOTS_PER_ORDER}. */
  minBaseLotsPerOrder?: number;
}

export interface ScaleOrderLevel {
  /** 0-based index of the sub-order, ascending by price. */
  index: number;
  /** Nearest-tick-snapped price in USD (for display/preview). */
  priceUsd: number;
  /** Price in ticks (for the on-chain order packet). */
  priceInTicks: bigint;
  /** Allocated size in base lots. */
  sizeBaseLots: number;
  /** Allocated size in base units (for display/preview). */
  sizeUnits: number;
}

export type ScaleOrderWarningCode =
  | "ORDER_COUNT_EXCEEDS_MAX"
  | "INSUFFICIENT_SIZE_FOR_COUNT"
  | "LEVELS_MERGED_DUPLICATE_TICK"
  | "INVALID_PRICE_RANGE"
  | "NON_POSITIVE_TOTAL_SIZE";

export interface ScaleOrderWarning {
  code: ScaleOrderWarningCode;
  message: string;
  /** Indices (in the final level list) affected, when applicable. */
  levelIndices?: number[];
}

export interface ScaleOrderPreview {
  /** Final, merged levels — every level rests at a distinct tick with size ≥ min. */
  levels: ScaleOrderLevel[];
  /** Raw order count the caller requested. */
  requestedOrderCount: number;
  /** Number of levels actually produced (after clamp / merge / drop). */
  effectiveOrderCount: number;
  /** `min(64, floor(totalBaseLots / minBaseLotsPerOrder))`. */
  maxOrderCountForSize: number;
  /** How many distinct ticks the price range can support. */
  distinctTickCount: number;
  /** Sum of `sizeBaseLots` across levels; equals `totalBaseLots` when valid. */
  totalBaseLotsAllocated: number;
  /** Whether the configuration is submittable. */
  isValid: boolean;
  warnings: ScaleOrderWarning[];
}

export interface ScaleLevelsToPacketOptions {
  clientOrderId?: bigint | null;
  slide?: boolean;
  lastValidSlot?: bigint | null;
}

export interface ScaleLevelsToPacketV2Options extends ScaleLevelsToPacketOptions {
  reduceOnly?: boolean;
  /** 1-255 = caller-assigned ladder id; 0 (default) = not part of a scale-order set. */
  scaleSetId?: number;
}

export const clampScaleBias = (bias: number): number => {
  if (!Number.isFinite(bias)) {
    return 0;
  }
  return Math.max(MIN_SCALE_BIAS, Math.min(MAX_SCALE_BIAS, bias));
};

export const clampScaleOrderCount = (count: number): number => {
  if (!Number.isFinite(count)) {
    return MIN_SCALE_ORDERS;
  }
  return Math.max(
    MIN_SCALE_ORDERS,
    Math.min(MAX_SCALE_ORDERS, Math.floor(count))
  );
};

const marketParamsOf = (input: ScaleOrderInput): OrderPacketMarketParams => ({
  tickSize: input.tickSize,
  baseLotsDecimals: input.baseLotsDecimals,
});

/** Format a positive float as a plain decimal string (never exponential). */
const toUsdString = (price: number): string =>
  Number.isFinite(price) && price > 0 ? price.toFixed(12) : "0";

/** Snap a raw USD price to the nearest tick, returning both the tick and its USD value. */
const snapToNearestTick = (
  rawPrice: number,
  marketParams: OrderPacketMarketParams
): { priceInTicks: bigint; priceUsd: number } => {
  const floorTicks = priceUsdToTicksWithMarketParams(
    toUsdString(rawPrice),
    marketParams
  );
  const ceilTicks = floorTicks + 1n;
  const floorUsd = ticksToUsdWithMarketParams(floorTicks, marketParams);
  const ceilUsd = ticksToUsdWithMarketParams(ceilTicks, marketParams);
  const nearest =
    Math.abs(rawPrice - floorUsd) <= Math.abs(ceilUsd - rawPrice)
      ? floorTicks
      : ceilTicks;
  return {
    priceInTicks: nearest,
    priceUsd: ticksToUsdWithMarketParams(nearest, marketParams),
  };
};

/**
 * Allocate `total` base lots across `weights`, flooring each share and
 * distributing the rounding remainder to the largest-weight indices so the
 * allocated total exactly equals `total`.
 */
const allocateByWeight = (weights: number[], total: number): number[] => {
  let weightSum = weights.reduce((sum, w) => sum + w, 0);
  let effectiveWeights = weights;
  if (weightSum <= 0) {
    effectiveWeights = weights.map(() => 1);
    weightSum = effectiveWeights.length;
  }

  const sizes = effectiveWeights.map((w) =>
    Math.floor((total * w) / weightSum)
  );
  let remainder = total - sizes.reduce((sum, size) => sum + size, 0);

  const byWeightDesc = effectiveWeights
    .map((w, i) => ({ w, i }))
    .sort((a, b) => b.w - a.w)
    .map((entry) => entry.i);

  let cursor = 0;
  while (remainder > 0 && byWeightDesc.length > 0) {
    sizes[byWeightDesc[cursor % byWeightDesc.length]] += 1;
    remainder -= 1;
    cursor += 1;
  }
  return sizes;
};

interface WorkingLevel {
  priceInTicks: bigint;
  priceUsd: number;
  weight: number;
}

/**
 * Compute a full scale-order preview: the laddered levels plus the validation
 * the UI needs to surface (order-count cap, size-vs-count limit, merged
 * duplicate ticks).
 *
 * Price levels warp a normalized position `t in [0, 1]` with a
 * mirror-symmetric power law: a positive `priceBias` applies `t^(2^-bias)`
 * (clusters toward the upper bound) and a negative bias applies the reflection
 * `1 - (1-t)^(2^-|bias|)` (clusters toward the lower bound), so `bias` and
 * `-bias` yield exactly mirrored ladders. Levels snap to the **nearest** tick
 * and any that land on the same tick are merged.
 *
 * Base size is allocated so the requested order count is preserved: every
 * surviving level is first reserved `minBaseLotsPerOrder` (the smallest tradable
 * amount), and the remaining surplus is distributed by a linear weight ramp
 * `1 + sizeBias*(2t - 1)`, floored to whole base lots with the remainder going
 * to the largest-weight levels. The smallest levels are therefore rounded **up**
 * to the minimum rather than dropped, and the allocated total always equals
 * `totalBaseLots`. The count is only reduced when the size cannot support it
 * (`INSUFFICIENT_SIZE_FOR_COUNT`) or when levels collide on the same tick
 * (`LEVELS_MERGED_DUPLICATE_TICK`).
 */
export const previewScaleOrder = (
  input: ScaleOrderInput
): ScaleOrderPreview => {
  const marketParams = marketParamsOf(input);
  const priceBias = clampScaleBias(input.priceBias);
  const sizeBias = clampScaleBias(input.sizeBias);
  const totalBaseLots = Math.max(0, Math.floor(input.totalBaseLots));
  const requestedOrderCount = Number.isFinite(input.orderCount)
    ? Math.floor(input.orderCount)
    : 0;
  const minPerOrder = Math.max(
    1,
    Math.floor(input.minBaseLotsPerOrder ?? DEFAULT_MIN_BASE_LOTS_PER_ORDER)
  );

  const warnings: ScaleOrderWarning[] = [];

  const hasValidRange =
    Number.isFinite(input.lowerPrice) &&
    Number.isFinite(input.upperPrice) &&
    input.lowerPrice > 0 &&
    input.upperPrice > 0 &&
    input.lowerPrice < input.upperPrice;

  if (!hasValidRange) {
    warnings.push({
      code: "INVALID_PRICE_RANGE",
      message:
        "Scale order requires a positive price range with lowerPrice < upperPrice.",
    });
  }
  if (totalBaseLots <= 0) {
    warnings.push({
      code: "NON_POSITIVE_TOTAL_SIZE",
      message: "Scale order total size must be greater than zero.",
    });
  }

  const maxOrderCountForSize =
    totalBaseLots <= 0
      ? 0
      : Math.min(MAX_SCALE_ORDERS, Math.floor(totalBaseLots / minPerOrder));

  const invalid = (distinctTickCount = 0): ScaleOrderPreview => ({
    levels: [],
    requestedOrderCount,
    effectiveOrderCount: 0,
    maxOrderCountForSize,
    distinctTickCount,
    totalBaseLotsAllocated: 0,
    isValid: false,
    warnings,
  });

  if (!hasValidRange || totalBaseLots <= 0) {
    return invalid();
  }

  // Capacity of the range, measured in distinct ticks between the snapped bounds.
  const lowerSnap = snapToNearestTick(input.lowerPrice, marketParams);
  const upperSnap = snapToNearestTick(input.upperPrice, marketParams);
  const distinctTickCount = Math.max(
    1,
    Number(upperSnap.priceInTicks - lowerSnap.priceInTicks) + 1
  );

  if (requestedOrderCount > MAX_SCALE_ORDERS) {
    warnings.push({
      code: "ORDER_COUNT_EXCEEDS_MAX",
      message: `Requested ${requestedOrderCount} orders exceeds the on-chain limit of ${MAX_SCALE_ORDERS} per side; clamped to at most ${MAX_SCALE_ORDERS} (the effective count may be reduced further by available size and tick spacing).`,
    });
  }

  let count = clampScaleOrderCount(requestedOrderCount);
  if (count > maxOrderCountForSize) {
    warnings.push({
      code: "INSUFFICIENT_SIZE_FOR_COUNT",
      message: `Total size of ${totalBaseLots} base lots supports at most ${maxOrderCountForSize} order(s) of ${minPerOrder} base lot(s); reducing order count.`,
    });
    count = maxOrderCountForSize;
  }

  if (count < MIN_SCALE_ORDERS) {
    return invalid(distinctTickCount);
  }

  // Build raw price levels + size weights over `count` evenly-indexed positions.
  // The price warp is mirror-symmetric in `priceBias`: a positive bias warps
  // `t` by `t^(2^-bias)` (concave, clusters toward the upper bound) and a
  // negative bias uses the reflection `1 - (1-t)^(2^-|bias|)` (clusters toward
  // the lower bound), so `bias` and `-bias` produce exactly mirrored ladders.
  // (A plain `t^(2^-bias)` is *not* symmetric: `t^0.5` and `t^2` are inverses,
  // not reflections, so negative biases would skew harder than positive ones.)
  const exponent = Math.pow(2, -Math.abs(priceBias));
  const span = input.upperPrice - input.lowerPrice;
  const weights: number[] = [];
  const snapped: { priceInTicks: bigint; priceUsd: number }[] = [];
  for (let i = 0; i < count; i++) {
    const t = i / (count - 1);
    const warped =
      priceBias >= 0 ? Math.pow(t, exponent) : 1 - Math.pow(1 - t, exponent);
    const rawPrice = input.lowerPrice + span * warped;
    snapped.push(snapToNearestTick(rawPrice, marketParams));
    weights.push(Math.max(0, 1 + sizeBias * (2 * t - 1)));
  }

  // Merge levels that snapped to the same tick (prices are non-decreasing, so
  // collisions are adjacent). Sizes are allocated afterward, so merging only
  // combines weights here.
  const merged: WorkingLevel[] = [];
  let mergedAny = false;
  for (let i = 0; i < count; i++) {
    const prev = merged[merged.length - 1];
    if (prev !== undefined && prev.priceInTicks === snapped[i].priceInTicks) {
      prev.weight += weights[i];
      mergedAny = true;
    } else {
      merged.push({
        priceInTicks: snapped[i].priceInTicks,
        priceUsd: snapped[i].priceUsd,
        weight: weights[i],
      });
    }
  }
  if (mergedAny) {
    warnings.push({
      code: "LEVELS_MERGED_DUPLICATE_TICK",
      message: `Some price levels snapped to the same tick and were merged; ${merged.length} distinct level(s) remain.`,
    });
  }

  if (merged.length < MIN_SCALE_ORDERS) {
    return invalid(distinctTickCount);
  }

  // Reserve the per-order minimum for every surviving level so none falls below
  // the smallest tradable amount, then distribute the surplus by weight. The
  // up-front count clamp guarantees `merged.length * minPerOrder <=
  // totalBaseLots`, so the surplus is non-negative and the total is conserved
  // exactly while the requested order count is preserved (the smallest levels
  // are rounded up to the minimum rather than dropped).
  const surplus = totalBaseLots - merged.length * minPerOrder;
  const surplusShares = allocateByWeight(
    merged.map((level) => level.weight),
    surplus
  );

  const baseLotsMultiplier = 10 ** input.baseLotsDecimals;
  const levels: ScaleOrderLevel[] = merged.map((level, index) => {
    const sizeBaseLots = minPerOrder + surplusShares[index];
    return {
      index,
      priceUsd: level.priceUsd,
      priceInTicks: level.priceInTicks,
      sizeBaseLots,
      sizeUnits: sizeBaseLots / baseLotsMultiplier,
    };
  });

  const totalBaseLotsAllocated = levels.reduce(
    (sum, level) => sum + level.sizeBaseLots,
    0
  );

  return {
    levels,
    requestedOrderCount,
    effectiveOrderCount: levels.length,
    maxOrderCountForSize,
    distinctTickCount,
    totalBaseLotsAllocated,
    isValid: levels.length >= MIN_SCALE_ORDERS,
    warnings,
  };
};

/**
 * Compute the price + size for each sub-order of a scale (multi-limit) order.
 * Returns the final, merged levels (size ≥ per-order minimum, one per tick), or
 * an empty array for invalid input. Use {@link previewScaleOrder} for the
 * validation/warning detail behind an empty or clamped result.
 */
export const computeScaleOrderLevels = (
  input: ScaleOrderInput
): ScaleOrderLevel[] => previewScaleOrder(input).levels;

/**
 * Map computed levels + a {@link Side} into a one-sided {@link MultipleOrderPacket}.
 * `Side.Bid` populates `bids`; `Side.Ask` populates `asks`. Zero-size levels are
 * skipped. Throws if more than {@link MAX_SCALE_ORDERS} orders are supplied (the
 * on-chain per-side cap) — split with {@link chunkScaleLevelsForTx} first.
 */
export const scaleLevelsToMultipleOrderPacket = (
  levels: ScaleOrderLevel[],
  side: Side,
  options?: ScaleLevelsToPacketOptions
): MultipleOrderPacket => ({
  ...levelsToSideOrders(levels, side, (level) => ({
    priceInTicks: level.priceInTicks,
    sizeInBaseLots: BigInt(level.sizeBaseLots),
    lastValidSlot: options?.lastValidSlot ?? null,
  })),
  clientOrderId: options?.clientOrderId ?? null,
  slide: options?.slide ?? false,
});

/**
 * Map computed levels + a {@link Side} into a one-sided {@link MultipleOrderPacketV2}.
 * `Side.Bid` populates `bids`; `Side.Ask` populates `asks`. Zero-size levels are
 * skipped. Fans packet-level `slide`/`reduceOnly` into every leg's flags byte
 * and maps `lastValidSlot: null` to the `0` wire sentinel. Throws if more than
 * {@link MAX_SCALE_ORDERS} orders are supplied (the on-chain per-side cap) —
 * split with {@link chunkScaleLevelsForTx} first.
 */
export const scaleLevelsToMultipleOrderPacketV2 = (
  levels: ScaleOrderLevel[],
  side: Side,
  options?: ScaleLevelsToPacketV2Options
): MultipleOrderPacketV2 => {
  let flags = CondensedOrderFlags.None;
  if (options?.slide) {
    flags |= CondensedOrderFlags.Slide;
  }
  if (options?.reduceOnly) {
    flags |= CondensedOrderFlags.ReduceOnly;
  }

  return {
    ...levelsToSideOrders(levels, side, (level) => ({
      priceInTicks: level.priceInTicks,
      sizeInBaseLots: BigInt(level.sizeBaseLots),
      lastValidSlot: options?.lastValidSlot ?? null,
      flags,
    })),
    clientOrderId: options?.clientOrderId ?? null,
    scaleSetId: options?.scaleSetId ?? 0,
  };
};

const levelsToSideOrders = <TOrder>(
  levels: ScaleOrderLevel[],
  side: Side,
  makeOrder: (level: ScaleOrderLevel) => TOrder
): { bids: TOrder[]; asks: TOrder[] } => {
  const orders = levels
    .filter((level) => level.sizeBaseLots > 0)
    .map(makeOrder);

  if (orders.length > MAX_SCALE_ORDERS) {
    throw new Error(
      `A scale order side may have at most ${MAX_SCALE_ORDERS} orders; got ${orders.length}. Use chunkScaleLevelsForTx to split across transactions.`
    );
  }

  return {
    bids: side === Side.Bid ? orders : [],
    asks: side === Side.Ask ? orders : [],
  };
};

/**
 * Partition levels into transaction-sized chunks (each at most `maxOrdersPerTx`,
 * never above {@link MAX_SCALE_ORDERS}). Pure and deterministic; preserves the
 * input order. A large ladder cannot fit one transaction, so the caller turns
 * each chunk into its own `place_multi_limit_order` transaction.
 *
 * `usesV2Instruction` selects the smaller {@link DEFAULT_MAX_ORDERS_PER_TX_V2}
 * default; an explicit `maxOrdersPerTx` always wins.
 */
export const chunkScaleLevelsForTx = (
  levels: ScaleOrderLevel[],
  options?: { maxOrdersPerTx?: number; usesV2Instruction?: boolean }
): ScaleOrderLevel[][] => {
  const fallback = options?.usesV2Instruction
    ? DEFAULT_MAX_ORDERS_PER_TX_V2
    : DEFAULT_MAX_ORDERS_PER_TX;
  const requestedRaw = options?.maxOrdersPerTx ?? fallback;
  const requested = Number.isFinite(requestedRaw)
    ? Math.floor(requestedRaw)
    : fallback;
  const max = Math.max(1, Math.min(MAX_SCALE_ORDERS, requested));
  const chunks: ScaleOrderLevel[][] = [];
  for (let i = 0; i < levels.length; i += max) {
    chunks.push(levels.slice(i, i + max));
  }
  return chunks;
};

/**
 * Row fields {@link cancelIdsForScaleSet} reads from a trader-state
 * limit-order row. Pass the currently-open rows of a single market —
 * `TraderStateManager#orders(subaccountIndex, symbol)` returns exactly that.
 * When assembling rows by hand, drop `change: "closed"` rows yourself
 * (`status` is `"active"` even on closed rows).
 */
export type ScaleSetCancelableOrderRow = Pick<
  TraderStateMarketLimitOrderRow,
  "priceTicks" | "orderSequenceNumber" | "scaleSetId"
>;

/**
 * Pure filter: convert the rows tagged `scaleSetId` (1-255) into
 * {@link CancelId}s for `buildCancelOrdersByIdIxResolved`, preserving input
 * order. An empty result means the set already left the book — skip the
 * cancel (the builder throws on an empty list). A two-sided set can reach
 * 2 * {@link MAX_SCALE_ORDERS} = 128 ids, above the builder's 100-id cap:
 * split larger results across instructions.
 */
export const cancelIdsForScaleSet = (
  rows: readonly ScaleSetCancelableOrderRow[],
  scaleSetId: number
): CancelId[] => {
  if (!Number.isInteger(scaleSetId) || scaleSetId < 1 || scaleSetId > 255) {
    throw new Error(
      `scaleSetId must be an integer in 1..=255; got ${scaleSetId}`
    );
  }
  const cancelIds: CancelId[] = [];
  for (const row of rows) {
    if (row.scaleSetId !== scaleSetId) {
      continue;
    }
    cancelIds.push({
      nodePointer: null,
      orderId: {
        priceInTicks: ticks(u64(row.priceTicks)),
        orderSequenceNumber: u64(row.orderSequenceNumber),
      },
    });
  }
  return cancelIds;
};
