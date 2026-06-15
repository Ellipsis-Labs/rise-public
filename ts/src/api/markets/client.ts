import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import type {
  CommodityMarketCalendarResponse,
  MarketStatsHistoryParams,
  MarketStatsHistoryResponse,
  MarketCalendarRecord,
  MarketCalendarResponse,
  NextCommodityMarketTransition,
  NextMarketCalendarTransition,
} from "./types";
import {
  CommodityMarketCalendarResponseSchema,
  MarketStatsHistoryResponseSchema,
  MarketCalendarRecordSchema,
  MarketCalendarResponseSchema,
  NextCommodityMarketTransitionSchema,
  NextMarketCalendarTransitionSchema,
} from "./types";
import { compactParams } from "@/api/utils/query";
import type { ExchangeMarketConfig } from "@/api/exchange";
import { ExchangeMarketConfigSchema } from "@/api/exchange";

export class V1MarketsClient {
  constructor(private http: HttpTransport) {}

  async getMarkets(): Promise<ExchangeMarketConfig[]> {
    return get(
      this.http,
      "/v1/view/exchange/markets",
      ExchangeMarketConfigSchema.array()
    );
  }

  async getMarket(symbol: string): Promise<ExchangeMarketConfig> {
    return get(
      this.http,
      `/v1/view/exchange/market/${encodeURIComponent(symbol)}`,
      ExchangeMarketConfigSchema
    );
  }

  async getMarketStatsHistory(
    symbol: string,
    params?: MarketStatsHistoryParams
  ): Promise<MarketStatsHistoryResponse> {
    return get(
      this.http,
      `/v1/market/${encodeURIComponent(symbol)}/stats`,
      MarketStatsHistoryResponseSchema,
      { params: compactParams(params) }
    );
  }

  async getNextMarketCalendarTransition(
    symbol: string
  ): Promise<NextMarketCalendarTransition> {
    return get(
      this.http,
      `/v1/market/${encodeURIComponent(symbol)}/next-market-calendar-transition`,
      NextMarketCalendarTransitionSchema
    );
  }

  async getNextCommodityMarketTransition(): Promise<NextCommodityMarketTransition> {
    return get(
      this.http,
      "/v1/market/next-commodity-market-transition",
      NextCommodityMarketTransitionSchema
    );
  }

  async getMarketCalendar(symbol: string): Promise<MarketCalendarResponse> {
    return get(
      this.http,
      `/v1/market/${encodeURIComponent(symbol)}/market-calendar`,
      MarketCalendarResponseSchema
    );
  }

  async getMarketCalendarById(
    marketCalendarId: string
  ): Promise<MarketCalendarRecord> {
    return get(
      this.http,
      `/v1/market-calendar/${encodeURIComponent(marketCalendarId)}`,
      MarketCalendarRecordSchema
    );
  }

  async getCommodityMarketCalendar(): Promise<CommodityMarketCalendarResponse> {
    return get(
      this.http,
      "/v1/market/commodity-calendar",
      CommodityMarketCalendarResponseSchema
    );
  }
}
