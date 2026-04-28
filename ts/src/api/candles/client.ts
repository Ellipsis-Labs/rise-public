import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import {
  ApiCandleSchema,
  type ApiCandle,
  type TradingCandlesQuery,
} from "./types";
import { compactParams } from "@/api/utils/query";

export class V1CandlesClient {
  constructor(private http: HttpTransport) {}

  async getCandles(
    symbol: string,
    params?: TradingCandlesQuery
  ): Promise<ApiCandle[]> {
    return get(this.http, `/v1/candles/${symbol}`, ApiCandleSchema.array(), {
      params: compactParams(params),
    });
  }
}
