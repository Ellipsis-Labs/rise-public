import { describe, expect, it } from "vitest";

import { NotificationItemSchema } from "@/api/notifications/types";

const wire = {
  source: "event",
  id: 7,
  slot: 100,
  slotIndex: 1,
  instructionIndex: 0,
  eventIndex: 2,
  recipientIndex: 0,
  createdAt: "2026-08-21T00:00:00Z",
  acked: false,
  notificationType: "spot_collateral_liquidation",
  data: {
    slot: 100,
    slotIndex: 1,
    timestamp: 1700000000,
    liquidator: "Liq1111111111111111111111111111111111111111",
    liquidatedTrader: "Trader11111111111111111111111111111111111111",
    assetIndex: 3,
    liquidationSize: "5000",
    overCapExcess: 0,
    quoteLotsDeposited: 4500,
    oracleNotional: 4800,
    liquidationDiscountBps: 625,
  },
  details: {
    type: "spotCollateralLiquidation",
    assetIndex: 3,
    liquidationSize: 5000,
    quoteLotsDeposited: 4500,
    oracleNotional: 4800,
    liquidationDiscountBps: 625,
  },
};

describe("spot collateral liquidation notification", () => {
  it("parses as a known typed item", () => {
    const parsed = NotificationItemSchema.parse(wire);
    expect(parsed).toMatchObject({
      source: "event",
      notificationType: "spot_collateral_liquidation",
      data: {
        assetIndex: 3,
        liquidationSize: 5000n,
        liquidatedTrader: wire.data.liquidatedTrader,
      },
      details: {
        type: "spotCollateralLiquidation",
        liquidationDiscountBps: 625,
      },
    });
  });

  it("rejects a malformed payload instead of falling back to unknown", () => {
    const malformed = { ...wire, data: { ...wire.data, assetIndex: "three" } };
    expect(() => NotificationItemSchema.parse(malformed)).toThrow();
  });
});
