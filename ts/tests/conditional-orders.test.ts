import {
  Direction,
  Side,
  StopLossOrderKind,
  buildTakeProfitStopLossTriggerOrders,
  buildTriggerOrderParams,
  executionPriceFromSlippageBps,
  ticks,
} from "@/index";
import { describe, expect, it } from "vitest";

describe("conditional order helpers", () => {
  it("uses explicit execution price when supplied", () => {
    const trigger = buildTriggerOrderParams({
      triggerDirection: Direction.GreaterThan,
      tradeSide: Side.Ask,
      orderKind: StopLossOrderKind.IOC,
      triggerPrice: ticks(15_000),
      executionPrice: ticks(14_900),
    });

    expect(trigger.executionPrice).toBe(14_900n);
  });

  it("derives sell-side execution price below trigger from slippage bps", () => {
    expect(executionPriceFromSlippageBps(ticks(15_000), Side.Ask, 100)).toBe(
      14_850n
    );
  });

  it("derives buy-side execution price above trigger from slippage bps", () => {
    expect(executionPriceFromSlippageBps(ticks(12_000), Side.Bid, 100)).toBe(
      12_120n
    );
  });

  it("defaults IOC execution price to 10% slippage", () => {
    const trigger = buildTriggerOrderParams({
      triggerDirection: Direction.LessThan,
      tradeSide: Side.Ask,
      triggerPrice: ticks(12_000),
    });

    expect(trigger.orderKind).toBe(StopLossOrderKind.IOC);
    expect(trigger.executionPrice).toBe(10_800n);
  });

  it("defaults limit execution price to the trigger price", () => {
    const trigger = buildTriggerOrderParams({
      triggerDirection: Direction.GreaterThan,
      tradeSide: Side.Ask,
      orderKind: StopLossOrderKind.Limit,
      triggerPrice: ticks(15_000),
    });

    expect(trigger.executionPrice).toBe(15_000n);
  });

  it("moves at least one tick when nonzero slippage rounds to the trigger", () => {
    expect(executionPriceFromSlippageBps(ticks(10), Side.Ask, 1)).toBe(9n);
    expect(executionPriceFromSlippageBps(ticks(10), Side.Bid, 1)).toBe(11n);
  });

  it("rejects ambiguous execution inputs", () => {
    expect(() =>
      buildTriggerOrderParams({
        triggerDirection: Direction.LessThan,
        tradeSide: Side.Bid,
        triggerPrice: ticks(10),
        executionPrice: ticks(11),
        slippageBps: 100,
      })
    ).toThrow("executionPrice or slippageBps");
  });

  it("builds long TP/SL trigger slots like the frontend", () => {
    const orders = buildTakeProfitStopLossTriggerOrders({
      primarySide: Side.Bid,
      takeProfitPrice: ticks(15_000),
      stopLossPrice: ticks(12_000),
    });

    expect(orders.greaterTriggerOrder).toMatchObject({
      triggerDirection: Direction.GreaterThan,
      tradeSide: Side.Ask,
      orderKind: StopLossOrderKind.Limit,
      triggerPrice: 15_000n,
      executionPrice: 15_000n,
    });
    expect(orders.lessTriggerOrder).toMatchObject({
      triggerDirection: Direction.LessThan,
      tradeSide: Side.Ask,
      orderKind: StopLossOrderKind.IOC,
      triggerPrice: 12_000n,
      executionPrice: 10_800n,
    });
  });

  it("builds short TP/SL trigger slots like the frontend", () => {
    const orders = buildTakeProfitStopLossTriggerOrders({
      primarySide: Side.Ask,
      takeProfitPrice: ticks(9_000),
      stopLossPrice: ticks(12_000),
    });

    expect(orders.greaterTriggerOrder).toMatchObject({
      triggerDirection: Direction.GreaterThan,
      tradeSide: Side.Bid,
      orderKind: StopLossOrderKind.IOC,
      triggerPrice: 12_000n,
      executionPrice: 13_200n,
    });
    expect(orders.lessTriggerOrder).toMatchObject({
      triggerDirection: Direction.LessThan,
      tradeSide: Side.Bid,
      orderKind: StopLossOrderKind.Limit,
      triggerPrice: 9_000n,
      executionPrice: 9_000n,
    });
  });
});
