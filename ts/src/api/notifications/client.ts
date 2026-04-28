import z from "zod";
import type { HttpTransport } from "@/http/transport";
import { get, post } from "@/http/transport";
import { compactParams } from "@/api/utils/query";
import {
  AckNotificationsBodySchema,
  GetNotificationsResponseSchema,
  type AckBeforeTimestampBody,
  type AckNotificationsBody,
  type GetNotificationsQuery,
  type GetNotificationsResponse,
} from "./types";

export class V1NotificationsClient {
  constructor(private readonly httpClient: HttpTransport) {}

  async getNotificationsByTrader(
    traderPubkey: string,
    query?: GetNotificationsQuery
  ): Promise<GetNotificationsResponse> {
    return get(
      this.httpClient,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/notifications`,
      GetNotificationsResponseSchema,
      { params: compactParams(query as Record<string, unknown>) }
    );
  }

  async ackUpToTimestamp(
    traderPubkey: string,
    body: AckBeforeTimestampBody
  ): Promise<{ ok: boolean }> {
    return post(
      this.httpClient,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/notifications/ack/up-to`,
      z.object({ ok: z.boolean() }),
      body
    );
  }

  async ackNotifications(
    traderPubkey: string,
    body: AckNotificationsBody
  ): Promise<{ ok: boolean }> {
    return post(
      this.httpClient,
      `/v1/traders/${encodeURIComponent(traderPubkey)}/notifications/ack/notifications`,
      z.object({ ok: z.boolean() }),
      AckNotificationsBodySchema.parse(body)
    );
  }
}
