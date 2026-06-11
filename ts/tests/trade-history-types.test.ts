import { describe, expect, it } from "vitest";
import {
  FillRecordSchema,
  TradeHistoryV2ItemSchema,
  UserLiquidationHistoryPointSchema,
} from "@/api/trades/types";

describe("trade history types", () => {
  it("parses fillId for the legacy trades-history endpoint", () => {
    const parsed = FillRecordSchema.parse({
      userId: 1,
      traderId: 2,
      traderPdaIndex: 0,
      subaccountIndex: 0,
      marketSymbol: "SOL-PERP",
      signature: "5PRu7zP_mnhT5c",
      fillId: "43148c7f-1389-34f5-99f0-c7296f5858c2",
      timestamp: 1_745_238_400_000,
      slot: 377700000,
      slotIndex: 2442,
      eventIndex: 0,
      instructionIndex: 4,
      instructionType: "PlaceMarketOrder",
      baseLotsBefore: "0",
      baseLotsAfter: "1",
      baseLotsDelta: "1",
      virtualQuoteLotsBefore: "0",
      virtualQuoteLotsAfter: "150000",
      virtualQuoteLotsDelta: "150000",
      price: "150",
      realizedPnl: "1",
      fees: "0.1",
      liquidity: "taker",
      orderSequenceNumber: null,
      splineSequenceNumber: null,
      tradeType: "market",
    });

    expect(parsed.fillId).toBe("43148c7f-1389-34f5-99f0-c7296f5858c2");
  });

  it("parses nullable fillId for trades_v2", () => {
    const withFillId = TradeHistoryV2ItemSchema.parse({
      userId: 1,
      traderId: 2,
      traderPdaIndex: 0,
      subaccountIndex: 0,
      marketSymbol: "SOL-PERP",
      signature: "5PRu7zP_mnhT5c",
      fillId: "661ed5ff-f768-3699-ae86-1480171513ca",
      timestamp: 1_745_238_400_000,
      slot: 377700000,
      slotIndex: 2442,
      eventIndex: 0,
      instructionIndex: 4,
      instructionType: "PlaceMarketOrder",
      baseLotsBefore: "0",
      baseLotsAfter: "1",
      baseLotsDelta: "1",
      virtualQuoteLotsBefore: "0",
      virtualQuoteLotsAfter: "150000",
      virtualQuoteLotsDelta: "150000",
      price: "150",
      realizedPnl: "1",
      fees: "0.1",
      liquidity: "taker",
      orderSequenceNumber: null,
      splineSequenceNumber: null,
      tradeType: "market",
    });

    const withoutFillId = TradeHistoryV2ItemSchema.parse({
      userId: 1,
      traderId: 2,
      traderPdaIndex: 0,
      subaccountIndex: 0,
      marketSymbol: "SOL-PERP",
      signature: "5PRu7zP_mnhT5c",
      timestamp: 1_745_238_400_000,
      slot: 377700000,
      slotIndex: 2442,
      eventIndex: 0,
      instructionIndex: 4,
      instructionType: "PlaceMarketOrder",
      baseLotsBefore: "0",
      baseLotsAfter: "1",
      baseLotsDelta: "1",
      virtualQuoteLotsBefore: "0",
      virtualQuoteLotsAfter: "150000",
      virtualQuoteLotsDelta: "150000",
      price: "150",
      realizedPnl: "1",
      fees: "0.1",
      liquidity: "taker",
      orderSequenceNumber: null,
      splineSequenceNumber: null,
      tradeType: "market",
    });

    expect(withFillId.fillId).toBe("661ed5ff-f768-3699-ae86-1480171513ca");
    expect(withoutFillId.fillId).toBeNull();
  });

  it("parses user liquidation history type and ixName fields", () => {
    const parsed = UserLiquidationHistoryPointSchema.parse({
      kind: "market_order",
      type: "market",
      ixName: "LiquidateViaMarketOrder",
      role: "liquidatee",
      slot: "10",
      slotIndex: 2,
      eventIndex: 1,
      timestamp: "1700000000",
      signature: "signature",
      symbol: "SOL-PERP",
      market: "SOL-PERP",
      subaccountIndex: 0,
      liquidatee: "liquidatee",
      liquidator: "liquidator",
      side: "LONG",
      size: "1",
      price: "150",
      positionClosed: true,
      baseLotsFilled: "1",
      quoteLotsFilled: "150",
    });

    expect(parsed.type).toBe("market");
    expect(parsed.ixName).toBe("LiquidateViaMarketOrder");
    expect(parsed.kind).toBe("market_order");
    expect(parsed.size).toBe("1");
  });
});
