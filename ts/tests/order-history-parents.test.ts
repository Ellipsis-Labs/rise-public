import { describe, expect, expectTypeOf, it, afterEach, vi } from "vitest";
import {
  PhoenixHttpClient,
  OrganizedOrderHistoryV2ResponseSchema,
  OrderHistoryV2ResponseSchema,
  type OrganizedOrderHistoryV2Response,
  type OrderHistoryV2Request,
  type OrderHistoryV2Response,
} from "@/index";
import { TraderStateServerMessageSchema } from "@/api/traders/traderState";
import { compareOrderHistorySortKeys } from "@/api/orders";

describe("parent order history contract", () => {
  it("keeps flat and organized history data shapes separate", () => {
    const old = OrderHistoryV2ResponseSchema.parse({
      data: [],
      hasMore: false,
    });
    expect(old).not.toHaveProperty("parentOrders");
    expect(() =>
      OrderHistoryV2ResponseSchema.parse({
        data: { twapOrders: [], singleOrders: [] },
        hasMore: false,
      })
    ).toThrow();
    const organized = OrganizedOrderHistoryV2ResponseSchema.parse({
      data: { twapOrders: [], singleOrders: [] },
      hasMore: false,
    });
    expect(organized.data).toEqual({ twapOrders: [], singleOrders: [] });
    expect(() => OrganizedOrderHistoryV2ResponseSchema.parse(old)).toThrow();
  });
  it("retains nullable evidence and the terminal ordering key", () => {
    const parent = {
      parentOrderId: "twap:450000000:3:5",
      parentOrderType: "twap",
      marketSymbol: "SOL-PERP",
      traderPdaIndex: 0,
      traderSubaccountIndex: 0,
      side: null,
      isReduceOnly: null,
      nChildOrders: null,
      nDustOrders: null,
      cooldownSlots: null,
      status: "cancelled",
      requestedBaseQty: null,
      filledBaseQty: "0",
      filledQuoteQty: "0",
      placedAt: null,
      terminalAt: "2026-10-06T14:05:00Z",
      sortKey: "ohp1:1791295500000000:450000000:3:4:5:1",
      children: [],
    };
    expect(
      OrganizedOrderHistoryV2ResponseSchema.parse({
        data: { twapOrders: [parent], singleOrders: [] },
        hasMore: false,
      }).data.twapOrders
    ).toEqual([parent]);
  });
  it("compares numeric components including integers beyond Number precision", () => {
    expect(
      compareOrderHistorySortKeys(
        "ohp1:9007199254740993:2:0:0:0:0",
        "ohp1:9007199254740992:10:0:0:0:1"
      )
    ).toBe(1);
    expect(
      compareOrderHistorySortKeys("ohp1:1:2:0:0:0:0", "ohp1:1:10:0:0:0:0")
    ).toBe(-1);
    expect(
      compareOrderHistorySortKeys("ohp1:1:2:0:0:0:0", "ohp1:1:2:0:0:0:1")
    ).toBe(-1);
    expect(() =>
      compareOrderHistorySortKeys("invalid", "ohp1:1:2:0:0:0:0")
    ).toThrow();
  });
});

const parent = {
  parentOrderId: "twap:450000000:3:5",
  parentOrderType: "twap",
  marketSymbol: "SOL-PERP",
  traderPdaIndex: 0,
  traderSubaccountIndex: 0,
  side: null,
  isReduceOnly: null,
  nChildOrders: 10,
  nDustOrders: 1,
  cooldownSlots: 60,
  status: "cancelled",
  requestedBaseQty: null,
  filledBaseQty: "0",
  filledQuoteQty: "0",
  placedAt: null,
  terminalAt: "2026-10-06T14:05:00Z",
  sortKey: "ohp1:1791295500000000:450000000:3:4:5:1",
  children: [],
};
const ordinary = {
  userId: 1,
  traderId: 1,
  traderPdaIndex: 0,
  traderSubaccountIndex: 0,
  slot: 100,
  slotIndex: 0,
  instructionIndex: 0,
  eventIndex: 0,
  marketSymbol: "SOL-PERP",
  instructionType: "PlaceMarketOrder",
  orderType: "market",
  createdAt: "2026-10-06T14:05:00Z",
  price: "100",
  intended: {
    orderKind: "ImmediateOrCancel",
    side: "buy",
    priceInTicks: null,
    priceUsd: null,
    hasPriceLimit: false,
    baseLots: 1,
    baseQty: "1",
    quoteLotBudget: null,
    matchLimit: null,
    lastValidSlot: null,
    clientOrderId: null,
    orderFlags: 0,
    isReduceOnly: false,
    isStopLoss: false,
    isPostOnly: false,
    isTakeOnly: true,
    cancelExisting: false,
  },
  fillBeforePlacement: null,
  placedOrder: null,
  currentOrderState: null,
  isReduceOnly: false,
  isStopLoss: false,
  status: "filled",
};

describe("history transport boundaries", () => {
  afterEach(() => vi.unstubAllGlobals());
  it("opts into organized history without changing the default flat view", async () => {
    const urls: URL[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = new URL(String(input));
        urls.push(url);
        const data =
          url.searchParams.get("organize_twap_orders") === "true"
            ? { twapOrders: [], singleOrders: [] }
            : [ordinary];
        return new Response(JSON.stringify({ data, hasMore: false }), {
          headers: { "content-type": "application/json" },
        });
      })
    );
    const client = new PhoenixHttpClient({ apiUrl: "https://example.com" });
    const organized = await client.orders().getTraderOrderHistoryV2("trader", {
      organizeTwapOrders: true,
    });
    const flat = await client.orders().getTraderOrderHistoryV2("trader");
    expectTypeOf(organized).toEqualTypeOf<OrganizedOrderHistoryV2Response>();
    expectTypeOf(flat).toEqualTypeOf<OrderHistoryV2Response>();
    expect(urls[0]?.searchParams.get("organize_twap_orders")).toBe("true");
    expect(organized.data).toEqual({ twapOrders: [], singleOrders: [] });
    expect(urls[1]?.searchParams.has("organize_twap_orders")).toBe(false);
    expect(flat.data).toHaveLength(1);
  });
  it("retains REST child attribution without requiring an ID for suppression", () => {
    for (const metadata of [
      { parentOrderType: "twap" },
      { parentOrderType: "twap", parentOrderId: "twap:100:0:0" },
    ]) {
      const parsed = OrderHistoryV2ResponseSchema.parse({
        data: [{ ...ordinary, ...metadata }],
        hasMore: false,
      });
      expect(parsed.data[0]).toMatchObject(metadata);
    }
  });
  it("retains existing order items under organized parents and standalone orders", () => {
    const child = {
      ...ordinary,
      parentOrderId: parent.parentOrderId,
      parentOrderType: "twap",
      fillBeforePlacement: {
        totalBaseQtyFilled: "0.1",
        totalQuoteQtyFilled: "10",
        totalTakerFee: "0.01",
        totalNumFills: 2,
        numTrades: 2,
      },
    };
    const parsed = OrganizedOrderHistoryV2ResponseSchema.parse({
      data: {
        twapOrders: [{ ...parent, children: [child] }],
        singleOrders: [ordinary],
      },
      prevCursor: "newer",
      nextCursor: "older",
      hasMore: true,
    });
    expect(parsed.data.twapOrders[0]).toMatchObject({
      nChildOrders: 10,
      nDustOrders: 1,
      cooldownSlots: 60,
      children: [child],
    });
    expect(parsed.data.singleOrders[0]).toMatchObject(ordinary);
    expect(parsed).toMatchObject({
      prevCursor: "newer",
      nextCursor: "older",
      hasMore: true,
    });
    for (const invalid of [
      { ...child, intended: null },
      { ...child, parentOrderType: "unknown" },
    ]) {
      expect(() =>
        OrganizedOrderHistoryV2ResponseSchema.parse({
          data: {
            twapOrders: [{ ...parent, children: [invalid] }],
            singleOrders: [],
          },
          hasMore: false,
        })
      ).toThrow();
    }
  });
  it("retains exact WS child type with or without parent identity", () => {
    for (const parentOrderId of [undefined, "twap:100:0:0"]) {
      const row = {
        parentOrderId,
        parentOrderType: "twap",
        timestamp: 1,
        slot: 1,
        slotIndex: 0,
        instructionIndex: 0,
        eventIndex: 0,
        market: "SOL-PERP",
        instructionType: "PlaceMarketOrder",
        orderType: "market",
        status: "filled",
        size: "1",
        price: "100",
        filledSize: "1",
      };
      const message = {
        channel: "traderState",
        authority: "authority",
        traderPdaIndex: 0,
        slot: 1,
        messageType: "delta",
        deltas: [
          {
            subaccountIndex: 0,
            sequence: 1,
            collateral: "0",
            orderHistory: [row],
          },
        ],
      };
      expect(
        TraderStateServerMessageSchema.parse(message).deltas?.[0]
          ?.orderHistory[0]
      ).toMatchObject(row);
      expect(() =>
        TraderStateServerMessageSchema.parse({
          ...message,
          deltas: [
            {
              ...message.deltas[0],
              orderHistory: [{ ...row, parentOrderType: "unknown" }],
            },
          ],
        })
      ).toThrow();
    }
  });
  it("rejects invalid parent quantities and sort key boundaries", () => {
    for (const quantity of ["NaN", "-1", "1e3", ""]) {
      expect(() =>
        OrganizedOrderHistoryV2ResponseSchema.parse({
          data: {
            twapOrders: [{ ...parent, filledBaseQty: quantity }],
            singleOrders: [],
          },
          hasMore: false,
        })
      ).toThrow();
    }
    for (const key of [
      "ohp1:9223372036854775808:1:0:0:0:1",
      "ohp1:1:1:2147483648:0:0:1",
      "ohp1:1:1:0:0:0:2",
    ]) {
      expect(() => compareOrderHistorySortKeys(key, parent.sortKey)).toThrow();
    }
    expect(() =>
      OrderHistoryV2ResponseSchema.parse({
        data: [{ ...ordinary, parentOrderType: "scale" }],
        hasMore: false,
      })
    ).toThrow();
  });
  it("validates parent timestamps before mapping them to display values", () => {
    for (const invalid of [
      "not-a-date",
      "2026-10-06",
      "2026-13-06T14:05:00Z",
    ]) {
      for (const field of ["terminalAt", "placedAt"]) {
        expect(() =>
          OrganizedOrderHistoryV2ResponseSchema.parse({
            data: {
              twapOrders: [{ ...parent, [field]: invalid }],
              singleOrders: [],
            },
            hasMore: false,
          })
        ).toThrow();
      }
    }
    const known = {
      ...parent,
      placedAt: "2026-10-06T14:00:00.123456+00:00",
      terminalAt: "2026-10-06T14:05:00.123456Z",
    };
    expect(
      OrganizedOrderHistoryV2ResponseSchema.parse({
        data: { twapOrders: [known], singleOrders: [] },
        hasMore: false,
      }).data.twapOrders[0]
    ).toEqual(known);
  });
  it("rejects invalid original schedule counts and cooldown evidence", () => {
    for (const field of ["nChildOrders", "nDustOrders", "cooldownSlots"]) {
      for (const invalid of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
        expect(() =>
          OrganizedOrderHistoryV2ResponseSchema.parse({
            data: {
              twapOrders: [{ ...parent, [field]: invalid }],
              singleOrders: [],
            },
            hasMore: false,
          })
        ).toThrow();
      }
    }
  });
  it("validates the response against the requested view for literal and runtime flags", async () => {
    const organized = {
      data: { twapOrders: [], singleOrders: [] },
      hasMore: false,
    };
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ data: [], hasMore: false }))
        )
        .mockResolvedValueOnce(new Response(JSON.stringify(organized)))
        .mockResolvedValueOnce(new Response(JSON.stringify(organized)))
    );
    const orders = new PhoenixHttpClient({
      apiUrl: "https://example.com",
    }).orders();
    await expect(
      orders.getTraderOrderHistoryV2("trader", { organizeTwapOrders: true })
    ).rejects.toThrow();
    await expect(
      orders.getTraderOrderHistoryV2("trader", { organizeTwapOrders: false })
    ).rejects.toThrow();
    const request: OrderHistoryV2Request<boolean> = {
      organizeTwapOrders: true,
    };
    const getWithOptionalRequest = (request?: OrderHistoryV2Request<boolean>) =>
      orders.getTraderOrderHistoryV2("trader", request);
    expectTypeOf(getWithOptionalRequest).returns.toEqualTypeOf<
      Promise<OrderHistoryV2Response | OrganizedOrderHistoryV2Response>
    >();
    const result = await orders.getTraderOrderHistoryV2("trader", request);
    expectTypeOf(result).toEqualTypeOf<
      OrderHistoryV2Response | OrganizedOrderHistoryV2Response
    >();
    expect(result.data).toEqual(organized.data);
  });
  it("preserves legacy request typing and serializes explicit false with filters", async () => {
    const urls: URL[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        urls.push(new URL(String(input)));
        return new Response(JSON.stringify({ data: [], hasMore: false }), {
          headers: { "content-type": "application/json" },
        });
      })
    );
    const client = new PhoenixHttpClient({ baseUrl: "https://example.com" });
    const request: OrderHistoryV2Request = {
      limit: 2,
      marketSymbol: "SOL-PERP",
      organizeTwapOrders: false,
    };
    const filtered = await client
      .orders()
      .getTraderOrderHistoryV2("trader", request);
    const unfiltered = await client
      .orders()
      .getTraderOrderHistoryV2("trader", { limit: 2 });
    expectTypeOf(filtered).toEqualTypeOf<OrderHistoryV2Response>();
    expect(filtered.data).toEqual([]);
    expect(unfiltered.data).toEqual([]);
    expect(urls[0]?.searchParams.get("organize_twap_orders")).toBe("false");
    expect(urls[0]?.searchParams.get("market_symbol")).toBe("SOL-PERP");
    expect(urls[1]?.searchParams.has("organize_twap_orders")).toBe(false);
  });
});
