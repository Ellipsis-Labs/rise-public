import { describe, expect, it } from "vitest";

import {
  GetNotificationsResponseSchema,
  NotificationItemSchema,
} from "@/api/notifications/types";
import { NotificationMsgSchema } from "@/ws/adapters/notifications/wire";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";

const base = {
  source: "event",
  id: 42,
  slot: 100,
  slotIndex: 1,
  instructionIndex: 2,
  eventIndex: 3,
  recipientIndex: 0,
  createdAt: "2026-08-17T00:00:00Z",
  acked: false,
};

const futureNotification = {
  ...base,
  notificationType: "margin_call",
  data: { symbol: "SOL-PERP", healthRatio: 0.03 },
  details: { type: "marginCall", symbol: "SOL-PERP" },
};

describe("unknown event notification fallback", () => {
  it("parses an unrecognized notificationType as unknown", () => {
    const parsed = NotificationItemSchema.parse(futureNotification);
    expect(parsed).toMatchObject({
      source: "event",
      notificationType: "unknown",
      rawNotificationType: "margin_call",
      data: { symbol: "SOL-PERP", healthRatio: 0.03 },
      details: { type: "marginCall", symbol: "SOL-PERP" },
    });
  });

  it("parses without optional details", () => {
    const { details: _details, ...withoutDetails } = futureNotification;
    const parsed = NotificationItemSchema.parse(withoutDetails);
    expect(parsed).toMatchObject({
      notificationType: "unknown",
      rawNotificationType: "margin_call",
    });
  });

  it("does not swallow a known notificationType with malformed data", () => {
    const malformed = {
      ...base,
      notificationType: "order_filled",
      data: { bogus: true },
    };
    expect(() => NotificationItemSchema.parse(malformed)).toThrow();
  });

  it("keeps a REST page with an unknown item parseable", () => {
    const parsed = GetNotificationsResponseSchema.parse({
      items: [
        futureNotification,
        {
          source: "general",
          id: 7,
          notificationType: "announcement",
          title: "hi",
          body: "hello",
          data: null,
          createdAt: "2026-08-17T00:00:00Z",
          acked: false,
        },
      ],
      nextCursor: null,
    });
    expect(parsed.items).toHaveLength(2);
    expect(parsed.items[0]).toMatchObject({
      notificationType: "unknown",
      rawNotificationType: "margin_call",
    });
    expect(parsed.items[1]).toMatchObject({ source: "general" });
  });

  it("passes strict-mode WS parsing", () => {
    const strictSchema = applyStrictModeRecursive(NotificationMsgSchema);
    const parsed = strictSchema.parse({
      channel: "notification",
      payload: futureNotification,
      authority: "Auth1111111111111111111111111111111111111111",
      traderPubkey: "Trader11111111111111111111111111111111111111",
      traderPdaIndex: 0,
    });
    expect(parsed.payload).toMatchObject({
      notificationType: "unknown",
      rawNotificationType: "margin_call",
    });
  });
});
