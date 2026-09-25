import { describe, expect, it } from "vitest";

import {
  PlaceTwapOrderRequestSchema,
  type PlaceTwapOrderRequest,
} from "@/index";

const request: PlaceTwapOrderRequest = {
  authority: "11111111111111111111111111111112",
  symbol: "SOL-PERP",
  side: "buy",
  cooldownSlots: 10,
  childOrders: 3,
  childOrderParams: { numBaseLots: 34, dustOrderSize: 2 },
};

describe("TWAP HTTP request", () => {
  it.each([undefined, null, 0, 2])("preserves dust count %s", (nDustOrders) => {
    const payload = {
      ...request,
      ...(nDustOrders === undefined ? {} : { nDustOrders }),
    };
    const parsed = PlaceTwapOrderRequestSchema.parse(payload);

    expect(parsed).toEqual(payload);
    expect(JSON.parse(JSON.stringify(parsed))).toEqual(payload);
  });

  it("preserves isolated collateral parameters under the API field name", () => {
    const payload: PlaceTwapOrderRequest = {
      ...request,
      nDustOrders: 2,
      marginType: "isolated",
      isolatedTWAPOrderParams: {
        childOrderCollateralQuoteLotsToTransfer: 10,
        transferAmount: 100,
        transferSpotCollateralAmounts: { SOL: 1_000 },
        allowCrossAndIsolatedForAsset: true,
      },
    };

    expect(PlaceTwapOrderRequestSchema.parse(payload)).toEqual(payload);
  });

  it.each([-1, 1.5])("rejects invalid dust count %s", (nDustOrders) => {
    expect(
      PlaceTwapOrderRequestSchema.safeParse({ ...request, nDustOrders }).success
    ).toBe(false);
  });
});
