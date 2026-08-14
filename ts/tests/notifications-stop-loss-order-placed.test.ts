import { describe, expect, it } from "vitest";

import {
  GetNotificationsResponseSchema,
  NotificationItemSchema,
} from "@/api/notifications/types";

const stopLossOrderPlaced = {
  source: "event",
  id: 42,
  slot: 100,
  slotIndex: 1,
  instructionIndex: 2,
  eventIndex: 3,
  recipientIndex: 0,
  createdAt: "2026-08-07T00:00:00Z",
  acked: false,
  notificationType: "stop_loss_order_placed",
  data: {
    slot: "100",
    slotIndex: 1,
    timestamp: "1754524800",
    symbol: "SOL-PERP",
    orderSequenceNumber: "7",
    trader: "TraderPubkey11111111111111111111111111111111",
    orderFlags: 5,
    clientOrderId: null,
    orderId: null,
    price: "1234",
    quantity: "-500",
    side: "ask",
    lastValidSlot: null,
    initialSlot: "99",
    orderbookSequenceNumber: "9",
  },
  details: {
    type: "stopLossOrderPlaced",
    symbol: "SOL-PERP",
    side: "ask",
    triggerType: "stop_loss",
    priceInTicks: 1234,
    baseLotsRemaining: -500,
    filledBaseAmount: 1.5,
    filledQuoteAmount: 300.75,
  },
};

describe("stop_loss_order_placed notification", () => {
  it("parses with filledBaseAmount", () => {
    const parsed = NotificationItemSchema.parse(stopLossOrderPlaced);
    expect(parsed).toMatchObject({
      notificationType: "stop_loss_order_placed",
      details: {
        type: "stopLossOrderPlaced",
        filledBaseAmount: 1.5,
        filledQuoteAmount: 300.75,
      },
    });
    if (parsed.source !== "event") throw new Error("expected event source");
    if (parsed.notificationType !== "stop_loss_order_placed") {
      throw new Error("expected stop_loss_order_placed");
    }
    expect(parsed.data.quantity).toBe(-500n);
    expect(parsed.data.orderFlags).toBe(5);
  });

  // Historical pre-deploy rows carry filledBaseAmount without
  // filledQuoteAmount; both fill fields are independently optional.
  it("parses without fill fields (fully rested order)", () => {
    const {
      filledBaseAmount: _omittedBase,
      filledQuoteAmount: _omittedQuote,
      ...details
    } = stopLossOrderPlaced.details;
    const parsed = GetNotificationsResponseSchema.parse({
      items: [{ ...stopLossOrderPlaced, details }],
      nextCursor: null,
    });
    expect(parsed.items).toHaveLength(1);
    expect(parsed.items[0]).toMatchObject({
      notificationType: "stop_loss_order_placed",
    });
    expect(
      (parsed.items[0] as { details?: Record<string, unknown> }).details
    ).not.toHaveProperty("filledBaseAmount");
    expect(
      (parsed.items[0] as { details?: Record<string, unknown> }).details
    ).not.toHaveProperty("filledQuoteAmount");
  });

  it("still parses an existing notification type", () => {
    const parsed = NotificationItemSchema.parse({
      source: "event",
      id: 1,
      slot: 10,
      slotIndex: 0,
      instructionIndex: 0,
      eventIndex: 0,
      recipientIndex: 0,
      createdAt: "2026-08-07T00:00:00Z",
      acked: true,
      notificationType: "stop_loss_executed",
      data: {
        slot: "10",
        slotIndex: 0,
        timestamp: "1754524800",
        symbol: "SOL-PERP",
        taker: "TraderPubkey11111111111111111111111111111111",
        tradeSequenceNumber: "3",
        side: "bid",
        baseLotsFilled: "5",
        quoteLotsFilled: "6",
        feeInQuoteLots: "1",
        baseAmount: 0.5,
        quoteAmount: 100,
        numFills: 1,
      },
      details: {
        type: "stopLossExecuted",
        symbol: "SOL-PERP",
        side: "bid",
        baseAmount: 0.5,
        quoteAmount: 100,
        triggerType: "stop_loss",
      },
    });
    expect(parsed).toMatchObject({ notificationType: "stop_loss_executed" });
  });
});
