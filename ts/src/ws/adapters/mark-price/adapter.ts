import { createUpdateStream } from "@/ws/adapters/_utils";
import type { WsClient } from "@/ws/types";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";

import type { MarkPricePort } from "./ports";
import {
  MarkPriceMsgSchema,
  type MarkPriceMsg,
  type MarkPriceUpdate,
} from "./wire";

export type MarkPriceAdapter = MarkPricePort;

export interface MarkPriceAdapterOptions {
  buffer?: number;
}

export const createMarkPriceAdapter = (
  ws: WsClient,
  opts?: MarkPriceAdapterOptions,
  strictMode?: boolean
): MarkPriceAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(MarkPriceMsgSchema)
    : MarkPriceMsgSchema;

  return createUpdateStream<MarkPriceMsg, MarkPriceUpdate, [symbol: string]>(
    ws,
    {
      channel: "markPrice",
      schema,
      buildKey: (symbol: string) => `markPrice:${symbol}`,
      buildSubParams: (symbol: string) => ({ symbol }),
      processMessage: (m) => ({
        symbol: m.symbol,
        slot: m.slot,
        slotIndex: m.slotIndex,
        markPrice: m.markPrice,
      }),
      schemaErrorMessage: "Failed to parse MarkPrice message",
    },
    opts
  );
};
