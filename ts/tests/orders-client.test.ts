import { afterEach, describe, expect, it, vi } from "vitest";

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
