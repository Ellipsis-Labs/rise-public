import type { WsClient } from "@/ws/types";
import { createUpdateStream } from "@/ws/adapters/_utils";
import type { ExchangeStatusPort } from "./ports";
import { ExchangeStatusMsgSchema } from "./wire";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";

export type ExchangeStatusAdapter = ExchangeStatusPort;

export interface ExchangeStatusAdapterOptions {
  buffer?: number;
}

export const createExchangeStatusAdapter = (
  ws: WsClient,
  opts?: ExchangeStatusAdapterOptions,
  strictMode?: boolean
): ExchangeStatusAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(ExchangeStatusMsgSchema)
    : ExchangeStatusMsgSchema;
  return createUpdateStream(
    ws,
    {
      channel: "exchangeStatus",
      schema,
      buildKey: () => "exchangeStatus",
      buildSubParams: () => ({}),
      processMessage: (m) => ({
        id: m.id,
        notificationType: m.notificationType,
        data: m.data,
        isRead: m.isRead,
        createdAt: m.createdAt,
      }),
      schemaErrorMessage: "Failed to parse ExchangeStatus message",
    },
    opts
  );
};
