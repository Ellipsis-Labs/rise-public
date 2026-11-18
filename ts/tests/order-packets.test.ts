import {
  buildLimitOrderPacketFromMarketParams,
  buildMarketOrderPacketFromMarketParams,
  priceUsdToTicksWithMarketParams,
} from "@/index";
import { baseLots, quoteLots, ticks } from "@/primitives/_numberTypes";
import { Side } from "@/primitives/Side";
import { OrderFlags, SelfTradeBehavior } from "@/primitives/OrderPacket";
import { describe, expect, it } from "vitest";

describe("order packet builders", () => {
  it("builds a limit order packet synchronously from market params", () => {
    const packet = buildLimitOrderPacketFromMarketParams(
      {
        side: Side.Bid,
        priceUsd: "135.87",
        baseUnits: "0.25",
        selfTradeBehavior: SelfTradeBehavior.CancelProvide,
        clientOrderId: 7n,
        orderFlags: OrderFlags.ReduceOnly,
      },
      {
        tickSize: 100,
        baseLotsDecimals: 2,
      }
    );

    expect(packet).toEqual({
      side: Side.Bid,
      priceInTicks: ticks(13587n),
      numBaseLots: baseLots(25n),
      selfTradeBehavior: SelfTradeBehavior.CancelProvide,
      matchLimit: null,
      clientOrderId: 7n,
      lastValidSlot: null,
      orderFlags: OrderFlags.ReduceOnly,
      cancelExisting: false,
    });
  });

  it("rounds off-tick order prices down for bids and up for asks", () => {
    const marketParams = { tickSize: 100, baseLotsDecimals: 2 };
    const priceFor = (side: Side) =>
      buildLimitOrderPacketFromMarketParams(
        { side, priceUsd: "135.875", baseUnits: "1" },
        marketParams
      ).priceInTicks;
    const limitFor = (side: Side) =>
      buildMarketOrderPacketFromMarketParams(
        { side, baseUnits: "1", priceLimitUsd: 135.875 },
        marketParams
      ).priceInTicks;

    expect(priceFor(Side.Bid)).toEqual(ticks(13587n));
    expect(priceFor(Side.Ask)).toEqual(ticks(13588n));
    expect(limitFor(Side.Bid)).toEqual(ticks(13587n));
    expect(limitFor(Side.Ask)).toEqual(ticks(13588n));
  });

  it("handles negative base lot decimals when converting price and size", () => {
    expect(
      priceUsdToTicksWithMarketParams("0.000012", {
        tickSize: 1,
        baseLotsDecimals: -4,
      })
    ).toEqual(ticks(120000n));

    const packet = buildMarketOrderPacketFromMarketParams(
      {
        side: Side.Ask,
        baseUnits: "20000",
        priceLimitUsd: "0.000012",
      },
      {
        tickSize: 1,
        baseLotsDecimals: -4,
      }
    );

    expect(packet.priceInTicks).toEqual(ticks(120000n));
    expect(packet.numBaseLots).toEqual(baseLots(2n));
    expect(packet.minBaseLotsToFill).toEqual(baseLots(2n));
  });

  describe("priceUsdToTicksWithMarketParams on negative base lot decimals", () => {
    // Reference values match the `to_ticks` tests in
    // `rise/rust/math/src/price.rs`: ticks are the exact floor of
    // priceUsd * 1e6 * 10^-d / tickSize, with no intermediate rounding to
    // micro-USD.
    it("keeps digits finer than one micro-USD per base unit", () => {
      // PUMP-like market: tickSize = 10, d = -2, so one tick is 1e-7 USD.
      const pump = { tickSize: 10, baseLotsDecimals: -2 };
      expect(priceUsdToTicksWithMarketParams("0.004853012", pump)).toEqual(
        ticks(48530n)
      );
      // Sub-micro-USD digits cross tick boundaries: 48_539.95 -> 48_539.
      // Flooring to micro-USD first (0.004853) gave 48_530.
      expect(priceUsdToTicksWithMarketParams("0.004853995", pump)).toEqual(
        ticks(48539n)
      );

      // Same lots, tickSize = 1 (1e-8 USD per tick).
      expect(
        priceUsdToTicksWithMarketParams("0.004853012", {
          tickSize: 1,
          baseLotsDecimals: -2,
        })
      ).toEqual(ticks(485301n));
    });

    it("matches the ultra-low-price sizing doc example", () => {
      // dev-docs/09: P = 0.00001146, d = -4, tickSize = 1 => 1e-10 USD per
      // tick => 114_600 ticks. Micro-USD flooring gave 110_000.
      const params = { tickSize: 1, baseLotsDecimals: -4 };
      expect(priceUsdToTicksWithMarketParams("0.00001146", params)).toEqual(
        ticks(114600n)
      );
      expect(priceUsdToTicksWithMarketParams("0.0001146", params)).toEqual(
        ticks(1146000n)
      );
      // Trailing zeros in the input must not change the result.
      expect(priceUsdToTicksWithMarketParams("0.000011460000", params)).toEqual(
        ticks(114600n)
      );
    });

    it("still floors to the tick boundary for non-negative decimals", () => {
      // BTC-like: tickSize = 100, d = 4 => 1 USD per tick.
      expect(
        priceUsdToTicksWithMarketParams("99443.1002", {
          tickSize: 100,
          baseLotsDecimals: 4,
        })
      ).toEqual(ticks(99443n));
      // DOGE-like: d = 0, price has more digits than one micro-USD.
      expect(
        priceUsdToTicksWithMarketParams("0.2345678", {
          tickSize: 1,
          baseLotsDecimals: 0,
        })
      ).toEqual(ticks(234567n));
    });
  });

  it("builds a market order packet with explicit minimum fill overrides", () => {
    const packet = buildMarketOrderPacketFromMarketParams(
      {
        side: Side.Bid,
        baseUnits: "1.50",
        minBaseUnitsToFill: "1.25",
        minQuoteLotsToFill: quoteLots(10n),
      },
      {
        tickSize: 100,
        baseLotsDecimals: 3,
      }
    );

    expect(packet).toMatchObject({
      side: Side.Bid,
      priceInTicks: null,
      numBaseLots: baseLots(1500n),
      minBaseLotsToFill: baseLots(1250n),
      minQuoteLotsToFill: quoteLots(10n),
      selfTradeBehavior: SelfTradeBehavior.Abort,
    });
  });

  it("preserves an explicit zero minimum fill override", () => {
    const packet = buildMarketOrderPacketFromMarketParams(
      {
        side: Side.Bid,
        baseUnits: "1.50",
        minBaseUnitsToFill: 0,
      },
      {
        tickSize: 100,
        baseLotsDecimals: 3,
      }
    );

    expect(packet).toMatchObject({
      side: Side.Bid,
      numBaseLots: baseLots(1500n),
      minBaseLotsToFill: baseLots(0n),
    });
  });
});
