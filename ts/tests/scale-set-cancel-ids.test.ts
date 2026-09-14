import {
  cancelIdsForScaleSet,
  getCancelIdEncoder,
  type BuildCancelOrdersByIdIxResolvedInput,
  type ScaleSetCancelableOrderRow,
  type TraderStateMarketLimitOrderRow,
} from "@/index";
import { describe, expect, it } from "vitest";

const row = (
  overrides: Partial<ScaleSetCancelableOrderRow>
): ScaleSetCancelableOrderRow => ({
  priceTicks: "100",
  orderSequenceNumber: "1",
  ...overrides,
});

describe("cancelIdsForScaleSet", () => {
  it("returns [] for empty rows", () => {
    expect(cancelIdsForScaleSet([], 3)).toEqual([]);
  });

  it("returns [] when no rows match the target set", () => {
    const rows = [row({ scaleSetId: 3 }), row({ scaleSetId: 7 })];
    expect(cancelIdsForScaleSet(rows, 9)).toEqual([]);
  });

  it("filters mixed rows to only the target set, preserving input order", () => {
    const rows = [
      row({ priceTicks: "10", orderSequenceNumber: "1", scaleSetId: 3 }),
      row({ priceTicks: "20", orderSequenceNumber: "2", scaleSetId: 7 }),
      row({ priceTicks: "30", orderSequenceNumber: "3", scaleSetId: 0 }),
      row({ priceTicks: "40", orderSequenceNumber: "4" }),
      row({ priceTicks: "50", orderSequenceNumber: "5", scaleSetId: 3 }),
    ];

    const result = cancelIdsForScaleSet(rows, 3);

    expect(result).toHaveLength(2);
    expect(result[0]?.orderId.priceInTicks).toBe(10n);
    expect(result[0]?.orderId.orderSequenceNumber).toBe(1n);
    expect(result[1]?.orderId.priceInTicks).toBe(50n);
    expect(result[1]?.orderId.orderSequenceNumber).toBe(5n);
  });

  it("preserves exact string->bigint precision beyond Number.MAX_SAFE_INTEGER", () => {
    const rows = [
      row({
        priceTicks: "18446744073709551615", // near u64::MAX
        orderSequenceNumber: "9007199254740993", // MAX_SAFE_INTEGER + 2
        scaleSetId: 5,
      }),
    ];

    const result = cancelIdsForScaleSet(rows, 5);

    expect(result[0]?.orderId.priceInTicks).toBe(18446744073709551615n);
    expect(result[0]?.orderId.orderSequenceNumber).toBe(9007199254740993n);
  });

  it.each([0, -1, 256, 1.5, Number.NaN])(
    "throws for invalid target %s",
    (invalid) => {
      expect(() => cancelIdsForScaleSet([], invalid)).toThrow(
        `scaleSetId must be an integer in 1..=255; got ${invalid}`
      );
    }
  );

  it("accepts boundary targets 1 and 255", () => {
    expect(() => cancelIdsForScaleSet([], 1)).not.toThrow();
    expect(() => cancelIdsForScaleSet([], 255)).not.toThrow();
  });

  it("throws for a negative orderSequenceNumber", () => {
    const rows = [row({ orderSequenceNumber: "-5", scaleSetId: 3 })];
    expect(() => cancelIdsForScaleSet(rows, 3)).toThrow();
  });

  it("throws for an orderSequenceNumber beyond u64::MAX", () => {
    const rows = [
      row({
        orderSequenceNumber: "18446744073709551616", // 2^64
        scaleSetId: 3,
      }),
    ];
    expect(() => cancelIdsForScaleSet(rows, 3)).toThrow();
  });

  it.each(["", "0x10", " 5", "007"])(
    "throws for a malformed orderSequenceNumber %j",
    (malformed) => {
      const rows = [row({ orderSequenceNumber: malformed, scaleSetId: 3 })];
      expect(() => cancelIdsForScaleSet(rows, 3)).toThrow();
    }
  );

  it.each(["", "0x1f4", " 5", "00"])(
    "throws for a malformed priceTicks %j",
    (malformed) => {
      const rows = [row({ priceTicks: malformed, scaleSetId: 3 })];
      expect(() => cancelIdsForScaleSet(rows, 3)).toThrow();
    }
  );

  it("returns all 128 ids of a two-sided 64+64 set uncapped — splitting across the builder's 100-id limit is the caller's job", () => {
    const bidRows = Array.from({ length: 64 }, (_, i) =>
      row({
        priceTicks: `${100 + i}`,
        orderSequenceNumber: `${i + 1}`,
        scaleSetId: 6,
      })
    );
    const askRows = Array.from({ length: 64 }, (_, i) =>
      row({
        priceTicks: `${200 + i}`,
        orderSequenceNumber: `${1000 + i}`,
        scaleSetId: 6,
      })
    );
    const rows = [...bidRows, ...askRows];

    const result = cancelIdsForScaleSet(rows, 6);

    expect(result).toHaveLength(128);
    expect(result[0]?.orderId.orderSequenceNumber).toBe(1n);
    expect(result[63]?.orderId.orderSequenceNumber).toBe(64n);
    expect(result[64]?.orderId.orderSequenceNumber).toBe(1000n);
    expect(result[127]?.orderId.orderSequenceNumber).toBe(1063n);
  });

  it("sets nodePointer to null and round-trips through getCancelIdEncoder as the 0 sentinel", () => {
    const rows = [row({ scaleSetId: 4 })];
    const result = cancelIdsForScaleSet(rows, 4);

    expect(result[0]?.nodePointer).toBeNull();

    const encoded = getCancelIdEncoder().encode(result[0]!);
    const bytes = Array.from(encoded);
    // Leading u32 (little-endian) is the nodePointer sentinel: 0.
    expect(bytes.slice(0, 4)).toEqual([0, 0, 0, 0]);
  });

  it("accepts a real TraderStateMarketLimitOrderRow literal (structural compatibility)", () => {
    const realRow: TraderStateMarketLimitOrderRow = {
      // Ask-side sequence numbers are bit-complemented on-chain, so real
      // values sit near u64::MAX — beyond Number.MAX_SAFE_INTEGER.
      orderSequenceNumber: "18446744073709551610",
      side: "ask",
      orderType: "limit",
      priceTicks: "123456",
      priceUsd: "1.23",
      sizeRemainingLots: "10",
      initialSizeLots: "10",
      reduceOnly: false,
      scaleSetId: 9,
      status: "active",
    };

    const result = cancelIdsForScaleSet([realRow], 9);

    expect(result).toEqual([
      {
        nodePointer: null,
        orderId: {
          priceInTicks: 123456n,
          orderSequenceNumber: 18446744073709551610n,
        },
      },
    ]);
  });

  it("is assignable to buildCancelOrdersByIdIxResolved's orderIds param (compile-time)", () => {
    const rows = [row({ scaleSetId: 4 })];
    const orderIds = cancelIdsForScaleSet(rows, 4);
    const _check: BuildCancelOrdersByIdIxResolvedInput["orderIds"] = orderIds;
    expect(_check).toBe(orderIds);
  });
});
