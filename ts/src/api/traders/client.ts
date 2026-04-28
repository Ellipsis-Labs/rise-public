import type { HttpTransport } from "@/http/transport";
import type { ParamValue } from "@/http/transport";
import { get } from "@/http/transport";
import type { TraderStateSnapshotResponse } from "./traderState";
import type {
  HistoricalValuesRequest,
  PnlDataPoint,
  PortfolioValueDataPoint,
  TraderCapabilitiesMetadata,
  TraderMarketPnLQueryParams,
  TraderMarketPnLSeries,
  TraderStateResponse,
  TraderView,
} from "./types";
import {
  PnlDataPointSchema,
  PortfolioValueDataPointSchema,
  TraderCapabilitiesMetadataSchema,
  TraderMarketPnLSeriesSchema,
  TraderStateResponseSchema,
  TraderViewSchema,
} from "./types";
import { TraderStateSnapshotResponseSchema } from "./traderState";

const buildHistoricalValuesQuery = (
  request: HistoricalValuesRequest
): Record<string, ParamValue> => {
  const params: Record<string, ParamValue> = {
    resolution: request.resolution,
  };

  if (request.startTime !== undefined) params.startTime = request.startTime;
  if (request.endTime !== undefined) params.endTime = request.endTime;
  if (request.limit !== undefined) params.limit = request.limit;
  if (request.includeEarliest !== undefined) {
    params.includeEarliest = request.includeEarliest;
  }
  if (request.includeLatest !== undefined) {
    params.includeLatest = request.includeLatest;
  }

  return params;
};

export interface TraderStateRequest {
  pdaIndex?: number;
}

export class V1TradersClient {
  constructor(private http: HttpTransport) {}

  async getTrader(pubkey: string): Promise<TraderView> {
    return get(this.http, `/v1/view/trader/${pubkey}`, TraderViewSchema);
  }

  async getTraderState(
    authority: string,
    request?: TraderStateRequest
  ): Promise<TraderStateResponse> {
    const params: Record<string, ParamValue> | undefined =
      request?.pdaIndex !== undefined
        ? { pdaIndex: request.pdaIndex }
        : undefined;

    return get(
      this.http,
      `/trader/${encodeURIComponent(authority)}/state`,
      TraderStateResponseSchema,
      { params }
    );
  }

  async getTraderStateSnapshot(
    authority: string,
    request?: { traderPdaIndex?: number }
  ): Promise<TraderStateSnapshotResponse> {
    const params: Record<string, ParamValue> | undefined =
      request?.traderPdaIndex !== undefined
        ? { traderPdaIndex: request.traderPdaIndex }
        : undefined;

    return get(
      this.http,
      `/v1/trader/state/${encodeURIComponent(authority)}`,
      TraderStateSnapshotResponseSchema,
      { params }
    );
  }

  async getTraderPnl(
    authority: string,
    request: HistoricalValuesRequest
  ): Promise<PnlDataPoint[]> {
    return get(
      this.http,
      `/trader/${encodeURIComponent(authority)}/pnl`,
      PnlDataPointSchema.array(),
      { params: buildHistoricalValuesQuery(request) }
    );
  }

  async getTraderPortfolioValues(
    traderPubkey: string,
    request: HistoricalValuesRequest
  ): Promise<PortfolioValueDataPoint[]> {
    return get(
      this.http,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/portfolio-values`,
      PortfolioValueDataPointSchema.array(),
      { params: buildHistoricalValuesQuery(request) }
    );
  }

  async getTraderPnlValues(
    traderPubkey: string,
    request: HistoricalValuesRequest
  ): Promise<PnlDataPoint[]> {
    return get(
      this.http,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/pnl`,
      PnlDataPointSchema.array(),
      { params: buildHistoricalValuesQuery(request) }
    );
  }

  async getTraderCapabilities(): Promise<TraderCapabilitiesMetadata> {
    return get(
      this.http,
      "/v1/view/trader-capabilities",
      TraderCapabilitiesMetadataSchema
    );
  }

  async getTraderMarketPnl(
    traderPubkey: string,
    request?: TraderMarketPnLQueryParams
  ): Promise<TraderMarketPnLSeries[]> {
    const params: Record<string, ParamValue> | undefined = request
      ? {
          resolution: request.resolution,
          ...(request.startTime !== undefined
            ? { startTime: request.startTime }
            : {}),
          ...(request.endTime !== undefined
            ? { endTime: request.endTime }
            : {}),
          ...(request.limit !== undefined ? { limit: request.limit } : {}),
          ...(request.symbols && request.symbols.length > 0
            ? { symbols: request.symbols }
            : {}),
        }
      : undefined;

    return get(
      this.http,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/pnl/markets`,
      TraderMarketPnLSeriesSchema.array(),
      { params }
    );
  }
}
