import { zstdCompressSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { base64 } from "@scure/base";
import { createExchangeAdapter } from "@/ws/adapters/exchange";
import type {
  SubscriptionMessage,
  WsChannelRegistration,
  WsClient,
} from "@/ws/types";

type RegisteredExchangeSubscription = {
  subMsg: SubscriptionMessage;
  onMessage: (data: unknown) => void;
};

const buildRiskActionPriceValidityRules = (
  fill: "ignore" | "require" | "forbid" = "ignore"
): ("ignore" | "require" | "forbid")[][][] =>
  Array.from({ length: 8 }, () =>
    Array.from({ length: 4 }, () => Array.from({ length: 8 }, () => fill))
  );

const createFakeWs = (): {
  ws: WsClient;
  subscriptions: Map<string, RegisteredExchangeSubscription>;
} => {
  const subscriptions = new Map<string, RegisteredExchangeSubscription>();

  return {
    subscriptions,
    ws: {
      subscribe(key, subMsg, onMessage) {
        subscriptions.set(key, { subMsg, onMessage });
      },
      unsubscribe(key) {
        subscriptions.delete(key);
      },
      registerChannel(_channel: string, _registration: WsChannelRegistration) {
        return () => {};
      },
      close() {},
      onServerError() {
        return () => {};
      },
    },
  };
};

const buildSnapshotPayload = () => ({
  version: 1,
  sequenceNumber: "9",
  slot: "42",
  slotIndex: 3,
  reason: "snapshot",
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
    canonicalMint: "canonical-mint",
    usdcMint: "usdc-mint",
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
      assetId: 7,
      marketStatus: "active",
      marketPubkey: "market-pubkey",
      splinePubkey: "spline-pubkey",
      tickSize: 100,
      baseLotsDecimals: 0,
      takerFee: 0.003,
      makerFee: -0.0005,
      leverageTiers: [],
      riskFactors: {
        maintenance: 67,
        backstop: 50,
        highRisk: 20,
        upnl: 100,
        upnlForWithdrawals: 100,
        cancelOrder: 70,
      },
      fundingConfig: {
        fundingIntervalSeconds: 3600,
        fundingPeriodSeconds: 86400,
        maxFundingRatePerInterval: 5000,
      },
      openInterestCapBaseLots: "10000",
      maxLiquidationSizeBaseLots: "500",
      isolatedOnly: false,
      markPriceParameters: {
        emaPeriodSlots: "375",
        emaDiffRadius: "250",
        bookPriceRadius: "500",
        commoditiesAfterHoursRadius: "0",
        adjustedExchangeSpotPriceWeight: "2000",
        bookPriceWeight: "4000",
        exchangePerpPriceWeight: "4000",
        spotPriceStaleThreshold: "120",
        bookPriceStaleThreshold: "15",
        perpPriceStaleThreshold: "15",
        riskActionPriceValidityRules: buildRiskActionPriceValidityRules(),
        oracleDivergenceRadius: 500,
        minOracleResponses: 1,
      },
    },
  ],
});

describe("exchange adapter", () => {
  it("includes the requested encoding in the subscribe params", () => {
    const { ws, subscriptions } = createFakeWs();
    const adapter = createExchangeAdapter(ws, { encoding: "json" });

    const abort = new AbortController();
    adapter(abort.signal);

    const [subscription] = [...subscriptions.values()];
    expect(subscription?.subMsg).toEqual({
      type: "subscribe",
      subscription: {
        channel: "exchange",
        encoding: "json",
      },
    });
  });

  it("decodes encoded snapshot frames into normalized snapshot messages", async () => {
    const { ws, subscriptions } = createFakeWs();
    const adapter = createExchangeAdapter(ws);
    const abort = new AbortController();
    const iterator = adapter(abort.signal)[Symbol.asyncIterator]();
    const nextMessage = iterator.next();
    const [subscription] = [...subscriptions.values()];

    const payload = buildSnapshotPayload();
    const compressed = zstdCompressSync(Buffer.from(JSON.stringify(payload)));
    subscription?.onMessage({
      channel: "exchange",
      messageType: "encodedSnapshot",
      version: 1,
      sequenceNumber: "9",
      slot: "42",
      slotIndex: 3,
      reason: "snapshot",
      encoding: "base64+zstd",
      payload: base64.encode(compressed),
    });

    const result = await nextMessage;
    if (result.done) {
      throw new Error("expected decoded exchange snapshot");
    }

    expect(result.value.messageType).toBe("snapshot");
    if (result.value.messageType !== "snapshot") {
      throw new Error("expected snapshot message");
    }
    expect(result.value.sequenceNumber).toBe(9n);
    expect(result.value.slot).toBe(42n);
    expect(result.value.exchange.globalConfig).toBe("global-config");
    expect(result.value.markets[0]?.symbol).toBe("SOL-PERP");
    expect(result.value.markets[0]?.openInterestCapBaseLots).toBe(10000n);

    abort.abort();
  });

  it("normalizes snake_case delta op fields from the exchange stream", async () => {
    const { ws, subscriptions } = createFakeWs();
    const adapter = createExchangeAdapter(ws);
    const abort = new AbortController();
    const iterator = adapter(abort.signal)[Symbol.asyncIterator]();
    const nextMessage = iterator.next();
    const [subscription] = [...subscriptions.values()];

    subscription?.onMessage({
      channel: "exchange",
      messageType: "delta",
      version: 1,
      sequenceNumber: "10",
      slot: "43",
      slotIndex: 4,
      ops: [
        {
          kind: "exchangeStatusChanged",
          previous_bits: 128,
          new_bits: 129,
          previous_features: ["initialized"],
          new_features: ["initialized", "active"],
          enabled_features: ["active"],
          disabled_features: [],
          active: true,
          gated: false,
        },
        {
          kind: "marketStatusChanged",
          symbol: "XAU",
          previous_market_status: "active",
          new_market_status: "postOnly",
        },
        {
          kind: "marketClosed",
          symbol: "XAU",
          previous_market_status: "postOnly",
          finalized_mark_price: "123",
        },
        {
          kind: "marketTombstoned",
          symbol: "XAU",
          previous_market_status: "closed",
          final_sequence_number: "11",
          final_trade_sequence_number: "12",
          final_order_sequence_number: "13",
        },
        {
          kind: "marketDeleted",
          symbol: "XAU",
          asset_id: 7,
        },
      ],
    });

    const result = await nextMessage;
    if (result.done) {
      throw new Error("expected normalized exchange delta");
    }

    expect(result.value.messageType).toBe("delta");
    if (result.value.messageType !== "delta") {
      throw new Error("expected delta message");
    }

    expect(result.value.ops[0]).toMatchObject({
      kind: "exchangeStatusChanged",
      previousBits: 128,
      newBits: 129,
      previousFeatures: ["initialized"],
      newFeatures: ["initialized", "active"],
      enabledFeatures: ["active"],
      disabledFeatures: [],
      active: true,
      gated: false,
    });
    expect(result.value.ops[1]).toMatchObject({
      kind: "marketStatusChanged",
      symbol: "XAU",
      previousMarketStatus: "active",
      newMarketStatus: "postOnly",
    });
    expect(result.value.ops[2]).toMatchObject({
      kind: "marketClosed",
      symbol: "XAU",
      previousMarketStatus: "postOnly",
      finalizedMarkPrice: 123n,
    });
    expect(result.value.ops[3]).toMatchObject({
      kind: "marketTombstoned",
      symbol: "XAU",
      previousMarketStatus: "closed",
      finalSequenceNumber: 11n,
      finalTradeSequenceNumber: 12n,
      finalOrderSequenceNumber: 13n,
    });
    expect(result.value.ops[4]).toMatchObject({
      kind: "marketDeleted",
      symbol: "XAU",
      assetId: 7,
    });

    abort.abort();
  });
});
