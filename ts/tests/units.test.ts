import { Side } from "@/primitives/Side";
import {
  baseUnitsToBaseLotsWithMarketParams,
  orderPriceUsdToTicksWithMarketParams,
  parseNonNegativeDecimal,
  priceUsdToTicksWithMarketParams,
  usdToQuoteLots,
  type Rounding,
} from "@/units";
import { describe, expect, it } from "vitest";

const MODES: readonly Rounding[] = ["floor", "ceil", "nearest"];

describe("units", () => {
  it("round-trips float base units and USD amounts without losing a lot", () => {
    const mismatches: string[] = [];
    for (let d = 0; d <= 6; d += 1) {
      const scale = 10 ** d;
      for (let n = 1; n <= 100_000; n += 1) {
        const lots = baseUnitsToBaseLotsWithMarketParams(n / scale, {
          baseLotsDecimals: d,
        });
        if (lots !== BigInt(n)) {
          mismatches.push(`baseLots d=${d} n=${n} got=${lots}`);
        }
      }
    }
    for (let n = 1; n <= 100_000; n += 1) {
      if (usdToQuoteLots(n / 100) !== BigInt(n) * 10_000n) {
        mismatches.push(`quoteLots 2dp n=${n}`);
      }
      if (usdToQuoteLots(n / 1_000_000) !== BigInt(n)) {
        mismatches.push(`quoteLots 6dp n=${n}`);
      }
    }
    expect(mismatches).toEqual([]);
    expect(usdToQuoteLots("1.0000019")).toBe(1_000_001n);
  });

  it("converts prices to ticks in every rounding mode", () => {
    const cent = { tickSize: 100, baseLotsDecimals: 2 };
    const bundled = { tickSize: 1, baseLotsDecimals: -4 };
    const cases: [number | string, typeof cent, [bigint, bigint, bigint]][] = [
      [135.87, cent, [13587n, 13587n, 13587n]],
      ["135.874", cent, [13587n, 13588n, 13587n]],
      [135.875, cent, [13587n, 13588n, 13588n]],
      ["0.000012", bundled, [120000n, 120000n, 120000n]],
      ["0.00000000015", bundled, [1n, 2n, 2n]],
      [1.5e-10, bundled, [1n, 2n, 2n]],
      [1.4e-10, bundled, [1n, 2n, 1n]],
      [1e-7, bundled, [1000n, 1000n, 1000n]],
      [
        1e21,
        { tickSize: 1_000_000, baseLotsDecimals: 6 },
        [10n ** 15n, 10n ** 15n, 10n ** 15n],
      ],
    ];

    for (const [price, params, expected] of cases) {
      expect(
        MODES.map((mode) =>
          priceUsdToTicksWithMarketParams(price, params, mode)
        ),
        String(price)
      ).toEqual(expected);
    }
    expect(priceUsdToTicksWithMarketParams("135.879", cent)).toBe(13587n);

    const mismatches: string[] = [];
    for (let n = 1; n <= 100_000; n += 1) {
      for (const mode of MODES) {
        const got = priceUsdToTicksWithMarketParams(n / 100, cent, mode);
        if (got !== BigInt(n)) {
          mismatches.push(`ticks ${mode} n=${n} got=${got}`);
        }
      }
    }
    expect(mismatches).toEqual([]);
  });

  it("rounds order prices by side and rejects zero ticks", () => {
    const cent = { tickSize: 100, baseLotsDecimals: 2 };
    const orderTicks = (price: string, side: Side) =>
      orderPriceUsdToTicksWithMarketParams(price, cent, side);

    expect(orderTicks("135.87", Side.Bid)).toBe(13587n);
    expect(orderTicks("135.87", Side.Ask)).toBe(13587n);
    expect(orderTicks("135.874", Side.Bid)).toBe(13587n);
    expect(orderTicks("135.874", Side.Ask)).toBe(13588n);
    expect(orderTicks("0.004", Side.Ask)).toBe(1n);
    expect(() => orderTicks("0.004", Side.Bid)).toThrow(
      "order price must be at least one tick"
    );
    expect(() => orderTicks("0", Side.Ask)).toThrow(
      "order price must be at least one tick"
    );
  });

  it("rejects negative and malformed decimals", () => {
    for (const value of [-1, -1e-7, Number.NaN, Infinity, "1e5", "abc", ""]) {
      expect(() => parseNonNegativeDecimal(value, "value")).toThrow();
    }
  });
});
