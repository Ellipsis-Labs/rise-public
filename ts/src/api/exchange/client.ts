import type { HttpTransport } from "@/http/transport";
import { get, post } from "@/http/transport";
import type {
  BuildRegisterIxsRequest,
  BuildRegisterIxsResponse,
  ExchangeConfig,
  ExchangeKeys,
  ExchangeMarketConfig,
  ExchangeSnapshotView,
  ExchangeStatusView,
  SendRegisterIxsRequest,
  SendRegisterIxsResponse,
} from "./types";
import {
  BuildRegisterIxsRequestSchema,
  BuildRegisterIxsResponseSchema,
  ExchangeConfigSchema,
  ExchangeKeysSchema,
  ExchangeMarketConfigSchema,
  ExchangeSnapshotViewSchema,
  ExchangeStatusViewSchema,
  SendRegisterIxsRequestSchema,
  SendRegisterIxsResponseSchema,
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

  async buildRegisterIxs(
    request: BuildRegisterIxsRequest
  ): Promise<BuildRegisterIxsResponse> {
    const payload = BuildRegisterIxsRequestSchema.parse(request);
    return post(
      this.http,
      "/v1/exchange/build-register-ixs",
      BuildRegisterIxsResponseSchema,
      payload
    );
  }

  async sendRegisterIxs(
    request: SendRegisterIxsRequest
  ): Promise<SendRegisterIxsResponse> {
    const payload = SendRegisterIxsRequestSchema.parse(request);
    return post(
      this.http,
      "/v1/exchange/send-register-ixs",
      SendRegisterIxsResponseSchema,
      payload
    );
  }
}
