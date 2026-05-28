import { describe, expect, it } from "vitest";

import { V1CandlesClient } from "@/api/candles";
import { V1CollateralClient } from "@/api/collateral";
import { V1ExchangeClient } from "@/api/exchange";
import { V1FundingClient } from "@/api/funding";
import { V1InviteClient } from "@/api/invite";
import { V1MarketsClient } from "@/api/markets";
import { V1OrdersClient } from "@/api/orders";
import { V1TradersClient } from "@/api/traders";
import { V1TradesClient } from "@/api/trades";
import { PhoenixHttpError } from "@/errors";
import type { HttpTransport, QueryParams, RequestOptions } from "@/http";

interface RequestRecord {
  method: string;
  endpoint: string;
  params?: QueryParams;
  body?: unknown;
  auth?: RequestOptions["auth"];
}

const buildRiskActionPriceValidityRules = (
  fill: "ignore" | "require" | "forbid" = "ignore"
): ("ignore" | "require" | "forbid")[][][] =>
  Array.from({ length: 8 }, () =>
    Array.from({ length: 4 }, () => Array.from({ length: 8 }, () => fill))
  );

const EXCHANGE_CONFIG_RESPONSE = {
  keys: {
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
    pendingAuthorities: {
      rootAuthority: "root",
      riskAuthority: "risk",
      marketAuthority: "market",
      oracleAuthority: "oracle",
      adlAuthority: "adl",
      cancelAuthority: "cancel",
      backstopAuthority: "backstop",
    },
    canonicalMint: "mint",
    globalVault: "vault",
    perpAssetMap: "perp-asset-map",
    globalTraderIndex: ["global-trader-index-0", "global-trader-index-1"],
    activeTraderBuffer: ["active-trader-buffer-0", "active-trader-buffer-1"],
    withdrawQueue: "withdraw-queue",
  },
  markets: [],
};

const EXCHANGE_MARKET_RESPONSE = {
  symbol: "SOL",
  assetId: 1,
  marketStatus: "active",
  marketPubkey: "market-pubkey",
  splinePubkey: "spline-pubkey",
  tickSize: 1,
  baseLotsDecimals: 0,
  takerFee: 0,
  makerFee: 0,
  leverageTiers: [],
  riskFactors: {
    maintenance: 1,
    backstop: 1,
    highRisk: 1,
    upnl: 1,
    upnlForWithdrawals: 1,
    cancelOrder: 1,
  },
  fundingIntervalSeconds: 1,
  fundingPeriodSeconds: 1,
  maxFundingRatePerInterval: 1,
  maxFundingRatePerIntervalPercentage: 1,
  openInterestCapBaseLots: "1",
  maxLiquidationSizeBaseLots: "1",
  isolatedOnly: false,
};

const EXCHANGE_SNAPSHOT_RESPONSE = {
  version: 1,
  slot: "42",
  slotIndex: 7,
  exchange: {
    programId: "program-id",
    globalConfig: "global-config",
    currentAuthorities: EXCHANGE_CONFIG_RESPONSE.keys.currentAuthorities,
    canonicalMint: "mint",
    usdcMint: "usdc-mint",
    globalVault: "vault",
    perpAssetMap: "perp-asset-map",
    globalTraderIndex: ["global-trader-index-0", "global-trader-index-1"],
    activeTraderBuffer: ["active-trader-buffer-0", "active-trader-buffer-1"],
    withdrawQueue: "withdraw-queue",
    exchangeStatusBits: 129,
    exchangeStatusFeatures: ["initialized", "active"],
    active: true,
    gated: false,
  },
  markets: [
    {
      ...EXCHANGE_MARKET_RESPONSE,
      leverageTiers: [
        {
          maxLeverage: 20,
          maxSizeBaseLots: "1000",
          limitOrderRiskFactor: 250,
        },
      ],
      fundingConfig: {
        fundingIntervalSeconds: 1,
        fundingPeriodSeconds: 1,
        maxFundingRatePerInterval: 1,
      },
      openInterestCapBaseLots: "1",
      maxLiquidationSizeBaseLots: "1",
      markPriceParameters: {
        emaPeriodSlots: "1",
        emaDiffRadius: "1",
        bookPriceRadius: "1",
        commoditiesAfterHoursRadius: "0",
        adjustedExchangeSpotPriceWeight: "1",
        bookPriceWeight: "1",
        exchangePerpPriceWeight: "1",
        spotPriceStaleThreshold: "1",
        bookPriceStaleThreshold: "1",
        perpPriceStaleThreshold: "1",
        riskActionPriceValidityRules: buildRiskActionPriceValidityRules(),
        oracleDivergenceRadius: 1,
        minOracleResponses: 1,
      },
    },
  ],
};

const TRADER_CAPABILITIES = {
  placeLimitOrder: { immediate: true, viaColdActivation: false },
  placeMarketOrder: { immediate: true, viaColdActivation: false },
  riskIncreasingTrade: { immediate: true, viaColdActivation: false },
  riskReducingTrade: { immediate: true, viaColdActivation: false },
  depositCollateral: { immediate: true, viaColdActivation: false },
  withdrawCollateral: { immediate: true, viaColdActivation: false },
};

const TOKEN_AMOUNT = {
  value: 0,
  decimals: 0,
  ui: "0",
};

const TRADER_VIEW_RESPONSE = {
  flags: 0,
  state: "active",
  capabilities: TRADER_CAPABILITIES,
  slot: 1,
  slotIndex: 2,
  traderKey: "trader-pubkey",
  traderPdaIndex: 0,
  traderSubaccountIndex: 0,
  authority: "authority",
  collateralBalance: TOKEN_AMOUNT,
  effectiveCollateral: TOKEN_AMOUNT,
  effectiveCollateralForWithdrawals: TOKEN_AMOUNT,
  unrealizedPnl: TOKEN_AMOUNT,
  discountedUnrealizedPnl: TOKEN_AMOUNT,
  unsettledFundingOwed: TOKEN_AMOUNT,
  accumulatedFunding: TOKEN_AMOUNT,
  portfolioValue: TOKEN_AMOUNT,
  maintenanceMargin: TOKEN_AMOUNT,
  cancelMargin: TOKEN_AMOUNT,
  initialMargin: TOKEN_AMOUNT,
  initialMarginForWithdrawals: TOKEN_AMOUNT,
  riskState: "healthy",
  riskTier: "safe",
  positions: [],
  limitOrders: {},
  maxPositions: 0,
  lastDepositSlot: 0,
  isInActiveTraders: false,
  makerFeeOverrideMultiplier: 1,
  takerFeeOverrideMultiplier: 1,
};

const createTransport = (
  responses: Map<string, unknown>,
  records: RequestRecord[]
): HttpTransport => ({
  fetch: async (
    method: string,
    endpoint: string,
    options?: RequestOptions,
    body?: unknown
  ) => {
    const record: RequestRecord = {
      method,
      endpoint,
      params: options?.params,
      body,
    };
    if (options?.auth !== undefined) {
      record.auth = options.auth;
    }
    records.push(record);

    return new Response(JSON.stringify(responses.get(endpoint)), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  },
});

describe("public client route mapping", () => {
  it("uses the public exchange and market routes", async () => {
    const records: RequestRecord[] = [];
    const transport = createTransport(
      new Map<string, unknown>([
        ["/exchange", EXCHANGE_CONFIG_RESPONSE],
        ["/v1/exchange/snapshot", EXCHANGE_SNAPSHOT_RESPONSE],
        ["/exchange/market/SOL", EXCHANGE_MARKET_RESPONSE],
        ["/exchange/status", { active: true, gated: false }],
        ["/exchange/keys", EXCHANGE_CONFIG_RESPONSE.keys],
        ["/exchange/markets", [EXCHANGE_MARKET_RESPONSE]],
        [
          "/v1/market/SOL/stats",
          {
            market_id: 1,
            symbol: "SOL",
            timeframe: null,
            stats: [],
          },
        ],
        [
          "/v1/market/next-commodity-market-transition",
          {
            market: "XAU",
            loadedAt: "2026-04-24T12:00:00Z",
            utcNextTransition: "2026-04-24T21:00:00Z",
            nextMarketState: "afterHours",
            currentState: "open",
          },
        ],
      ]),
      records
    );

    const exchange = new V1ExchangeClient(transport);
    const markets = new V1MarketsClient(transport);

    await exchange.getExchange();
    await exchange.getSnapshot();
    await exchange.getMarket("SOL");
    await exchange.getStatus();
    await exchange.getKeys();
    await exchange.getMarkets();
    await markets.getMarkets();
    await markets.getMarket("SOL");
    await markets.getMarketStatsHistory("SOL", { limit: 25 });
    await markets.getNextCommodityMarketTransition();

    expect(records).toEqual([
      {
        method: "GET",
        endpoint: "/exchange",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/exchange/snapshot",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/exchange/market/SOL",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/exchange/status",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/exchange/keys",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/exchange/markets",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/exchange/markets",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/exchange/market/SOL",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/market/SOL/stats",
        params: { limit: 25 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/market/next-commodity-market-transition",
        params: undefined,
        body: undefined,
      },
    ]);
  });

  it("uses the public trader, history, and candle routes", async () => {
    const records: RequestRecord[] = [];
    const transport = createTransport(
      new Map<string, unknown>([
        ["/v1/view/trader/trader-pubkey", TRADER_VIEW_RESPONSE],
        ["/v1/view/trader-capabilities", { capabilities: [] }],
        [
          "/trader/authority/state",
          {
            slot: 1,
            slotIndex: 2,
            authority: "authority",
            pdaIndex: 0,
            traders: [TRADER_VIEW_RESPONSE],
          },
        ],
        [
          "/v1/trader/state/authority",
          {
            authority: "authority",
            traderPdaIndex: 0,
            slot: 1,
            slotIndex: 2,
            snapshot: {
              version: 1,
              capabilities: {
                flags: 1,
                state: "active",
                capabilities: {
                  placeLimitOrder: {
                    immediate: true,
                    viaColdActivation: false,
                  },
                  placeMarketOrder: {
                    immediate: true,
                    viaColdActivation: false,
                  },
                  riskIncreasingTrade: {
                    immediate: true,
                    viaColdActivation: false,
                  },
                  riskReducingTrade: {
                    immediate: true,
                    viaColdActivation: false,
                  },
                  depositCollateral: {
                    immediate: true,
                    viaColdActivation: false,
                  },
                  withdrawCollateral: {
                    immediate: true,
                    viaColdActivation: false,
                  },
                },
              },
              makerFeeOverrideMultiplier: 1,
              takerFeeOverrideMultiplier: 1,
              subaccounts: [],
            },
          },
        ],
        [
          "/trader/authority/pnl",
          [
            {
              timestamp: 1,
              startTime: 1,
              endTime: 2,
              cumulativePnl: 3,
              unrealizedPnl: 4,
              cumulativeFundingPayment: 5,
              cumulativeTakerFee: 6,
            },
          ],
        ],
        [
          "/trader/authority/collateral-history",
          { data: [], nextCursor: null, prevCursor: null, hasMore: false },
        ],
        [
          "/v1/users/user-pubkey/collateral-history",
          { data: [], nextCursor: null, prevCursor: null, hasMore: false },
        ],
        [
          "/trader/authority/funding-history",
          { events: [], nextCursor: null, prevCursor: null, hasMore: false },
        ],
        [
          "/v1/users/user-pubkey/funding-hourly",
          { events: [], nextCursor: null, prevCursor: null, hasMore: false },
        ],
        [
          "/trader/authority/order-history",
          { data: [], nextCursor: null, prevCursor: null, hasMore: false },
        ],
        [
          "/v1/traders/trader-pubkey/orders_v2",
          { data: [], nextCursor: null, prevCursor: null, hasMore: false },
        ],
        [
          "/trader/authority/trades-history",
          { data: [], nextCursor: null, prevCursor: null, hasMore: false },
        ],
        [
          "/market/SOL/fills",
          {
            data: [
              {
                marketSymbol: "SOL",
                baseQty: "1",
                quoteQty: "10",
                price: "10",
                timestamp: "123",
                transactionSignature: "fill-sig",
                instructionType: "Fill",
              },
            ],
            nextCursor: null,
            prevCursor: null,
            hasMore: false,
          },
        ],
        [
          "/v1/traders/trader-pubkey/trades_v2",
          { data: [], nextCursor: null, prevCursor: null, hasMore: false },
        ],
        [
          "/v1/traders/trader-pubkey/portfolio-values",
          [
            {
              timestamp: 1,
              startTime: 1,
              endTime: 2,
              value: 3,
              positions: null,
            },
          ],
        ],
        [
          "/v1/traders/trader-pubkey/pnl",
          [
            {
              timestamp: 1,
              startTime: 1,
              endTime: 2,
              cumulativePnl: 3,
              unrealizedPnl: 4,
              cumulativeFundingPayment: 5,
              cumulativeTakerFee: 6,
            },
          ],
        ],
        [
          "/v1/candles/SOL",
          [
            {
              time: 1,
              open: 1,
              high: 2,
              low: 1,
              close: 2,
              volume: 3,
              tradeCount: 4,
            },
          ],
        ],
        [
          "/v1/funding/SOL/rates",
          {
            marketId: 1,
            symbol: "SOL",
            rates: [],
          },
        ],
        [
          "/v1/funding/overview",
          {
            series: [],
          },
        ],
        [
          "/v1/invite/validate",
          {
            success: true,
            message: "ok",
            whitelisted: true,
          },
        ],
        [
          "/v1/invite/check/wallet-abc",
          {
            whitelisted: false,
            whitelisted_at: null,
            invite_code_used: null,
          },
        ],
        [
          "/v1/invite/activate",
          {
            trader_pda: "trader-pda-123",
          },
        ],
        [
          "/v1/referral/activate",
          {
            trader_pda: "trader-pda-123",
          },
        ],
      ]),
      records
    );

    const traders = new V1TradersClient(transport);
    const collateral = new V1CollateralClient(transport);
    const funding = new V1FundingClient(transport);
    const invite = new V1InviteClient(transport);
    const orders = new V1OrdersClient(transport);
    const trades = new V1TradesClient(transport);
    const candles = new V1CandlesClient(transport);

    const traderView = await traders.getTrader("trader-pubkey");
    await traders.getTraderCapabilities();
    await traders.getTraderState("authority", { pdaIndex: 0 });
    await traders.getTraderStateSnapshot("authority", { traderPdaIndex: 0 });
    await traders.getTraderPnl("authority", { resolution: "1h", limit: 10 });
    await collateral.getTraderCollateralHistory("authority", {
      pdaIndex: 0,
      limit: 10,
    });
    await collateral.getUserCollateralHistory("user-pubkey", {
      limit: 5,
      nextCursor: "next-cursor",
    });
    await funding.getTraderFundingHistory("authority", {
      pdaIndex: 0,
      limit: 10,
      symbol: "SOL",
    });
    await funding.getUserFundingHourly("user-pubkey", {
      symbol: "SOL",
      limit: 5,
      cursor: "funding-cursor",
      traderPdaIndex: 2,
    });
    await funding.getFundingRateHistory("SOL", {
      startTime: 0,
      endTime: 0,
      limit: 25,
    });
    await funding.getFundingOverview({
      startTime: 0,
      endTime: 0,
      perMarketLimit: 10,
    });
    await orders.getTraderOrderHistory("authority", {
      traderPdaIndex: 0,
      limit: 10,
      marketSymbol: "SOL",
      orderStatus: "open",
    });
    await orders.getTraderOrderHistoryV2("trader-pubkey", {
      marketSymbol: "SOL",
      limit: 5,
      cursor: "orders-v2-cursor",
      startTime: new Date("2025-01-01T00:00:00.000Z"),
      endTime: new Date("2025-01-02T00:00:00.000Z"),
    });
    await trades.getTraderTradesHistory("authority", {
      pdaIndex: 0,
      limit: 10,
      marketSymbol: "SOL",
    });
    const marketFills = await trades.getMarketFills("SOL", {
      limit: 3,
      cursor: "fills-cursor",
      startTime: 1_700_000_000_000,
      endTime: 1_700_000_060_000,
    });
    await trades.getTraderTradeHistoryV2("trader-pubkey", {
      marketSymbol: "SOL",
      limit: 5,
      cursor: "trades-v2-cursor",
      privyId: "privy-id",
    });
    await traders.getTraderPortfolioValues("trader-pubkey", {
      resolution: "1h",
      limit: 10,
    });
    await traders.getTraderPnlValues("trader-pubkey", {
      resolution: "1h",
      limit: 10,
    });
    await candles.getCandles("SOL", { timeframe: "1m", limit: 100 });
    await invite.validateInvite({
      code: "invite-123",
      wallet_address: "wallet-abc",
      transaction_signature: "sig-123",
    });
    await invite.checkWallet("wallet-abc");
    await invite.activateInvite({
      authority: "authority-abc",
      code: "invite-123",
    });
    await invite.activateReferral({
      authority: "authority-abc",
      referral_code: "ref-789",
    });

    expect(marketFills.data[0]?.timestamp).toBe(123);
    expect(traderView.verifyCapabilities()).toBe(true);

    expect(records).toEqual([
      {
        method: "GET",
        endpoint: "/v1/view/trader/trader-pubkey",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/view/trader-capabilities",
        params: undefined,
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/trader/authority/state",
        params: { pdaIndex: 0 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/trader/state/authority",
        params: { traderPdaIndex: 0 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/trader/authority/pnl",
        params: { resolution: "1h", limit: 10 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/trader/authority/collateral-history",
        params: { pdaIndex: 0, limit: 10 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/users/user-pubkey/collateral-history",
        params: { limit: 5, nextCursor: "next-cursor" },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/trader/authority/funding-history",
        params: { traderPdaIndex: 0, symbol: "SOL", limit: 10 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/users/user-pubkey/funding-hourly",
        params: {
          symbol: "SOL",
          limit: 5,
          cursor: "funding-cursor",
          traderPdaIndex: 2,
        },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/funding/SOL/rates",
        params: {
          startTime: 0,
          endTime: 0,
          limit: 25,
        },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/funding/overview",
        params: {
          startTime: 0,
          endTime: 0,
          perMarketLimit: 10,
        },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/trader/authority/order-history",
        params: {
          traderPdaIndex: 0,
          marketSymbol: "SOL",
          limit: 10,
          orderStatus: "open",
        },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/traders/trader-pubkey/orders_v2",
        params: {
          market_symbol: "SOL",
          limit: 5,
          cursor: "orders-v2-cursor",
          start_time: "2025-01-01T00:00:00.000Z",
          end_time: "2025-01-02T00:00:00.000Z",
        },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/trader/authority/trades-history",
        params: { pdaIndex: 0, marketSymbol: "SOL", limit: 10 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/market/SOL/fills",
        params: {
          limit: 3,
          cursor: "fills-cursor",
          startTime: 1_700_000_000_000,
          endTime: 1_700_000_060_000,
        },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/traders/trader-pubkey/trades_v2",
        params: {
          market_symbol: "SOL",
          limit: 5,
          cursor: "trades-v2-cursor",
          privy_id: "privy-id",
        },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/traders/trader-pubkey/portfolio-values",
        params: { resolution: "1h", limit: 10 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/traders/trader-pubkey/pnl",
        params: { resolution: "1h", limit: 10 },
        body: undefined,
      },
      {
        method: "GET",
        endpoint: "/v1/candles/SOL",
        params: { timeframe: "1m", limit: 100 },
        body: undefined,
      },
      {
        method: "POST",
        endpoint: "/v1/invite/validate",
        params: undefined,
        body: {
          code: "invite-123",
          wallet_address: "wallet-abc",
          transaction_signature: "sig-123",
        },
      },
      {
        method: "GET",
        endpoint: "/v1/invite/check/wallet-abc",
        params: undefined,
        body: undefined,
      },
      {
        method: "POST",
        endpoint: "/v1/invite/activate",
        params: undefined,
        body: {
          authority: "authority-abc",
          code: "invite-123",
        },
      },
      {
        method: "POST",
        endpoint: "/v1/referral/activate",
        params: undefined,
        body: {
          authority: "authority-abc",
          referral_code: "ref-789",
        },
        auth: "required",
      },
    ]);
  });

  it("explains that referral activation requires user auth", async () => {
    const invite = new V1InviteClient({
      fetch: async (method, endpoint, options, body) => {
        expect(method).toBe("POST");
        expect(endpoint).toBe("/v1/referral/activate");
        expect(options?.auth).toBe("required");
        expect(body).toEqual({
          authority: "authority-abc",
          referral_code: "ref-789",
        });

        return new Response(
          JSON.stringify({
            error: "missing_access_token",
            message: "missing access token",
          }),
          {
            status: 401,
            headers: { "content-type": "application/json" },
          }
        );
      },
    });

    let error: unknown;
    try {
      await invite.activateReferral({
        authority: "authority-abc",
        referral_code: "ref-789",
      });
    } catch (caught) {
      error = caught;
    }

    expect(error).toBeInstanceOf(PhoenixHttpError);
    const httpError = error as PhoenixHttpError;
    expect(httpError.status).toBe(401);
    expect(httpError.code).toBe("missing_access_token");
    expect(httpError.message).toContain(
      "Referral activation requires an authenticated user session for the authority wallet"
    );
  });

  it("uses the server-built instruction and invite routes", async () => {
    const records: RequestRecord[] = [];
    const transport = createTransport(
      new Map<string, unknown>([
        [
          "/v1/ix/place-isolated-limit-order",
          [
            {
              data: [1, 2, 3],
              keys: [
                {
                  pubkey: "11111111111111111111111111111111",
                  isSigner: false,
                  isWritable: false,
                },
              ],
              programId: "11111111111111111111111111111111",
            },
          ],
        ],
        [
          "/v1/ix/place-isolated-market-order",
          [
            {
              data: [4, 5, 6],
              keys: [
                {
                  pubkey: "11111111111111111111111111111111",
                  isSigner: true,
                  isWritable: true,
                },
              ],
              programId: "11111111111111111111111111111111",
            },
          ],
        ],
        [
          "/v1/ix/place-isolated-limit-order-with-conditionals",
          [
            {
              data: [16, 17, 18],
              keys: [
                {
                  pubkey: "11111111111111111111111111111111",
                  isSigner: true,
                  isWritable: true,
                },
              ],
              programId: "11111111111111111111111111111111",
            },
          ],
        ],
        [
          "/v1/ix/place-stop-loss-order",
          [
            {
              data: [19, 20, 21],
              keys: [
                {
                  pubkey: "11111111111111111111111111111111",
                  isSigner: true,
                  isWritable: true,
                },
              ],
              programId: "11111111111111111111111111111111",
            },
          ],
        ],
        [
          "/v1/ix/cancel-stop-loss-order",
          [
            {
              data: [22, 23, 24],
              keys: [
                {
                  pubkey: "11111111111111111111111111111111",
                  isSigner: true,
                  isWritable: false,
                },
              ],
              programId: "11111111111111111111111111111111",
            },
          ],
        ],
        [
          "/v1/ix/place-attached-conditional-order",
          [
            {
              data: [25, 26, 27],
              keys: [
                {
                  pubkey: "11111111111111111111111111111111",
                  isSigner: false,
                  isWritable: true,
                },
              ],
              programId: "11111111111111111111111111111111",
            },
          ],
        ],
        [
          "/v1/ix/place-position-conditional-order",
          [
            {
              data: [28, 29, 30],
              keys: [
                {
                  pubkey: "11111111111111111111111111111111",
                  isSigner: false,
                  isWritable: false,
                },
              ],
              programId: "11111111111111111111111111111111",
            },
          ],
        ],
        [
          "/v1/ix/place-isolated-limit-order-enhanced",
          {
            instructions: [
              {
                data: [7, 8, 9],
                keys: [
                  {
                    pubkey: "11111111111111111111111111111111",
                    isSigner: false,
                    isWritable: true,
                  },
                ],
                programId: "11111111111111111111111111111111",
              },
            ],
            estimatedLiquidationPriceUsd: 74.54,
          },
        ],
        [
          "/v1/ix/place-isolated-market-order-enhanced",
          {
            instructions: [
              {
                data: [10, 11, 12],
                keys: [
                  {
                    pubkey: "11111111111111111111111111111111",
                    isSigner: true,
                    isWritable: false,
                  },
                ],
                programId: "11111111111111111111111111111111",
              },
            ],
            estimatedLiquidationPriceUsd: 73.3,
          },
        ],
        [
          "/v1/ix/cancel-conditional-order",
          [
            {
              data: [13, 14, 15],
              keys: [
                {
                  pubkey: "11111111111111111111111111111111",
                  isSigner: true,
                  isWritable: true,
                },
              ],
              programId: "11111111111111111111111111111111",
            },
          ],
        ],
        ["/v1/invite/activate", { trader_pda: "trader-pda" }],
      ]),
      records
    );

    const orders = new V1OrdersClient(transport);
    const invite = new V1InviteClient(transport);

    const limitIxs = await orders.placeIsolatedLimitOrder({
      authority: "authority",
      symbol: "SOL",
      side: "buy",
      numBaseLots: 1,
    });
    const marketIxs = await orders.placeIsolatedMarketOrder({
      authority: "authority",
      symbol: "SOL",
      side: "sell",
      numBaseLots: 2,
    });
    const limitWithConditionalsIxs =
      await orders.placeIsolatedLimitOrderWithConditionals({
        authority: "authority",
        symbol: "SOL",
        side: "buy",
        price: 100,
        quantity: 1,
        greaterTrigger: {
          side: "sell",
          triggerPrice: 120,
          executionPrice: 119,
          orderKind: "limit",
        },
      });
    const stopLossIxs = await orders.placeStopLossOrder({
      authority: "authority",
      traderPdaIndex: 0,
      symbol: "SOL",
      side: "sell",
      stopLossTriggerPrice: 90,
      stopLossExecutionPrice: 89,
      orderKind: "ioc",
    });
    const cancelStopLossIxs = await orders.cancelStopLossOrder({
      authority: "authority",
      traderPdaIndex: 0,
      symbol: "SOL",
      executionDirection: "less_than",
    });
    const attachedConditionalIxs = await orders.placeAttachedConditionalOrder({
      authority: "authority",
      traderPdaIndex: 0,
      symbol: "SOL",
      orderSequenceNumber: "42",
      orderPriceInTicks: 1234,
      greaterTrigger: {
        side: "sell",
        triggerPriceInTicks: 1500,
      },
    });
    const positionConditionalIxs = await orders.placePositionConditionalOrder({
      authority: "authority",
      traderPdaIndex: 0,
      symbol: "SOL",
      sizePercent: 50,
      lessTrigger: {
        side: "buy",
        triggerPrice: 80,
        executionPrice: null,
      },
    });
    const enhancedLimitResult = await orders.placeIsolatedLimitOrderEnhanced({
      authority: "authority",
      symbol: "SOL",
      side: "buy",
      numBaseLots: 3,
    });
    const enhancedMarketResult = await orders.placeIsolatedMarketOrderEnhanced({
      authority: "authority",
      symbol: "SOL",
      side: "sell",
      numBaseLots: 4,
    });
    const cancelIxs = await orders.cancelConditionalOrder({
      authority: "authority",
      traderPdaIndex: 0,
      symbol: "SOL",
      conditionalOrderIndex: 7,
      executionDirection: "greater_than",
    });
    await invite.activateInvite({ authority: "authority", code: "code" });

    expect(limitIxs).toHaveLength(1);
    expect(marketIxs).toHaveLength(1);
    expect(limitWithConditionalsIxs).toHaveLength(1);
    expect(stopLossIxs).toHaveLength(1);
    expect(cancelStopLossIxs).toHaveLength(1);
    expect(attachedConditionalIxs).toHaveLength(1);
    expect(positionConditionalIxs).toHaveLength(1);
    expect(enhancedLimitResult.instructions).toHaveLength(1);
    expect(enhancedMarketResult.instructions).toHaveLength(1);
    expect(cancelIxs).toHaveLength(1);
    expect(enhancedLimitResult.estimatedLiquidationPriceUsd).toBe(74.54);
    expect(enhancedMarketResult.estimatedLiquidationPriceUsd).toBe(73.3);
    expect(records).toEqual([
      {
        method: "POST",
        endpoint: "/v1/ix/place-isolated-limit-order",
        params: undefined,
        body: {
          authority: "authority",
          symbol: "SOL",
          side: "buy",
          numBaseLots: 1,
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/place-isolated-market-order",
        params: undefined,
        body: {
          authority: "authority",
          symbol: "SOL",
          side: "sell",
          numBaseLots: 2,
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/place-isolated-limit-order-with-conditionals",
        params: undefined,
        body: {
          authority: "authority",
          symbol: "SOL",
          side: "buy",
          price: 100,
          quantity: 1,
          greaterTrigger: {
            side: "sell",
            triggerPrice: 120,
            executionPrice: 119,
            orderKind: "limit",
          },
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/place-stop-loss-order",
        params: undefined,
        body: {
          authority: "authority",
          traderPdaIndex: 0,
          symbol: "SOL",
          side: "sell",
          stopLossTriggerPrice: 90,
          stopLossExecutionPrice: 89,
          orderKind: "ioc",
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/cancel-stop-loss-order",
        params: undefined,
        body: {
          authority: "authority",
          traderPdaIndex: 0,
          symbol: "SOL",
          executionDirection: "less_than",
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/place-attached-conditional-order",
        params: undefined,
        body: {
          authority: "authority",
          traderPdaIndex: 0,
          symbol: "SOL",
          orderSequenceNumber: "42",
          orderPriceInTicks: 1234,
          greaterTrigger: {
            side: "sell",
            triggerPriceInTicks: 1500,
          },
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/place-position-conditional-order",
        params: undefined,
        body: {
          authority: "authority",
          traderPdaIndex: 0,
          symbol: "SOL",
          sizePercent: 50,
          lessTrigger: {
            side: "buy",
            triggerPrice: 80,
            executionPrice: null,
          },
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/place-isolated-limit-order-enhanced",
        params: undefined,
        body: {
          authority: "authority",
          symbol: "SOL",
          side: "buy",
          numBaseLots: 3,
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/place-isolated-market-order-enhanced",
        params: undefined,
        body: {
          authority: "authority",
          symbol: "SOL",
          side: "sell",
          numBaseLots: 4,
        },
      },
      {
        method: "POST",
        endpoint: "/v1/ix/cancel-conditional-order",
        params: undefined,
        body: {
          authority: "authority",
          traderPdaIndex: 0,
          symbol: "SOL",
          conditionalOrderIndex: 7,
          executionDirection: "greater_than",
        },
      },
      {
        method: "POST",
        endpoint: "/v1/invite/activate",
        params: undefined,
        body: {
          authority: "authority",
          code: "code",
        },
      },
    ]);
  });
});
