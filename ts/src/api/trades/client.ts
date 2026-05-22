import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import type { ParamValue } from "@/http/transport";
import type {
  FillsResponse,
  MarketFillsResponse,
  MarketTradeHistoryRequest,
  TradeHistoryRequest,
  TradeHistoryV2Response,
} from "./types";
import {
  FillsResponseSchema,
  MarketFillsResponseSchema,
  TradeHistoryV2ResponseSchema,
} from "./types";

const buildTradeHistoryQuery = (
  request?: TradeHistoryRequest
): Record<string, ParamValue> | undefined => {
  if (!request) return undefined;

  const params: Record<string, ParamValue> = {};

  if (request.pdaIndex !== undefined) params.pdaIndex = request.pdaIndex;
  if (request.marketSymbol) params.marketSymbol = request.marketSymbol;
  if (request.limit !== undefined) params.limit = request.limit;
  if (request.cursor) params.cursor = request.cursor;
  if (request.privyId) params.privyId = request.privyId;

  return Object.keys(params).length > 0 ? params : undefined;
};

const buildMarketTradeHistoryQuery = (
  request?: MarketTradeHistoryRequest
): Record<string, ParamValue> | undefined => {
  if (!request) return undefined;

  const params: Record<string, ParamValue> = {};

  if (request.limit !== undefined) params.limit = request.limit;
  if (request.cursor) params.cursor = request.cursor;
  if (request.startTime !== undefined) params.startTime = request.startTime;
  if (request.endTime !== undefined) params.endTime = request.endTime;

  return Object.keys(params).length > 0 ? params : undefined;
};

const buildTradeHistoryV2Query = (
  request?: TradeHistoryRequest
): Record<string, ParamValue> | undefined => {
  if (!request) return undefined;

  const params: Record<string, ParamValue> = {};

  if (request.marketSymbol) params.market_symbol = request.marketSymbol;
  if (request.limit !== undefined) params.limit = request.limit;
  if (request.cursor) params.cursor = request.cursor;
  if (request.privyId) params.privy_id = request.privyId;

  return Object.keys(params).length > 0 ? params : undefined;
};

export class V1TradesClient {
  constructor(private readonly http: HttpTransport) {}

  async getTraderTradesHistory(
    authority: string,
    request?: TradeHistoryRequest
  ): Promise<FillsResponse> {
    return get(
      this.http,
      `/trader/${encodeURIComponent(authority)}/trades-history`,
      FillsResponseSchema,
      { params: buildTradeHistoryQuery(request) }
    );
  }

  async getTraderTradeHistoryV2(
    traderPubkey: string,
    request?: TradeHistoryRequest
  ): Promise<TradeHistoryV2Response> {
    return get(
      this.http,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/trades_v2`,
      TradeHistoryV2ResponseSchema,
      { params: buildTradeHistoryV2Query(request) }
    );
  }

  async getMarketFills(
    symbol: string,
    request?: MarketTradeHistoryRequest
  ): Promise<MarketFillsResponse> {
    return get(
      this.http,
      `/market/${encodeURIComponent(symbol)}/fills`,
      MarketFillsResponseSchema,
      { params: buildMarketTradeHistoryQuery(request) }
    );
  }
}
