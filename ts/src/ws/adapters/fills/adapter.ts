import { createUpdateStream, normalizeTimestamp } from "@/ws/adapters/_utils";
import { handleError } from "@/ws/errorHandling/ErrorSystem";
import { createInvalidTimestampError } from "@/ws/errorHandling/errors";
import type { WsClient } from "@/ws/types";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import type { FillsPort } from "./ports";
import {
  FillDataSchema,
  FillsMsgSchema,
  type FillsMsg,
  type FillUpdate,
} from "./wire";

export type FillsAdapter = FillsPort;

export interface FillsAdapterOptions {
  buffer?: number;
  timestampUnit?: "s" | "ms";
}

export const createFillsAdapter = (
  ws: WsClient,
  opts?: FillsAdapterOptions,
  strictMode?: boolean
): FillsAdapter => {
  const { timestampUnit = "ms" } = opts ?? {};
  const schema = strictMode
    ? applyStrictModeRecursive(FillsMsgSchema)
    : FillsMsgSchema;

  return createUpdateStream<FillsMsg, FillUpdate, [symbol?: string]>(
    ws,
    {
      channel: "fills",
      schema,
      buildKey: (symbol?: string) => (symbol ? `fills:${symbol}` : "fills"),
      buildSubParams: (symbol?: string) =>
        symbol ? { marketSymbol: symbol } : {},
      processMessage: (message, [symbol]) => {
        const updates: FillUpdate[] = [];

        for (const fill of message.fills) {
          const parsed = FillDataSchema.safeParse(fill);
          if (!parsed.success) continue;

          const fillData = parsed.data;
          if (symbol && fillData.marketSymbol !== symbol) continue;

          const baseQty = Number.parseFloat(fillData.baseQty);
          const quoteQty = Number.parseFloat(fillData.quoteQty);
          fillData.price =
            Math.abs(baseQty) > 0 &&
            Number.isFinite(baseQty) &&
            Number.isFinite(quoteQty)
              ? (Math.abs(quoteQty) / Math.abs(baseQty)).toString()
              : fillData.price;

          let timestampMs: number;
          try {
            timestampMs = normalizeTimestamp(fillData.timestamp, timestampUnit);
          } catch {
            void handleError(
              createInvalidTimestampError(timestampUnit, {
                operation: "timestamp_normalization",
              })
            );
            continue;
          }

          const isIsoString =
            typeof fillData.timestamp === "string" &&
            !/^-?\\d+$/.test(fillData.timestamp);

          updates.push({
            symbol: message.symbol,
            fill: {
              ...fillData,
              timestamp: isIsoString ? timestampMs : BigInt(fillData.timestamp),
              timestampMs,
            },
          });
        }

        return updates;
      },
      schemaErrorMessage: "Failed to parse Fills message",
    },
    opts
  );
};
