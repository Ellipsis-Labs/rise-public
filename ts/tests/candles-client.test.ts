import { describe, expect, it } from "vitest";

import { V1CandlesClient } from "@/api/candles";
import type { HttpTransport, RequestOptions } from "@/http";

interface RecordedRequest {
  endpoint: string;
  options?: RequestOptions;
}

const transportWithResponse = (
  responseBody: unknown,
  requests: RecordedRequest[]
): HttpTransport => ({
  fetch: async (_method, endpoint, options) => {
    requests.push({ endpoint, options });
    return new Response(JSON.stringify(responseBody), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  },
});

describe("V1CandlesClient", () => {
  it("keeps legacy source selection and time windows on the legacy endpoint", async () => {
    const requests: RecordedRequest[] = [];
    const client = new V1CandlesClient(
      transportWithResponse(
        [
          {
            time: 60_000,
            open: 10,
            high: 12,
            low: 9,
            close: 11,
            volume: 5,
            tradeCount: 3,
          },
        ],
        requests
      )
    );

    const candles = await client.getCandles("SOL", {
      timeframe: "1m",
      startTime: 0,
      endTime: 60_000,
      limit: 1,
    });

    expect(requests).toEqual([
      {
        endpoint: "/v1/candles/SOL",
        options: {
          params: {
            timeframe: "1m",
            startTime: 0,
            endTime: 60_000,
            limit: 1,
          },
        },
      },
    ]);
    expect(candles).toEqual([
      {
        time: 60_000,
        open: 10,
        high: 12,
        low: 9,
        close: 11,
        volume: 5,
        tradeCount: 3,
      },
    ]);
  });

  it("exposes candles v2 pagination and partial-bar controls", async () => {
    const requests: RecordedRequest[] = [];
    const response = {
      symbol: "SOL",
      timeframe: "1m",
      from: 0,
      to: 120_000,
      bars: [],
      page: { hasMore: true, nextCursor: "next-page" },
    };
    const client = new V1CandlesClient(
      transportWithResponse(response, requests)
    );

    await expect(
      client.getCandlesV2("SOL", {
        timeframe: "1m",
        from: 0,
        to: 120_000,
        limit: 10_000,
        includePartial: true,
      })
    ).resolves.toEqual(response);

    expect(requests[0]).toEqual({
      endpoint: "/v1/candles_v2/SOL",
      options: {
        params: {
          timeframe: "1m",
          from: 0,
          to: 120_000,
          limit: 10_000,
          includePartial: true,
        },
      },
    });
  });

  it("forwards legacy limits unchanged and preserves the full server response", async () => {
    const requests: RecordedRequest[] = [];
    const client = new V1CandlesClient(
      transportWithResponse(
        Array.from({ length: 2_500 }, (_, time) => ({
          time,
          open: 1,
          high: 1,
          low: 1,
          close: 1,
          volume: 0,
          tradeCount: 0,
        })),
        requests
      )
    );

    const candles = await client.getCandles("SOL", {
      timeframe: "1m",
      limit: 10_000,
    });

    expect(requests[0]).toEqual({
      endpoint: "/v1/candles/SOL",
      options: {
        params: {
          timeframe: "1m",
          limit: 10_000,
        },
      },
    });
    expect(candles).toHaveLength(2_500);
  });

  it("keeps open-ended legacy windows authoritative on the server", async () => {
    const requests: RecordedRequest[] = [];
    const client = new V1CandlesClient(transportWithResponse([], requests));

    await client.getCandles("SOL", { timeframe: "1m" });

    expect(requests[0]).toEqual({
      endpoint: "/v1/candles/SOL",
      options: { params: { timeframe: "1m" } },
    });
  });

  it("defaults candles v2 initial requests to one thousand bars", async () => {
    const requests: RecordedRequest[] = [];
    const client = new V1CandlesClient(
      transportWithResponse(
        {
          symbol: "SOL",
          timeframe: "1m",
          from: 0,
          to: 60_000,
          bars: [],
          page: { hasMore: false },
        },
        requests
      )
    );

    await client.getCandlesV2("SOL", {
      timeframe: "1m",
      from: 0,
      to: 60_000,
    });

    expect(requests[0]).toEqual({
      endpoint: "/v1/candles_v2/SOL",
      options: {
        params: { timeframe: "1m", from: 0, to: 60_000, limit: 1_000 },
      },
    });
  });

  it("sends only cursor-compatible fields on continuation requests", async () => {
    const requests: RecordedRequest[] = [];
    const client = new V1CandlesClient(
      transportWithResponse(
        {
          symbol: "SOL",
          timeframe: "1m",
          from: 0,
          to: 120_000,
          bars: [],
          page: { hasMore: false },
        },
        requests
      )
    );

    await client.getCandlesV2("SOL", {
      cursor: "opaque-cursor",
    });

    expect(requests[0]).toEqual({
      endpoint: "/v1/candles_v2/SOL",
      options: {
        params: { cursor: "opaque-cursor" },
      },
    });
  });

  it("leaves legacy timeframe validation to the legacy server", async () => {
    const requests: RecordedRequest[] = [];
    const client = new V1CandlesClient(transportWithResponse([], requests));

    await client.getCandles("SOL", { timeframe: "custom", limit: 10 });

    expect(requests[0]).toEqual({
      endpoint: "/v1/candles/SOL",
      options: { params: { timeframe: "custom", limit: 10 } },
    });
  });

  it("retains explicit legacy external-source selection", async () => {
    const requests: RecordedRequest[] = [];
    const client = new V1CandlesClient(transportWithResponse([], requests));

    await client.getCandles("SOL", {
      timeframe: "1m",
      limit: 2_500,
      enableExternalSource: false,
    });

    expect(requests[0]).toEqual({
      endpoint: "/v1/candles/SOL",
      options: {
        params: {
          timeframe: "1m",
          limit: 2_500,
          enableExternalSource: false,
        },
      },
    });
  });

  it("rejects cursor requests mixed with initial-window fields", async () => {
    const requests: RecordedRequest[] = [];
    const client = new V1CandlesClient(transportWithResponse({}, requests));

    await expect(
      client.getCandlesV2("SOL", {
        cursor: "opaque-cursor",
        timeframe: "1m",
        from: 0,
        to: 60_000,
      } as never)
    ).rejects.toThrow(
      "candles_v2 cursor requests must not include timeframe, from, to, limit, or includePartial"
    );
    expect(requests).toEqual([]);
  });
});
