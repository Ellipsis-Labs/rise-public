import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import type { ParamValue } from "@/http/transport";
import type {
  CollateralAssetsResponse,
  CollateralHistoryRequest,
  CollateralHistoryResponse,
  CollateralHistoryV2Request,
  CollateralHistoryV2Response,
  CollateralTotalsResponse,
} from "./types";
import {
  CollateralAssetsResponseSchema,
  CollateralHistoryResponseSchema,
  CollateralHistoryV2ResponseSchema,
  CollateralTotalsResponseSchema,
} from "./types";

const buildCollateralHistoryQuery = (
  request?: CollateralHistoryRequest
): Record<string, ParamValue> | undefined => {
  if (!request) return undefined;

  const params: Record<string, ParamValue> = {};

  if (request.pdaIndex !== undefined) params.pdaIndex = request.pdaIndex;
  if (request.limit !== undefined) params.limit = request.limit;
  if (request.nextCursor) params.nextCursor = request.nextCursor;
  if (request.prevCursor) params.prevCursor = request.prevCursor;
  if (request.cursor) params.cursor = request.cursor;

  return Object.keys(params).length > 0 ? params : undefined;
};

export class V1CollateralClient {
  constructor(private readonly http: HttpTransport) {}

  async getAssets(): Promise<CollateralAssetsResponse> {
    return get(
      this.http,
      "/v1/collateral/assets",
      CollateralAssetsResponseSchema
    );
  }

  async getUserCollateralTotals(
    userPubkey: string
  ): Promise<CollateralTotalsResponse> {
    return get(
      this.http,
      `/v1/users/${encodeURIComponent(userPubkey)}/collateral-totals`,
      CollateralTotalsResponseSchema
    );
  }

  async getUserCollateralHistory(
    userPubkey: string,
    request?: Omit<CollateralHistoryRequest, "pdaIndex">
  ): Promise<CollateralHistoryResponse> {
    return get(
      this.http,
      `/v1/users/${encodeURIComponent(userPubkey)}/collateral-history`,
      CollateralHistoryResponseSchema,
      { params: buildCollateralHistoryQuery(request) }
    );
  }

  async getTraderCollateralHistory(
    authority: string,
    request?: CollateralHistoryRequest
  ): Promise<CollateralHistoryResponse> {
    return get(
      this.http,
      `/v1/trader/${encodeURIComponent(authority)}/collateral-history`,
      CollateralHistoryResponseSchema,
      { params: buildCollateralHistoryQuery(request) }
    );
  }

  async getTraderPdaCollateralHistory(
    traderPubkey: string,
    request?: Omit<CollateralHistoryRequest, "pdaIndex">
  ): Promise<CollateralHistoryResponse> {
    return get(
      this.http,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/collateral-history`,
      CollateralHistoryResponseSchema,
      { params: buildCollateralHistoryQuery(request) }
    );
  }

  /** Mixed history across quote and spot collateral assets. */
  async getUserCollateralHistoryV2(
    userPubkey: string,
    request: Omit<CollateralHistoryV2Request, "pdaIndex">
  ): Promise<CollateralHistoryV2Response> {
    return get(
      this.http,
      `/v1/users/${encodeURIComponent(userPubkey)}/collateral-history-v2`,
      CollateralHistoryV2ResponseSchema,
      { params: buildCollateralHistoryQuery(request) }
    );
  }

  /** Mixed history across quote and spot collateral assets. */
  async getTraderCollateralHistoryV2(
    authority: string,
    request: CollateralHistoryV2Request
  ): Promise<CollateralHistoryV2Response> {
    return get(
      this.http,
      `/v1/trader/${encodeURIComponent(authority)}/collateral-history-v2`,
      CollateralHistoryV2ResponseSchema,
      { params: buildCollateralHistoryQuery(request) }
    );
  }

  /** Mixed history across quote and spot collateral assets. */
  async getTraderPdaCollateralHistoryV2(
    traderPubkey: string,
    request: Omit<CollateralHistoryV2Request, "pdaIndex">
  ): Promise<CollateralHistoryV2Response> {
    return get(
      this.http,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/collateral-history-v2`,
      CollateralHistoryV2ResponseSchema,
      { params: buildCollateralHistoryQuery(request) }
    );
  }

  async getAllUserCollateralHistory(
    userPubkey: string,
    pageSize = 1000,
    request?: Omit<
      CollateralHistoryRequest,
      "limit" | "nextCursor" | "pdaIndex"
    >
  ): Promise<CollateralHistoryResponse["data"]> {
    const allEvents: CollateralHistoryResponse["data"] = [];
    let cursor: string | undefined;
    let hasMore = true;

    while (hasMore) {
      const response = await this.getUserCollateralHistory(userPubkey, {
        ...request,
        limit: pageSize,
        nextCursor: cursor,
      });

      allEvents.push(...response.data);
      hasMore = response.hasMore;
      cursor = response.nextCursor ?? undefined;
    }

    return allEvents;
  }

  async getAllTraderCollateralHistory(
    authority: string,
    pageSize = 1000,
    request?: Omit<CollateralHistoryRequest, "limit" | "nextCursor">
  ): Promise<CollateralHistoryResponse["data"]> {
    const allEvents: CollateralHistoryResponse["data"] = [];
    let cursor: string | undefined;
    let hasMore = true;

    while (hasMore) {
      const response = await this.getTraderCollateralHistory(authority, {
        ...request,
        limit: pageSize,
        nextCursor: cursor,
      });

      allEvents.push(...response.data);
      hasMore = response.hasMore;
      cursor = response.nextCursor ?? undefined;
    }

    return allEvents;
  }

  async getAllTraderPdaCollateralHistory(
    traderPubkey: string,
    pageSize = 1000,
    request?: Omit<
      CollateralHistoryRequest,
      "limit" | "nextCursor" | "pdaIndex"
    >
  ): Promise<CollateralHistoryResponse["data"]> {
    const allEvents: CollateralHistoryResponse["data"] = [];
    let cursor: string | undefined;
    let hasMore = true;

    while (hasMore) {
      const response = await this.getTraderPdaCollateralHistory(traderPubkey, {
        ...request,
        limit: pageSize,
        nextCursor: cursor,
      });

      allEvents.push(...response.data);
      hasMore = response.hasMore;
      cursor = response.nextCursor ?? undefined;
    }

    return allEvents;
  }
}
