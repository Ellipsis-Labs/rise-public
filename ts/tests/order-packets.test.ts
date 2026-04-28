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
