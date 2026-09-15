import { createUpdateStream } from "@/ws/adapters/_utils";
import type { WsClient } from "@/ws/types";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";

import {
  buildMarketStatsV2RoutingKey,
  buildMarketStatsV2SubscriptionParams,
} from "../market-stats-v2/routing";
import {
  MarketStatsV2MsgSchema,
  type MarketStatsV2Msg,
} from "../market-stats-v2/wire";
import { normalizeMarketStatsEntries } from "./normalize";
import type { MarketStatsPort } from "./ports";
import type { MarketStatsUpdate } from "./wire";

export type MarketStatsAdapter = MarketStatsPort;

export interface MarketStatsAdapterOptions {
  buffer?: number;
}

export const createMarketStatsAdapter = (
  ws: WsClient,
  opts?: MarketStatsAdapterOptions,
  strictMode?: boolean
): MarketStatsAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(MarketStatsV2MsgSchema)
    : MarketStatsV2MsgSchema;
  return createUpdateStream<
    MarketStatsV2Msg,
    MarketStatsUpdate,
    [symbol?: string]
  >(
    ws,
    {
      channel: "marketStatsV2",
      schema,
      buildKey: buildMarketStatsV2RoutingKey,
      buildSubParams: buildMarketStatsV2SubscriptionParams,
      processMessage: (message) =>
        normalizeMarketStatsEntries(message.stats)?.map(
          ({ update }) => update
        ) ?? null,
      schemaErrorMessage: "Failed to parse MarketStatsV2 compatibility message",
    },
    opts
  );
};
