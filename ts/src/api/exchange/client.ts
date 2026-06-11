import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import type {
  ExchangeConfig,
  ExchangeKeys,
  ExchangeMarketConfig,
  ExchangeSnapshotView,
  ExchangeStatusView,
} from "./types";
import {
  ExchangeConfigSchema,
  ExchangeKeysSchema,
  ExchangeMarketConfigSchema,
  ExchangeSnapshotViewSchema,
  ExchangeStatusViewSchema,
} from "./types";

export class V1ExchangeClient {
  constructor(private http: HttpTransport) {}

  async getExchange(): Promise<ExchangeConfig> {
    return get(this.http, "/v1/view/exchange", ExchangeConfigSchema);
  }

  async getSnapshot(): Promise<ExchangeSnapshotView> {
    return get(this.http, "/v1/exchange/snapshot", ExchangeSnapshotViewSchema);
  }

  async getMarket(symbol: string): Promise<ExchangeMarketConfig> {
    return get(
      this.http,
      `/v1/view/exchange/market/${encodeURIComponent(symbol)}`,
      ExchangeMarketConfigSchema
    );
  }

  async getStatus(): Promise<ExchangeStatusView> {
    return get(this.http, "/v1/view/exchange/status", ExchangeStatusViewSchema);
  }

  async getKeys(): Promise<ExchangeKeys> {
    return get(this.http, "/v1/view/exchange/keys", ExchangeKeysSchema);
  }

  async getMarkets(): Promise<ExchangeMarketConfig[]> {
    return get(
      this.http,
      "/v1/view/exchange/markets",
      ExchangeMarketConfigSchema.array()
    );
  }
}
