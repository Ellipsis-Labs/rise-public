import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import type {
  MarketStatsHistoryParams,
  MarketStatsHistoryResponse,
  NextCommodityMarketTransition,
} from "./types";
import {
  MarketStatsHistoryResponseSchema,
  NextCommodityMarketTransitionSchema,
} from "./types";
import { compactParams } from "@/api/utils/query";
import type { ExchangeMarketConfig } from "@/api/exchange";
import { ExchangeMarketConfigSchema } from "@/api/exchange";

export class V1MarketsClient {
  constructor(private http: HttpTransport) {}

  async getMarkets(): Promise<ExchangeMarketConfig[]> {
    return get(
      this.http,
      "/exchange/markets",
      ExchangeMarketConfigSchema.array()
    );
  }

  async getMarket(symbol: string): Promise<ExchangeMarketConfig> {
    return get(
      this.http,
      `/exchange/market/${encodeURIComponent(symbol)}`,
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

  async getNextCommodityMarketTransition(): Promise<NextCommodityMarketTransition> {
    return get(
      this.http,
      "/v1/market/next-commodity-market-transition",
      NextCommodityMarketTransitionSchema
    );
  }
}
