import { describe, expect, it, vi } from "vitest";

import { V1CollateralClient } from "@/api/collateral";
import { SpotAssetConfigSchema } from "@/api/collateral/types";
import type { HttpTransport, RequestOptions } from "@/http";
import swapResponse from "./mocks/swap-transaction.json";

const transportWithResponse = (
  responseBody: unknown,
  endpoints: string[]
): HttpTransport => ({
  fetch: async (_method, endpoint) => {
    endpoints.push(endpoint);
    return new Response(JSON.stringify(responseBody), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  },
});

describe("spot collateral metadata", () => {
  const config = {
    isActive: true,
    perpAssetIndex: null,
    maxPerTraderBalance: "1000000000",
    maxGlobalBalance: "10000000000",
    currGlobalBalance: "0",
    minMarginDiscountBps: 0,
    maxMarginDiscountBps: 5000,
    maxLiquidationDiscountBps: 5000,
    minLiquidationSlippageBps: 0,
    maxLiquidationSize: "1",
  };

  it("preserves optional oracle-only pricing metadata without losing integer precision", () => {
    expect(SpotAssetConfigSchema.parse(config).oraclePrice).toBeUndefined();
    expect(
      SpotAssetConfigSchema.parse({
        ...config,
        oraclePrice: {
          ticks: "9007199254740993",
          tickSize: "1000",
          baseLotDecimals: 3,
        },
      }).oraclePrice
    ).toEqual({
      ticks: 9007199254740993n,
      tickSize: 1000n,
      baseLotDecimals: 3,
    });
  });
});

describe("V1CollateralClient.getUserCollateralTotals", () => {
  it("parses per-asset totals from the user endpoint", async () => {
    const body = {
      totalDeposited: 101,
      totalWithdrawn: 64,
      assets: [
        {
          assetIndex: 4_294_901_760,
          symbol: "SOL",
          decimals: 9,
          deposited: { amount: 2_000_000_000, value: 100 },
          withdrawn: { amount: 1_100_000_000, value: 64 },
        },
        {
          assetIndex: 4_294_967_295,
          symbol: "USDC",
          decimals: 6,
          deposited: { amount: 1_000_000, value: 1 },
          withdrawn: { amount: 0, value: 0 },
        },
      ],
    };
    const endpoints: string[] = [];
    const client = new V1CollateralClient(
      transportWithResponse(body, endpoints)
    );

    await expect(client.getUserCollateralTotals("user/1")).resolves.toEqual(
      body
    );
    expect(endpoints).toEqual(["/v1/users/user%2F1/collateral-totals"]);
  });

  it("rejects a response missing an asset flow", async () => {
    const client = new V1CollateralClient(
      transportWithResponse(
        {
          totalDeposited: 1,
          totalWithdrawn: 0,
          assets: [
            {
              assetIndex: 4_294_967_295,
              symbol: "USDC",
              decimals: 6,
              deposited: { amount: 1_000_000, value: 1 },
            },
          ],
        },
        []
      )
    );

    await expect(client.getUserCollateralTotals("user")).rejects.toThrow();
  });
});

describe("V1CollateralClient collateral history v2", () => {
  const event = {
    slot: 10,
    slotIndex: 1,
    eventIndex: 2,
    traderPdaIndex: 0,
    traderSubaccountIndex: 0,
    assetIndex: 4_294_901_760,
    symbol: "SOL",
    category: "withdrawal",
    amount: -250_000_000,
    balanceAfter: 1_750_000_000,
    excess: 0,
    timestamp: "2026-09-01T00:00:00Z",
  };

  it("parses mixed events and forwards pagination params", async () => {
    const requests: { endpoint: string; options?: RequestOptions }[] = [];
    const http: HttpTransport = {
      fetch: async (_method, endpoint, options) => {
        requests.push({ endpoint, options });
        return new Response(
          JSON.stringify({
            data: [event, { ...event, eventIndex: 3, signature: "sig" }],
            nextCursor: "older",
            hasMore: true,
          }),
          { status: 200, headers: { "content-type": "application/json" } }
        );
      },
    };
    const client = new V1CollateralClient(http);

    const response = await client.getTraderCollateralHistoryV2("authority", {
      pdaIndex: 1,
      limit: 50,
      nextCursor: "cursor",
    });

    expect(requests).toEqual([
      {
        endpoint: "/v1/trader/authority/collateral-history-v2",
        options: { params: { pdaIndex: 1, limit: 50, nextCursor: "cursor" } },
      },
    ]);
    const { timestamp: _timestamp, ...rest } = event;
    expect(response).toEqual({
      data: [
        { ...rest, timestamp: Date.parse(event.timestamp) },
        {
          ...rest,
          eventIndex: 3,
          signature: "sig",
          timestamp: Date.parse(event.timestamp),
        },
      ],
      nextCursor: "older",
      prevCursor: null,
      hasMore: true,
    });
    expect(response.data[0]).not.toHaveProperty("signature");
  });

  it("uses the user and trader PDA paths", async () => {
    const endpoints: string[] = [];
    const client = new V1CollateralClient(
      transportWithResponse({ data: [], hasMore: false }, endpoints)
    );

    await client.getUserCollateralHistoryV2("user", { limit: 1 });
    await client.getTraderPdaCollateralHistoryV2("pda", { limit: 1 });

    expect(endpoints).toEqual([
      "/v1/users/user/collateral-history-v2",
      "/v1/traders/pda/collateral-history-v2",
    ]);
  });

  it("rejects an unknown category", async () => {
    const client = new V1CollateralClient(
      transportWithResponse(
        { data: [{ ...event, category: "rebate" }], hasMore: false },
        []
      )
    );

    await expect(
      client.getUserCollateralHistoryV2("user", { limit: 1 })
    ).rejects.toThrow();
  });
});

describe("V1CollateralClient native amount precision", () => {
  const rawTransport = (body: string): HttpTransport => ({
    fetch: async () =>
      new Response(body, {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
  });
  const unsafe = "9007199254740993";

  it("rejects totals amounts beyond the safe integer range", async () => {
    const client = new V1CollateralClient(
      rawTransport(
        `{"totalDeposited":1,"totalWithdrawn":0,"assets":[{"assetIndex":4294901760,"symbol":"SOL","decimals":9,"deposited":{"amount":${unsafe},"value":1},"withdrawn":{"amount":0,"value":0}}]}`
      )
    );

    await expect(client.getUserCollateralTotals("user")).rejects.toThrow();
  });

  it.each(["amount", "balanceAfter", "excess"])(
    "rejects a history %s beyond the safe integer range",
    async (field) => {
      const event: Record<string, unknown> = {
        slot: 1,
        slotIndex: 0,
        eventIndex: 0,
        traderPdaIndex: 0,
        traderSubaccountIndex: 0,
        assetIndex: 4_294_901_760,
        symbol: "SOL",
        category: "deposit",
        amount: 1,
        balanceAfter: 1,
        excess: 0,
        timestamp: 0,
      };
      const body = JSON.stringify({ data: [event], hasMore: false }).replace(
        `"${field}":${String(event[field])}`,
        `"${field}":${unsafe}`
      );
      const client = new V1CollateralClient(rawTransport(body));

      await expect(
        client.getUserCollateralHistoryV2("user", { limit: 1 })
      ).rejects.toThrow();
    }
  );
});

describe("V1CollateralClient.buildSwapTransaction", () => {
  it("forwards the isolated account, asset, and signing options without changing the attested response", async () => {
    const response = swapResponse;
    const requests: unknown[] = [];
    const client = new V1CollateralClient({
      fetch: async (method, endpoint, _options, body) => {
        requests.push({ method, endpoint, body });
        return Response.json(response);
      },
    });
    const request = {
      authority: "owner",
      positionAuthority: "delegate",
      assetIn: { spot: "mint" },
      assetOut: "quote" as const,
      amountAtoms: 1_000_000_000,
      traderPdaIndex: 2,
      traderSubaccountIndex: 3,
      feePayer: "sponsor",
      slippageBps: 50,
      computeUnitPriceMicroLamports: 1000,
      transactionVersion: 0 as const,
    };

    await expect(client.buildSwapTransaction(request)).resolves.toEqual(
      response
    );
    const defaultRequest = {
      authority: "owner",
      assetIn: "sol" as const,
      assetOut: "quote" as const,
      quantity: 0.5,
    };
    await expect(client.buildSwapTransaction(defaultRequest)).resolves.toEqual(
      response
    );
    expect(requests).toEqual([
      { method: "POST", endpoint: "/v1/ix/swap", body: request },
      { method: "POST", endpoint: "/v1/ix/swap", body: defaultRequest },
    ]);
  });

  it("rejects unsafe amounts and ambiguous sizes before sending", async () => {
    const fetch = vi.fn<HttpTransport["fetch"]>();
    const client = new V1CollateralClient({ fetch });
    const request = {
      authority: "owner",
      assetIn: "sol" as const,
      assetOut: "quote" as const,
    };
    for (const size of [
      {},
      { amountAtoms: 1, quantity: 1 },
      { amountAtoms: Number.MAX_SAFE_INTEGER + 1 },
    ]) {
      await expect(
        client.buildSwapTransaction({ ...request, ...size })
      ).rejects.toThrow();
    }
    expect(fetch).not.toHaveBeenCalled();
  });

  it("rejects incomplete quotes and propagates API failures", async () => {
    const request = {
      authority: "owner",
      assetIn: "sol" as const,
      assetOut: "quote" as const,
      quantity: 0.5,
    };
    for (const response of [
      { transaction: "bytes" },
      { ...swapResponse, minAmountOutAtoms: Number.MAX_SAFE_INTEGER + 1 },
    ]) {
      const client = new V1CollateralClient({
        fetch: async () => Response.json(response),
      });
      await expect(client.buildSwapTransaction(request)).rejects.toThrow();
    }
    const unavailable = new V1CollateralClient({
      fetch: async () =>
        Response.json({ error: "swap_unavailable" }, { status: 503 }),
    });
    await expect(
      unavailable.buildSwapTransaction(request)
    ).rejects.toMatchObject({ status: 503 });
  });
});
