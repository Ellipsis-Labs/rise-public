import { afterEach, describe, expect, it, vi } from "vitest";

import {
  PlaceAttachedConditionalOrderRequestSchema,
  PlaceIsolatedLimitOrderWithConditionalsRequestSchema,
  PlaceIsolatedMarketOrderRequestSchema,
  PlacePositionConditionalOrderRequestSchema,
} from "@/api/orders";
import { PhoenixHttpClient } from "@/index";

describe("orders client flight routing", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("injects default flight routing into isolated order API requests", async () => {
    const derivedCollector =
      "Trader1111111111111111111111111111111111" as never;
    const fetchMock = vi.fn(
      async (_input: RequestInfo | URL, init?: RequestInit) => {
        const body = JSON.parse(String(init?.body)) as Record<string, unknown>;

        expect(body.flightBuilderAuthority).toBe(
          "Builder111111111111111111111111111111111"
        );
        expect(body.flightFeeCollectorTrader).toBe(derivedCollector);

        return new Response(JSON.stringify([]), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      flight: {
        builderAuthority: "Builder111111111111111111111111111111111" as never,
        builderPdaIndex: 2,
        builderSubaccountIndex: 3,
      },
    });
    const getTraderAddressSpy = vi
      .spyOn(client.pda, "getTraderAddress")
      .mockResolvedValue(derivedCollector);

    await client.orders().placeIsolatedLimitOrder({
      authority: "User111111111111111111111111111111111111",
      symbol: "SOL-PERP",
      side: "buy",
      priceInTicks: 100,
      numBaseLots: 1,
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(getTraderAddressSpy).toHaveBeenCalledWith({
      authority: "Builder111111111111111111111111111111111",
      traderPdaIndex: 2,
      subaccountIndex: 3,
      phoenixProgramAddress: client.pda.getProgramAddress(),
    });
  });

  it("injects default flight routing into Flight-routable conditional API requests", async () => {
    const derivedCollector =
      "Trader1111111111111111111111111111111111" as never;
    const bodiesByPath = new Map<string, Record<string, unknown>>();
    const fetchMock = vi.fn(
      async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = new URL(String(input));
        bodiesByPath.set(
          url.pathname,
          JSON.parse(String(init?.body)) as Record<string, unknown>
        );

        return new Response(JSON.stringify([]), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      flight: {
        builderAuthority: "Builder111111111111111111111111111111111" as never,
        builderPdaIndex: 2,
        builderSubaccountIndex: 3,
      },
    });
    const getTraderAddressSpy = vi
      .spyOn(client.pda, "getTraderAddress")
      .mockResolvedValue(derivedCollector);
    const trigger = {
      side: "sell",
      triggerPrice: 120,
    };

    await client.orders().placeIsolatedLimitOrderWithConditionals({
      authority: "User111111111111111111111111111111111111",
      symbol: "SOL-PERP",
      side: "buy",
      priceInTicks: 100,
      numBaseLots: 1,
      greaterTrigger: trigger,
    });
    await client.orders().placeStopLossOrder({
      authority: "User111111111111111111111111111111111111",
      traderPdaIndex: 0,
      symbol: "SOL-PERP",
      side: "buy",
      stopLossTriggerPrice: 95,
      stopLossExecutionPrice: 94,
    });
    await client.orders().placeAttachedConditionalOrder({
      authority: "User111111111111111111111111111111111111",
      traderPdaIndex: 0,
      symbol: "SOL-PERP",
      orderSequenceNumber: "42",
      orderPriceInTicks: 100,
      lessTrigger: trigger,
    });
    await client.orders().placePositionConditionalOrder({
      authority: "User111111111111111111111111111111111111",
      traderPdaIndex: 0,
      symbol: "SOL-PERP",
      sizePercent: 50,
      lessTrigger: trigger,
    });

    expect(fetchMock).toHaveBeenCalledTimes(4);
    for (const endpoint of [
      "/v1/ix/place-isolated-limit-order-with-conditionals",
      "/v1/ix/place-stop-loss-order",
      "/v1/ix/place-attached-conditional-order",
      "/v1/ix/place-position-conditional-order",
    ]) {
      const body = bodiesByPath.get(endpoint);
      expect(body?.flightBuilderAuthority).toBe(
        "Builder111111111111111111111111111111111"
      );
      expect(body?.flightFeeCollectorTrader).toBe(derivedCollector);
    }
    expect(getTraderAddressSpy).toHaveBeenCalledTimes(4);
  });

  it("recomputes the fee collector trader when the request overrides the flight builder", async () => {
    const defaultBuilder = "Builder111111111111111111111111111111111" as never;
    const overrideBuilder = "Builder222222222222222222222222222222222" as never;
    const overrideCollector =
      "Trader2222222222222222222222222222222222" as never;
    const fetchMock = vi.fn(
      async (_input: RequestInfo | URL, init?: RequestInit) => {
        const body = JSON.parse(String(init?.body)) as Record<string, unknown>;

        expect(body.flightBuilderAuthority).toBe(overrideBuilder);
        expect(body.flightFeeCollectorTrader).toBe(overrideCollector);

        return new Response(JSON.stringify([]), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
    );
    vi.stubGlobal("fetch", fetchMock);

    const client = new PhoenixHttpClient({
      baseUrl: "https://example.com",
      flight: {
        builderAuthority: defaultBuilder,
        builderPdaIndex: 4,
        builderSubaccountIndex: 7,
      },
    });
    const getTraderAddressSpy = vi
      .spyOn(client.pda, "getTraderAddress")
      .mockResolvedValue(overrideCollector);

    await client.orders().placeIsolatedLimitOrder({
      authority: "User111111111111111111111111111111111111",
      symbol: "SOL-PERP",
      side: "buy",
      priceInTicks: 100,
      numBaseLots: 1,
      flightBuilderAuthority: overrideBuilder,
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(getTraderAddressSpy).toHaveBeenCalledWith({
      authority: overrideBuilder,
      traderPdaIndex: 4,
      subaccountIndex: 7,
      phoenixProgramAddress: client.pda.getProgramAddress(),
    });
  });
});

describe("conditional order request schemas", () => {
  const trigger = {
    side: "sell",
    triggerPrice: 120,
  };

  it("requires at least one attached conditional trigger", () => {
    expect(
      PlaceAttachedConditionalOrderRequestSchema.safeParse({
        authority: "authority",
        traderPdaIndex: 0,
        symbol: "SOL-PERP",
        orderSequenceNumber: "42",
        orderPriceInTicks: 1234,
      }).success
    ).toBe(false);

    expect(
      PlaceAttachedConditionalOrderRequestSchema.safeParse({
        authority: "authority",
        traderPdaIndex: 0,
        symbol: "SOL-PERP",
        orderSequenceNumber: "42",
        orderPriceInTicks: 1234,
        greaterTrigger: trigger,
      }).success
    ).toBe(true);
  });

  it("requires at least one position conditional trigger and validates size percent", () => {
    const baseRequest = {
      authority: "authority",
      traderPdaIndex: 0,
      symbol: "SOL-PERP",
    };

    expect(
      PlacePositionConditionalOrderRequestSchema.safeParse({
        ...baseRequest,
        sizePercent: 50,
      }).success
    ).toBe(false);
    expect(
      PlacePositionConditionalOrderRequestSchema.safeParse({
        ...baseRequest,
        sizePercent: 0,
        lessTrigger: trigger,
      }).success
    ).toBe(false);
    expect(
      PlacePositionConditionalOrderRequestSchema.safeParse({
        ...baseRequest,
        sizePercent: 101,
        lessTrigger: trigger,
      }).success
    ).toBe(false);
    expect(
      PlacePositionConditionalOrderRequestSchema.safeParse({
        ...baseRequest,
        sizePercent: 100,
        lessTrigger: trigger,
      }).success
    ).toBe(true);
  });

  it("requires at least one trigger for isolated limit orders with conditionals", () => {
    const baseRequest = {
      authority: "authority",
      symbol: "SOL-PERP",
      side: "buy",
      price: 100,
      quantity: 1,
    };

    expect(
      PlaceIsolatedLimitOrderWithConditionalsRequestSchema.safeParse(
        baseRequest
      ).success
    ).toBe(false);
    expect(
      PlaceIsolatedLimitOrderWithConditionalsRequestSchema.safeParse({
        ...baseRequest,
        greaterTrigger: trigger,
      }).success
    ).toBe(true);
  });
});

describe("isolated market order request schema", () => {
  const baseRequest = {
    authority: "authority",
    symbol: "SOL-PERP",
    side: "buy",
    numBaseLots: 25,
  };

  it("accepts positive minimum fills and preserves omission for FOK defaults", () => {
    const parsed = PlaceIsolatedMarketOrderRequestSchema.parse({
      ...baseRequest,
      minBaseLotsToFill: 1,
      minQuoteLotsToFill: 1,
    });

    expect(parsed.minBaseLotsToFill).toBe(1);
    expect(parsed.minQuoteLotsToFill).toBe(1);

    const defaulted = PlaceIsolatedMarketOrderRequestSchema.parse(baseRequest);
    expect(defaulted.minBaseLotsToFill).toBeUndefined();
    expect(defaulted.minQuoteLotsToFill).toBeUndefined();
  });

  it("accepts zero base and quote minimums for true IOC orders", () => {
    const parsed = PlaceIsolatedMarketOrderRequestSchema.parse({
      ...baseRequest,
      minBaseLotsToFill: 0,
      minQuoteLotsToFill: 0,
    });

    expect(parsed.minBaseLotsToFill).toBe(0);
    expect(parsed.minQuoteLotsToFill).toBe(0);
  });
});
