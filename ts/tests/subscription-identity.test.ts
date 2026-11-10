import { subscriptionIdentity } from "@/ws/subscriptionIdentity";
import { describe, expect, it } from "vitest";

// Representative wire requests for every server SubscriptionRequest variant.
const serverRequests = [
  { channel: "allMids" },
  { channel: "market", symbol: "INVALID" },
  { channel: "fundingRate", symbol: "SOL-PERP" },
  { channel: "orderbook", symbol: "SOL-PERP", bypassExecutionBand: true },
  { channel: "l2Book", coin: "SOL-PERP", bypassExecutionBand: true },
  { channel: "events" },
  { channel: "trader", traderKey: "trader-1" },
  { channel: "trades", marketSymbol: "SOL-PERP" },
  { channel: "fills", marketSymbol: "SOL-PERP" },
  { channel: "traderFills", authority: "authority-1", traderPdaIndex: 1 },
  { channel: "candles", symbol: "SOL-PERP", timeframe: "1m" },
  { channel: "markPrice", symbol: "SOL-PERP" },
  { channel: "notifications", userId: "user-1" },
  { channel: "exchange", encoding: "json" },
  { channel: "exchangeStatus" },
  { channel: "marketStats", symbol: "SOL-PERP" },
  { channel: "marketStatsV2", symbols: ["SOL-PERP"] },
  { channel: "traderState", authority: "authority-1", traderPdaIndex: 1 },
  { channel: "activeTraderState", encoding: "base64+zstd" },
  { channel: "traderVolume", authority: "authority-1", traderPdaIndex: 1 },
  { channel: "twap", authority: "authority-1", traderPdaIndex: 1 },
  { channel: "liquidations", symbol: "SOL-PERP" },
  { channel: "riskEvents" },
  { channel: "transaction", userPubkey: "user-1" },
];

describe("subscription identity", () => {
  it.each(serverRequests)(
    "matches the typed $channel echo after unknown fields are dropped",
    (request) => {
      expect(subscriptionIdentity({ ...request, futureOption: true })).toBe(
        subscriptionIdentity(request)
      );
    }
  );

  it.each(serverRequests.filter((request) => Object.keys(request).length > 1))(
    "retains every modeled $channel selector",
    (request) => {
      for (const [field, value] of Object.entries(request)) {
        if (field === "channel") continue;
        expect(
          subscriptionIdentity({ ...request, [field]: undefined })
        ).not.toBe(subscriptionIdentity(request));
        const otherValue = Array.isArray(value)
          ? ["BTC-PERP"]
          : typeof value === "number"
            ? value + 1
            : typeof value === "boolean"
              ? !value
              : `${value}-other`;
        expect(
          subscriptionIdentity({ ...request, [field]: otherValue })
        ).not.toBe(subscriptionIdentity(request));
      }
    }
  );

  it("normalizes supported symbol aliases, case and whitespace", () => {
    expect(
      subscriptionIdentity({ channel: "trades", symbol: " SOL-PERP " })
    ).toBe(
      subscriptionIdentity({ channel: "trades", marketSymbol: "sol-perp" })
    );
    expect(
      subscriptionIdentity({ channel: "l2Book", coin: " SOL-PERP " })
    ).toBe(subscriptionIdentity({ channel: "l2Book", coin: "sol-perp" }));
  });

  it("does not let an unknown symbol field override the modeled selector", () => {
    expect(
      subscriptionIdentity({
        channel: "market",
        symbol: "SOL-PERP",
        coin: "BTC",
      })
    ).toBe(subscriptionIdentity({ channel: "market", symbol: "SOL-PERP" }));
    expect(subscriptionIdentity({ channel: "fills", symbol: "SOL-PERP" })).toBe(
      subscriptionIdentity({ channel: "fills" })
    );
  });

  it("canonicalizes symbol arrays without erasing an explicit empty filter", () => {
    expect(
      subscriptionIdentity({
        channel: "marketStatsV2",
        symbols: [" SOL-PERP ", "btc-perp", "sol-perp"],
      })
    ).toBe(
      subscriptionIdentity({
        channel: "marketStatsV2",
        symbols: ["BTC-PERP", "SOL-PERP"],
      })
    );
    expect(
      subscriptionIdentity({ channel: "marketStatsV2", symbols: [] })
    ).not.toBe(subscriptionIdentity({ channel: "marketStatsV2" }));
  });

  it.each(["orderbook", "l2Book"])(
    "normalizes optional defaults for %s without dropping bypass opt-in",
    (channel) => {
      const request =
        channel === "l2Book"
          ? { channel, coin: "SOL-PERP" }
          : { channel, symbol: "SOL-PERP" };
      expect(
        subscriptionIdentity({ ...request, bypassExecutionBand: false })
      ).toBe(subscriptionIdentity(request));
      expect(
        subscriptionIdentity({ ...request, bypassExecutionBand: null })
      ).toBe(subscriptionIdentity(request));
      expect(
        subscriptionIdentity({ ...request, bypassExecutionBand: true })
      ).not.toBe(subscriptionIdentity(request));
    }
  );

  it("preserves explicit encodings and normalizes absent optional fields", () => {
    expect(subscriptionIdentity({ channel: "exchange", encoding: null })).toBe(
      subscriptionIdentity({ channel: "exchange" })
    );
    expect(
      subscriptionIdentity({ channel: "exchange", encoding: "json" })
    ).not.toBe(
      subscriptionIdentity({ channel: "exchange", encoding: "base64+zstd" })
    );
    expect(
      subscriptionIdentity({ channel: "notifications", userId: null })
    ).toBe(subscriptionIdentity({ channel: "notifications" }));
  });

  it.each(["custom", "toString", "__proto__"])(
    "retains arbitrary fields of the custom channel %s",
    (channel) => {
      const request = { channel, customOption: { b: 2, a: 1 } };
      expect(subscriptionIdentity(request)).toBe(
        subscriptionIdentity({ channel, customOption: { a: 1, b: 2 } })
      );
      for (const extra of [
        { futureOption: true },
        { bypassExecutionBand: false },
        { selector: null },
        { symbol: "SOL-PERP" },
        { coin: "SOL-PERP" },
      ]) {
        expect(subscriptionIdentity({ ...request, ...extra })).not.toBe(
          subscriptionIdentity(request)
        );
      }
    }
  );

  it("rejects values without a string channel", () => {
    for (const value of [null, [], "market", {}, { channel: 1 }]) {
      expect(subscriptionIdentity(value)).toBeNull();
    }
  });
});
