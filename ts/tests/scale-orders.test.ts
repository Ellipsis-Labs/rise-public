import {
  DEFAULT_MAX_ORDERS_PER_TX,
  DEFAULT_MAX_ORDERS_PER_TX_V2,
  MAX_SCALE_ORDERS,
  MIN_SCALE_ORDERS,
  buildPlaceMultiLimitOrderIxResolved,
  chunkScaleLevelsForTx,
  clampScaleBias,
  clampScaleOrderCount,
  computeScaleOrderLevels,
  CondensedOrderFlags,
  previewScaleOrder,
  priceUsdToTicksWithMarketParams,
  scaleLevelsToMultipleOrderPacket,
  ticksToUsdWithMarketParams,
  type ScaleOrderInput,
  type ScaleOrderLevel,
} from "@/index";
import { scaleLevelsToMultipleOrderPacketV2 } from "@/scaleOrders";
import type { ResolvedPlaceOrderContext } from "@/ixs/types";
import { Side } from "@/primitives/Side";
import { DISCRIMINANTS } from "@/core/discriminants";
import { describe, expect, it } from "vitest";

// SOL-PERP-like params: 3 base lot decimals, tick size 1 (0.001 USD ticks).
const baseInput: ScaleOrderInput = {
  lowerPrice: 100,
  upperPrice: 200,
  orderCount: 5,
  totalBaseLots: 1000,
  priceBias: 0,
  sizeBias: 0,
  baseLotsDecimals: 3,
  tickSize: 1,
};

const sumLots = (levels: ScaleOrderLevel[]): number =>
  levels.reduce((sum, level) => sum + level.sizeBaseLots, 0);

describe("clampScaleBias", () => {
  it("clamps to [-1, 1] and defaults non-finite to 0", () => {
    expect(clampScaleBias(-5)).toBe(-1);
    expect(clampScaleBias(5)).toBe(1);
    expect(clampScaleBias(0.5)).toBe(0.5);
    expect(clampScaleBias(Number.NaN)).toBe(0);
  });
});

describe("clampScaleOrderCount", () => {
  it("clamps to [2, MAX] and floors", () => {
    expect(clampScaleOrderCount(1)).toBe(MIN_SCALE_ORDERS);
    expect(clampScaleOrderCount(1000)).toBe(MAX_SCALE_ORDERS);
    expect(clampScaleOrderCount(4.9)).toBe(4);
  });
});

describe("computeScaleOrderLevels", () => {
  it("returns empty for invalid ranges and sizes", () => {
    expect(
      computeScaleOrderLevels({
        ...baseInput,
        lowerPrice: 200,
        upperPrice: 100,
      })
    ).toEqual([]);
    expect(computeScaleOrderLevels({ ...baseInput, lowerPrice: 0 })).toEqual(
      []
    );
    expect(computeScaleOrderLevels({ ...baseInput, totalBaseLots: 0 })).toEqual(
      []
    );
  });

  it("clamps an order count below the minimum up to the minimum", () => {
    const levels = computeScaleOrderLevels({ ...baseInput, orderCount: 1 });
    expect(levels).toHaveLength(2);
    expect(sumLots(levels)).toBe(1000);
  });

  it("spaces prices evenly with zero price bias", () => {
    const levels = computeScaleOrderLevels(baseInput);
    expect(levels).toHaveLength(5);
    expect(levels[0].priceUsd).toBeCloseTo(100, 6);
    expect(levels[1].priceUsd).toBeCloseTo(125, 6);
    expect(levels[2].priceUsd).toBeCloseTo(150, 6);
    expect(levels[3].priceUsd).toBeCloseTo(175, 6);
    expect(levels[4].priceUsd).toBeCloseTo(200, 6);
  });

  it("conserves the total base lots across levels (even)", () => {
    const levels = computeScaleOrderLevels(baseInput);
    expect(sumLots(levels)).toBe(1000);
    for (const level of levels) {
      expect(level.sizeBaseLots).toBe(200);
    }
  });

  it("conserves total even when not evenly divisible", () => {
    const levels = computeScaleOrderLevels({
      ...baseInput,
      totalBaseLots: 1003,
    });
    expect(sumLots(levels)).toBe(1003);
  });

  it("clusters price levels toward the lower bound for negative price bias", () => {
    const even = computeScaleOrderLevels(baseInput);
    const lower = computeScaleOrderLevels({ ...baseInput, priceBias: -1 });
    expect(lower[2].priceUsd).toBeLessThan(even[2].priceUsd);
    expect(lower[0].priceUsd).toBeCloseTo(100, 6);
    expect(lower[4].priceUsd).toBeCloseTo(200, 6);
  });

  it("clusters price levels toward the upper bound for positive price bias", () => {
    const even = computeScaleOrderLevels(baseInput);
    const higher = computeScaleOrderLevels({ ...baseInput, priceBias: 1 });
    expect(higher[2].priceUsd).toBeGreaterThan(even[2].priceUsd);
    expect(higher[0].priceUsd).toBeCloseTo(100, 6);
    expect(higher[4].priceUsd).toBeCloseTo(200, 6);
  });

  it("mirrors the price ladder for opposite price biases", () => {
    const lower = computeScaleOrderLevels({ ...baseInput, priceBias: -1 });
    const higher = computeScaleOrderLevels({ ...baseInput, priceBias: 1 });
    const span = baseInput.upperPrice - baseInput.lowerPrice;
    // Level i from the bottom under -bias should sit the same distance off the
    // lower bound as level (n-1-i) from the top under +bias sits off the upper
    // bound: priceLow[i] - lower === upper - priceHigh[n-1-i].
    const n = lower.length;
    expect(higher).toHaveLength(n);
    for (let i = 0; i < n; i++) {
      const offLower = lower[i].priceUsd - baseInput.lowerPrice;
      const offUpper = baseInput.upperPrice - higher[n - 1 - i].priceUsd;
      expect(offLower).toBeCloseTo(offUpper, 6);
    }
    // Sanity: the tightest gap off the bottom (-bias) equals the tightest gap
    // off the top (+bias), unlike the old asymmetric `t^(2^-bias)` warp.
    expect(lower[1].priceUsd - lower[0].priceUsd).toBeCloseTo(
      higher[n - 1].priceUsd - higher[n - 2].priceUsd,
      6
    );
    expect(span).toBeGreaterThan(0);
  });

  it("allocates more size to lower prices for negative size bias", () => {
    const levels = computeScaleOrderLevels({ ...baseInput, sizeBias: -0.5 });
    expect(sumLots(levels)).toBe(1000);
    expect(levels[0].sizeBaseLots).toBeGreaterThan(
      levels[levels.length - 1].sizeBaseLots
    );
  });

  it("allocates more size to higher prices for positive size bias", () => {
    const levels = computeScaleOrderLevels({ ...baseInput, sizeBias: 0.5 });
    expect(sumLots(levels)).toBe(1000);
    expect(levels[levels.length - 1].sizeBaseLots).toBeGreaterThan(
      levels[0].sizeBaseLots
    );
  });

  it("clamps order count to the max and conserves total", () => {
    const levels = computeScaleOrderLevels({ ...baseInput, orderCount: 1000 });
    expect(levels).toHaveLength(MAX_SCALE_ORDERS);
    expect(sumLots(levels)).toBe(1000);
  });
});

describe("nearest-tick snapping", () => {
  it("rounds the top level UP to the nearest tick instead of flooring", () => {
    // tick size 10 at 3 decimals => 0.01 USD ticks. 100.027 is closest to 100.03,
    // which floor truncation would miss (it would land on 100.02).
    const levels = computeScaleOrderLevels({
      ...baseInput,
      lowerPrice: 100,
      upperPrice: 100.027,
      orderCount: 2,
      tickSize: 10,
    });
    expect(levels).toHaveLength(2);
    expect(levels[0].priceUsd).toBeCloseTo(100, 6);
    expect(levels[1].priceUsd).toBeCloseTo(100.03, 6);
  });

  it("keeps every level on a tick boundary", () => {
    const levels = computeScaleOrderLevels({
      ...baseInput,
      lowerPrice: 100.003,
      upperPrice: 100.027,
      orderCount: 3,
      tickSize: 10,
    });
    for (const level of levels) {
      // 0.01 USD ticks => price * 100 is an integer.
      expect(
        Math.abs(Math.round(level.priceUsd * 100) - level.priceUsd * 100)
      ).toBeLessThan(1e-6);
    }
  });
});

describe("duplicate-tick merging", () => {
  it("merges levels that snap to the same tick, conserving total", () => {
    // Range [100, 100.02] with 0.01 ticks supports only 3 distinct ticks.
    const preview = previewScaleOrder({
      ...baseInput,
      lowerPrice: 100,
      upperPrice: 100.02,
      orderCount: 10,
      tickSize: 10,
    });
    expect(preview.isValid).toBe(true);
    expect(preview.effectiveOrderCount).toBeLessThanOrEqual(3);
    expect(preview.effectiveOrderCount).toBeGreaterThanOrEqual(
      MIN_SCALE_ORDERS
    );
    expect(sumLots(preview.levels)).toBe(1000);
    expect(
      preview.warnings.some((w) => w.code === "LEVELS_MERGED_DUPLICATE_TICK")
    ).toBe(true);
    // No two levels share a tick.
    const ticksSeen = new Set(
      preview.levels.map((l) => l.priceInTicks.toString())
    );
    expect(ticksSeen.size).toBe(preview.levels.length);
  });
});

describe("flooring sub-minimum levels to the per-order minimum", () => {
  it("rounds a zero-weight end level up to the minimum, preserving count and total", () => {
    const preview = previewScaleOrder({ ...baseInput, sizeBias: 1 });
    expect(preview.isValid).toBe(true);
    // The lowest-price level has weight 0 at sizeBias=1; instead of being
    // dropped it is rounded up to the per-order minimum, so all 5 levels remain.
    expect(preview.effectiveOrderCount).toBe(5);
    expect(sumLots(preview.levels)).toBe(1000);
    // Smallest level is floored to the minimum trade amount (1 base lot here).
    expect(preview.levels[0].sizeBaseLots).toBe(1);
    for (const level of preview.levels) {
      expect(level.sizeBaseLots).toBeGreaterThanOrEqual(1);
    }
  });

  it("honors a custom per-order minimum as the floor for the smallest level", () => {
    const preview = previewScaleOrder({
      ...baseInput,
      sizeBias: 1,
      minBaseLotsPerOrder: 25,
    });
    expect(preview.isValid).toBe(true);
    expect(preview.effectiveOrderCount).toBe(5);
    expect(sumLots(preview.levels)).toBe(1000);
    for (const level of preview.levels) {
      expect(level.sizeBaseLots).toBeGreaterThanOrEqual(25);
    }
    expect(preview.levels[0].sizeBaseLots).toBe(25);
  });
});

describe("ticksToUsdWithMarketParams", () => {
  it("round-trips priceUsdToTicksWithMarketParams", () => {
    for (const params of [
      { tickSize: 1, baseLotsDecimals: 3 },
      { tickSize: 100, baseLotsDecimals: 2 },
      { tickSize: 5, baseLotsDecimals: -2 },
    ]) {
      for (const priceUsd of ["100", "135.87", "0.5"]) {
        const t = priceUsdToTicksWithMarketParams(priceUsd, params);
        expect(ticksToUsdWithMarketParams(t, params)).toBeCloseTo(
          Number(priceUsd),
          6
        );
      }
    }
  });
});

describe("previewScaleOrder", () => {
  it("warns and clamps when the requested count exceeds the max", () => {
    const preview = previewScaleOrder({ ...baseInput, orderCount: 1000 });
    expect(preview.requestedOrderCount).toBe(1000);
    expect(preview.effectiveOrderCount).toBe(MAX_SCALE_ORDERS);
    expect(
      preview.warnings.some((w) => w.code === "ORDER_COUNT_EXCEEDS_MAX")
    ).toBe(true);
  });

  it("reports max order count for the given size and clamps", () => {
    const preview = previewScaleOrder({
      ...baseInput,
      totalBaseLots: 3,
      orderCount: 5,
    });
    expect(preview.maxOrderCountForSize).toBe(3);
    expect(preview.effectiveOrderCount).toBeLessThan(
      preview.requestedOrderCount
    );
    expect(sumLots(preview.levels)).toBe(3);
    expect(
      preview.warnings.some((w) => w.code === "INSUFFICIENT_SIZE_FOR_COUNT")
    ).toBe(true);
  });

  it("honors a custom per-order minimum in maxOrderCountForSize", () => {
    const preview = previewScaleOrder({
      ...baseInput,
      totalBaseLots: 100,
      minBaseLotsPerOrder: 10,
    });
    expect(preview.maxOrderCountForSize).toBe(10);
  });

  it("is invalid for an inverted range", () => {
    const preview = previewScaleOrder({
      ...baseInput,
      lowerPrice: 200,
      upperPrice: 100,
    });
    expect(preview.isValid).toBe(false);
    expect(preview.levels).toEqual([]);
    expect(preview.warnings.some((w) => w.code === "INVALID_PRICE_RANGE")).toBe(
      true
    );
  });
});

describe("scaleLevelsToMultipleOrderPacket", () => {
  const levels = computeScaleOrderLevels(baseInput);

  it("places a buy ladder on the bid side", () => {
    const packet = scaleLevelsToMultipleOrderPacket(levels, Side.Bid);
    expect(packet.asks).toHaveLength(0);
    expect(packet.bids).toHaveLength(levels.length);
    expect(packet.bids[0].priceInTicks).toBe(levels[0].priceInTicks);
    expect(packet.bids[0].sizeInBaseLots).toBe(BigInt(levels[0].sizeBaseLots));
    expect(packet.bids[0].lastValidSlot).toBeNull();
    expect(packet.slide).toBe(false);
    expect(packet.clientOrderId).toBeNull();
  });

  it("places a sell ladder on the ask side and honors options", () => {
    const packet = scaleLevelsToMultipleOrderPacket(levels, Side.Ask, {
      slide: true,
      clientOrderId: 7n,
      lastValidSlot: 123n,
    });
    expect(packet.bids).toHaveLength(0);
    expect(packet.asks).toHaveLength(levels.length);
    expect(packet.asks[0].lastValidSlot).toBe(123n);
    expect(packet.slide).toBe(true);
    expect(packet.clientOrderId).toBe(7n);
  });

  it("skips zero-size levels", () => {
    const withZero: ScaleOrderLevel[] = [
      ...levels,
      {
        index: 99,
        priceUsd: 999,
        priceInTicks: 999000n,
        sizeBaseLots: 0,
        sizeUnits: 0,
      },
    ];
    const packet = scaleLevelsToMultipleOrderPacket(withZero, Side.Bid);
    expect(packet.bids).toHaveLength(levels.length);
  });

  it("throws when more than the per-side cap is supplied", () => {
    const tooMany: ScaleOrderLevel[] = Array.from(
      { length: MAX_SCALE_ORDERS + 1 },
      (_, i) => ({
        index: i,
        priceUsd: 100 + i,
        priceInTicks: BigInt(100000 + i),
        sizeBaseLots: 1,
        sizeUnits: 0.001,
      })
    );
    expect(() => scaleLevelsToMultipleOrderPacket(tooMany, Side.Bid)).toThrow();
  });
});

describe("scaleLevelsToMultipleOrderPacketV2", () => {
  const levels = computeScaleOrderLevels(baseInput);

  it("defaults to no flags, no scaleSetId, and the 0-sentinel expiry", () => {
    const packet = scaleLevelsToMultipleOrderPacketV2(levels, Side.Bid);
    expect(packet.asks).toHaveLength(0);
    expect(packet.bids).toHaveLength(levels.length);
    expect(packet.scaleSetId).toBe(0);
    for (const order of packet.bids) {
      expect(order.flags).toBe(CondensedOrderFlags.None);
      expect(order.lastValidSlot).toBeNull();
    }
  });

  it("fans packet-level slide and reduceOnly into every leg's flags", () => {
    const packet = scaleLevelsToMultipleOrderPacketV2(levels, Side.Ask, {
      slide: true,
      reduceOnly: true,
      clientOrderId: 7n,
      lastValidSlot: 123n,
      scaleSetId: 42,
    });
    expect(packet.bids).toHaveLength(0);
    expect(packet.asks).toHaveLength(levels.length);
    expect(packet.clientOrderId).toBe(7n);
    expect(packet.scaleSetId).toBe(42);
    for (const order of packet.asks) {
      expect(order.flags).toBe(
        CondensedOrderFlags.Slide | CondensedOrderFlags.ReduceOnly
      );
      expect(order.lastValidSlot).toBe(123n);
    }
  });

  it("skips zero-size levels", () => {
    const withZero: ScaleOrderLevel[] = [
      ...levels,
      {
        index: 99,
        priceUsd: 999,
        priceInTicks: 999000n,
        sizeBaseLots: 0,
        sizeUnits: 0,
      },
    ];
    const packet = scaleLevelsToMultipleOrderPacketV2(withZero, Side.Bid);
    expect(packet.bids).toHaveLength(levels.length);
  });

  it("throws when more than the per-side cap is supplied", () => {
    const tooMany: ScaleOrderLevel[] = Array.from(
      { length: MAX_SCALE_ORDERS + 1 },
      (_, i) => ({
        index: i,
        priceUsd: 100 + i,
        priceInTicks: BigInt(100000 + i),
        sizeBaseLots: 1,
        sizeUnits: 0.001,
      })
    );
    expect(() =>
      scaleLevelsToMultipleOrderPacketV2(tooMany, Side.Bid)
    ).toThrow();
  });
});

describe("chunkScaleLevelsForTx", () => {
  const levels = computeScaleOrderLevels({ ...baseInput, orderCount: 64 });

  it("splits a 64-order ladder into transaction-sized chunks", () => {
    const chunks = chunkScaleLevelsForTx(levels, { maxOrdersPerTx: 30 });
    expect(chunks.map((c) => c.length)).toEqual([30, 30, 4]);
    // Order preserved, union equals input.
    expect(chunks.flat()).toEqual(levels);
  });

  it("never exceeds the on-chain per-side cap even with a large request", () => {
    const chunks = chunkScaleLevelsForTx(levels, { maxOrdersPerTx: 1000 });
    expect(chunks).toHaveLength(1);
    expect(chunks[0].length).toBeLessThanOrEqual(MAX_SCALE_ORDERS);
  });

  it("uses the smaller V2 default for V2 batches", () => {
    expect(DEFAULT_MAX_ORDERS_PER_TX_V2).toBeLessThan(
      DEFAULT_MAX_ORDERS_PER_TX
    );
    expect(
      chunkScaleLevelsForTx(levels, { usesV2Instruction: true }).map(
        (c) => c.length
      )
    ).toEqual([
      DEFAULT_MAX_ORDERS_PER_TX_V2,
      DEFAULT_MAX_ORDERS_PER_TX_V2,
      MAX_SCALE_ORDERS - 2 * DEFAULT_MAX_ORDERS_PER_TX_V2,
    ]);
    expect(chunkScaleLevelsForTx(levels).map((c) => c.length)[0]).toEqual(
      DEFAULT_MAX_ORDERS_PER_TX
    );
  });

  it("lets an explicit maxOrdersPerTx override the V2 default", () => {
    const chunks = chunkScaleLevelsForTx(levels, {
      maxOrdersPerTx: 32,
      usesV2Instruction: true,
    });
    expect(chunks.map((c) => c.length)).toEqual([32, 32]);
  });
});

describe("buildPlaceMultiLimitOrderIxResolved", () => {
  const resolvedOrderContext = {
    exchange: {
      phoenixProgramAddress: "phoenix-program" as never,
      logAuthorityAddress: "log-authority" as never,
      globalConfigurationAddress: "global-config" as never,
      perpAssetMap: "perp-asset-map" as never,
      globalTraderIndex: ["gti-0"] as never,
      activeTraderBuffer: ["atb-0"] as never,
    },
    market: {
      marketAddress: "market-address" as never,
      splineCollection: "spline-address" as never,
    },
    trader: {
      authority: "trader-authority" as never,
      positionAuthority: "position-authority" as never,
      traderAccount: "trader-account" as never,
    },
  } as const satisfies ResolvedPlaceOrderContext;

  const packet = scaleLevelsToMultipleOrderPacket(
    computeScaleOrderLevels(baseInput),
    Side.Bid
  );

  it("encodes the place_multi_limit_order discriminant and resolves the signer", () => {
    const ix = buildPlaceMultiLimitOrderIxResolved({
      ...resolvedOrderContext,
      multipleOrderPacket: packet,
    });

    expect(ix.programAddress).toBe("phoenix-program");
    expect(Array.from(ix.data.slice(0, 8))).toEqual(
      Array.from(DISCRIMINANTS.PLACE_MULTI_LIMIT_ORDER)
    );
    // trader signer = positionAuthority ?? authority
    expect(ix.accounts[3]?.address).toBe("position-authority");
    expect(ix.accounts[4]?.address).toBe("trader-account");
    expect(ix.accounts.at(-2)?.address).toBe("market-address");
    expect(ix.accounts.at(-1)?.address).toBe("spline-address");
  });

  it("is margin-agnostic: the supplied subaccount PDA propagates (cross vs isolated)", () => {
    const isolated = buildPlaceMultiLimitOrderIxResolved({
      ...resolvedOrderContext,
      trader: {
        ...resolvedOrderContext.trader,
        traderAccount: "iso-subaccount" as never,
      },
      multipleOrderPacket: packet,
    });
    expect(isolated.accounts[4]?.address).toBe("iso-subaccount");
  });
});
