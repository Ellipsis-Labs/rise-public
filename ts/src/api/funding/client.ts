import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import type { ParamValue } from "@/http/transport";
import type {
  FundingHourlyHistoryResponse,
  FundingHourlyRequest,
  FundingOverviewRequest,
  FundingOverviewResponse,
  FundingRateHistoryRequest,
  FundingRateHistoryResponse,
  TraderFundingHistoryRequest,
  TraderFundingHistoryResponse,
} from "./types";
import {
  FundingHourlyHistoryResponseSchema,
  FundingOverviewResponseSchema,
  FundingRateHistoryResponseSchema,
  TraderFundingHistoryResponseSchema,
} from "./types";

const setDefinedParam = (
  params: Record<string, ParamValue>,
  key: string,
  value: ParamValue
) => {
  if (value !== undefined) {
    params[key] = value;
  }
};

const buildFundingRateHistoryQuery = (
  request?: FundingRateHistoryRequest
): Record<string, ParamValue> | undefined => {
  if (!request) return undefined;

  const params: Record<string, ParamValue> = {};

  setDefinedParam(params, "startTime", request.startTime);
  setDefinedParam(params, "endTime", request.endTime);
  setDefinedParam(params, "limit", request.limit);

  return Object.keys(params).length > 0 ? params : undefined;
};

const buildFundingOverviewQuery = (
  request?: FundingOverviewRequest
): Record<string, ParamValue> | undefined => {
  if (!request) return undefined;

  const params: Record<string, ParamValue> = {};

  setDefinedParam(params, "startTime", request.startTime);
  setDefinedParam(params, "endTime", request.endTime);
  setDefinedParam(params, "perMarketLimit", request.perMarketLimit);

  return Object.keys(params).length > 0 ? params : undefined;
};

export class V1FundingClient {
  constructor(private readonly http: HttpTransport) {}

  async getUserFundingHourly(
    userPubkey: string,
    request?: FundingHourlyRequest
  ): Promise<FundingHourlyHistoryResponse> {
    const params: Record<string, ParamValue> = {};

    setDefinedParam(params, "symbol", request?.symbol);
    setDefinedParam(params, "limit", request?.limit);
    setDefinedParam(params, "cursor", request?.cursor);
    setDefinedParam(params, "traderPdaIndex", request?.traderPdaIndex);

    return get(
      this.http,
      `/v1/users/${encodeURIComponent(userPubkey)}/funding-hourly`,
      FundingHourlyHistoryResponseSchema,
      {
        params: Object.keys(params).length > 0 ? params : undefined,
      }
    );
  }

  async getTraderFundingHistory(
    authority: string,
    request?: TraderFundingHistoryRequest
  ): Promise<TraderFundingHistoryResponse> {
    const params: Record<string, ParamValue> = {};

    setDefinedParam(params, "traderPdaIndex", request?.pdaIndex);
    setDefinedParam(params, "symbol", request?.symbol);
    setDefinedParam(params, "startTime", request?.startTime);
    setDefinedParam(params, "endTime", request?.endTime);
    setDefinedParam(params, "limit", request?.limit);
    setDefinedParam(params, "cursor", request?.cursor);
    setDefinedParam(params, "resolution", request?.resolution);

    return get(
      this.http,
      `/v1/trader/${encodeURIComponent(authority)}/funding-history`,
      TraderFundingHistoryResponseSchema,
      { params: Object.keys(params).length > 0 ? params : undefined }
    );
  }

  async getFundingRateHistory(
    symbol: string,
    request?: FundingRateHistoryRequest
  ): Promise<FundingRateHistoryResponse> {
    return get(
      this.http,
      `/v1/funding/${encodeURIComponent(symbol)}/rates`,
      FundingRateHistoryResponseSchema,
      { params: buildFundingRateHistoryQuery(request) }
    );
  }

  async getFundingOverview(
    request?: FundingOverviewRequest
  ): Promise<FundingOverviewResponse> {
    return get(
      this.http,
      "/v1/funding/overview",
      FundingOverviewResponseSchema,
      { params: buildFundingOverviewQuery(request) }
    );
  }
}
