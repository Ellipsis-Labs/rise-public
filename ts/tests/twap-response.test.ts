import { describe, expect, it, vi } from "vitest";

import { TwapSnapshotSchema } from "@/index";
import { V1TradersClient } from "@/api/traders/client";
import type { HttpTransport } from "@/http/transport";
import snapshot from "./mocks/twap-snapshot.json";

describe("TWAP HTTP response", () => {
  it("preserves dust parameters through the trader read client", async () => {
    const fetch = vi.fn<HttpTransport["fetch"]>(async () =>
      Response.json(snapshot)
    );
    const client = new V1TradersClient({ fetch });
    const params = { traderPdaIndex: 0, traderSubaccountIndex: 0, assetId: 0 };

    await expect(client.getTwap(snapshot.authority, params)).resolves.toEqual(
      snapshot
    );
    expect(fetch).toHaveBeenCalledWith(
      "GET",
      `/v1/trader/${snapshot.authority}/twap`,
      { params },
      undefined
    );
  });

  it.each([
    { nDustOrders: 0, dustOrderSize: 0 },
    { nDustOrders: 0, dustOrderSize: 7 },
    { nDustOrders: 2, dustOrderSize: 7 },
    { nDustOrders: 2, dustOrderSize: null },
  ])("preserves zero, legacy and multiple-dust sizes: %j", (dust) => {
    const payload = {
      ...snapshot,
      accounts: [
        {
          ...snapshot.accounts[0],
          order: { ...snapshot.accounts[0].order, ...dust },
        },
      ],
    };
    expect(TwapSnapshotSchema.parse(payload)).toEqual(payload);
  });

  it("accepts snapshots from servers without dust fields", () => {
    const payload = structuredClone(snapshot);
    const legacyOrder = payload.accounts[0].order;
    expect(legacyOrder).toBeDefined();
    if (!legacyOrder) throw new Error("Expected active TWAP fixture");
    Reflect.deleteProperty(legacyOrder, "nDustOrders");
    Reflect.deleteProperty(legacyOrder, "dustOrderSize");
    const order = TwapSnapshotSchema.parse(payload).accounts[0].order;
    expect(order?.nDustOrders).toBeUndefined();
    expect(order?.dustOrderSize).toBeUndefined();
  });
});
