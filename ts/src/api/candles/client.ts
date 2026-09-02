import type { HttpTransport, ParamValue } from "@/http/transport";
import { get } from "@/http/transport";
import {
  ApiCandleSchema,
  type ApiCandle,
  CandlesV2ResponseSchema,
  type CandlesV2Response,
  type TradingCandlesQuery,
  type TradingCandlesV2Query,
} from "./types";
import { compactParams } from "@/api/utils/query";

const DEFAULT_CANDLES_V2_LIMIT = 1_000;

const buildCandlesV2Query = (
  params: TradingCandlesV2Query
): Record<string, ParamValue> => {
  if ("cursor" in params) {
    const hasConflictingFields =
      "timeframe" in params ||
      "from" in params ||
      "to" in params ||
      "limit" in params ||
      "includePartial" in params;
    if (hasConflictingFields) {
      throw new Error(
        "candles_v2 cursor requests must not include timeframe, from, to, limit, or includePartial"
      );
    }
    return (
      compactParams({
        cursor: params.cursor,
      }) ?? {}
    );
  }

  if (
    params.timeframe === undefined ||
    params.from === undefined ||
    params.to === undefined
  ) {
    throw new Error(
      "candles_v2 initial requests require timeframe, from, and to"
    );
  }

  return (
    compactParams({
      timeframe: params.timeframe,
      from: params.from,
      to: params.to,
      limit: params.limit ?? DEFAULT_CANDLES_V2_LIMIT,
      includePartial: params.includePartial,
    }) ?? {}
  );
};

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

  async getCandlesV2(
    symbol: string,
    params: TradingCandlesV2Query
  ): Promise<CandlesV2Response> {
    return get(this.http, `/v1/candles_v2/${symbol}`, CandlesV2ResponseSchema, {
      params: buildCandlesV2Query(params),
    });
  }
}
