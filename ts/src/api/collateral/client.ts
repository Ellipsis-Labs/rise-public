import type { HttpTransport } from "@/http/transport";
import { get } from "@/http/transport";
import type { ParamValue } from "@/http/transport";
import type {
  CollateralHistoryRequest,
  CollateralHistoryResponse,
} from "./types";
import { CollateralHistoryResponseSchema } from "./types";

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
      `/trader/${encodeURIComponent(authority)}/collateral-history`,
      CollateralHistoryResponseSchema,
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
}
