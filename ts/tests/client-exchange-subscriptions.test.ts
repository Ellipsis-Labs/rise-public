import { afterEach, describe, expect, it, vi } from "vitest";
import { createPhoenixClient } from "@/index";
import type { ExchangeSnapshotView } from "@/api/exchange/types";

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  readonly sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((evt: { data: string }) => void) | null = null;
  onclose: ((evt: { code: number; reason?: string }) => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(
    public url: string,
    public protocol?: string | string[]
  ) {
    MockWebSocket.instances.push(this);
    queueMicrotask(() => this.onopen?.());
  }

  send(payload: string) {
    this.sent.push(payload);
  }

  close() {}
}

const buildSnapshot = (): ExchangeSnapshotView => ({
  version: 1,
  slot: 1n,
  slotIndex: 0,
  exchange: {
    programId: "program-id",
    globalConfig: "global-config",
    currentAuthorities: {
      rootAuthority: "root",
      riskAuthority: "risk",
      marketAuthority: "market",
      oracleAuthority: "oracle",
      adlAuthority: "adl",
      cancelAuthority: "cancel",
      backstopAuthority: "backstop",
    },
    canonicalMint: "So11111111111111111111111111111111111111112",
    usdcMint: "EPjFWdd5AufqSSqeM2qAEyFnjAu1pzsVbW8UVzA73NA",
    globalVault: "global-vault",
    perpAssetMap: "perp-asset-map",
    globalTraderIndex: ["gti-0"],
    activeTraderBuffer: ["atb-0"],
    withdrawQueue: "withdraw-queue",
    exchangeStatusBits: 129,
    exchangeStatusFeatures: ["initialized", "active"],
    active: true,
    gated: false,
  },
  markets: [
    {
      symbol: "SOL-PERP",
      assetId: 1,
      marketStatus: "active",
      marketPubkey: "sol-market",
      splinePubkey: "sol-spline",
      tickSize: 100,
      baseLotsDecimals: 3,
      takerFee: 0.0005,
      makerFee: -0.0001,
      leverageTiers: [],
      riskFactors: {
        maintenance: 5,
        backstop: 8,
        highRisk: 12,
        upnl: 90,
        upnlForWithdrawals: 80,
        cancelOrder: 2.5,
      },
      fundingConfig: {
        fundingIntervalSeconds: 3600,
        fundingPeriodSeconds: 28800,
        maxFundingRatePerInterval: 2500,
      },
      openInterestCapBaseLots: 5_000n,
      maxLiquidationSizeBaseLots: 250n,
      isolatedOnly: false,
      markPriceParameters: {
        emaPeriodSlots: 1n,
        emaDiffRadius: 1n,
        bookPriceRadius: 1n,
        commoditiesAfterHoursRadius: 0n,
        adjustedExchangeSpotPriceWeight: 1n,
        bookPriceWeight: 1n,
        exchangePerpPriceWeight: 1n,
        spotPriceStaleThreshold: 1n,
        bookPriceStaleThreshold: 1n,
        perpPriceStaleThreshold: 1n,
        riskActionPriceValidityRules: Array.from({ length: 8 }, () =>
          Array.from({ length: 4 }, () =>
            Array.from({ length: 8 }, () => "ignore" as const)
          )
        ),
        oracleDivergenceRadius: 1,
        minOracleResponses: 1,
      },
      commodityMetadata: null,
    },
  ],
});

const stringifyWithBigints = (value: unknown): string =>
  JSON.stringify(value, (_, innerValue) =>
    typeof innerValue === "bigint" ? innerValue.toString() : innerValue
  );

const waitUntil = async (
  predicate: () => boolean,
  message: string
): Promise<void> => {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error(message);
};

const sentSubscriptions = (): Array<{ channel: string; symbol?: string }> =>
  MockWebSocket.instances.flatMap((socket) =>
    socket.sent.map((payload) => {
      const message = JSON.parse(payload) as {
        subscription: { channel: string; symbol?: string };
      };
      return message.subscription;
    })
  );

afterEach(() => {
  vi.unstubAllGlobals();
  MockWebSocket.instances.length = 0;
});

describe("createPhoenixClient exchange subscriptions", () => {
  it("bootstraps exchange metadata from HTTP without opening a websocket by default", async () => {
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        return new Response(stringifyWithBigints(buildSnapshot()), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      })
    );

    const client = createPhoenixClient({
      apiUrl: "https://example.com",
    });

    await client.exchange.ready();

    expect(MockWebSocket.instances).toHaveLength(0);

    client.dispose();
  });

  it("subscribes to exchange deltas when exchangeMetadata.stream is enabled", async () => {
    vi.stubGlobal("WebSocket", MockWebSocket as unknown as typeof WebSocket);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        return new Response(stringifyWithBigints(buildSnapshot()), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      })
    );

    const client = createPhoenixClient({
      apiUrl: "https://example.com",
      exchangeMetadata: {
        stream: true,
      },
    });

    await client.exchange.ready();

    await waitUntil(
      () =>
        sentSubscriptions().some(
          (subscription) => subscription.channel === "exchange"
        ),
      "exchange subscription was not started"
    );

    expect(MockWebSocket.instances).toHaveLength(1);
    expect(
      sentSubscriptions().some(
        (subscription) => subscription.channel === "exchange"
      )
    ).toBe(true);

    client.dispose();
  });

  it("rejects conflicting exchange stream settings", () => {
    expect(() =>
      createPhoenixClient({
        exchangeMetadata: {
          stream: true,
          api: {
            liveUpdates: "none",
          },
        },
      })
    ).toThrow(
      "exchangeMetadata.stream conflicts with exchangeMetadata.api.liveUpdates"
    );
  });

  it("memoizes client-owned market data and orderbook managers", () => {
    const client = createPhoenixClient({
      apiUrl: "https://example.com",
      ws: false,
    });

    expect(client.marketData()).toBe(client.marketData());
    expect(client.orderbooks()).toBe(client.orderbooks());

    client.dispose();
  });
});
