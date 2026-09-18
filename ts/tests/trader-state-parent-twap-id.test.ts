import { describe, expect, it } from "vitest";

import { TraderStateServerMessageSchema } from "@/api/traders/traderState";

const PARENT_TWAP_ID = "future-parent-twap-format";

const buildMessage = (parentTwapId?: string | null) => ({
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
      tradeHistory: [
        {
          signature: "signature",
          fillId: null,
          parentTwapId,
          timestamp: 1,
          slot: 1,
          slotIndex: 0,
          instructionIndex: 0,
          eventIndex: 0,
          market: "SOL-PERP",
          instructionType: "PlaceMarketOrder",
          tradeType: "market",
          baseQtyBefore: "0",
          baseQtyAfter: "1",
          size: "1",
          liquidity: "taker",
          price: "100",
          fee: "0.1",
          realizedPnl: "0",
        },
      ],
    },
  ],
});

describe("trader-state parent TWAP ID", () => {
  it.each([
    PARENT_TWAP_ID,
    "11111111111111111111111111111111:42",
    null,
    undefined,
  ])("parses %s", (parentTwapId) => {
    const parsed = TraderStateServerMessageSchema.parse(
      buildMessage(parentTwapId)
    );

    expect(parsed.deltas?.[0]?.tradeHistory[0]?.parentTwapId).toBe(
      parentTwapId ?? null
    );
  });

  it("rejects empty parent TWAP IDs", () => {
    expect(() =>
      TraderStateServerMessageSchema.parse(buildMessage(""))
    ).toThrow();
  });
});
