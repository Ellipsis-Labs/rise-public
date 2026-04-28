import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import { compactParams } from "@/api/utils/query";
import type { OrderbookQueryParams, OrderbookView } from "./types";
import { OrderbookViewSchema } from "./types";

export class V1OrderbookClient {
  constructor(private readonly http: HttpTransport) {}

  async getOrderbook(
    symbol: string,
    params?: OrderbookQueryParams
  ): Promise<OrderbookView> {
    return get(
      this.http,
      `/v1/view/orderbook/${encodeURIComponent(symbol)}`,
      OrderbookViewSchema,
      { params: compactParams(params) }
    );
  }
}
