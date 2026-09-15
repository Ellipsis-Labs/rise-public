import { createUpdateStream } from "@/ws/adapters/_utils";
import type { WsClient } from "@/ws/types";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import { normalizeMarketStatsEntries } from "../market-stats/normalize";
import type { MarketStatsV2Port } from "./ports";
import {
  buildMarketStatsV2RoutingKey,
  buildMarketStatsV2SubscriptionParams,
  type MarketStatsV2Selector,
} from "./routing";
import {
  MarketStatsV2MsgSchema,
  type MarketStatsV2Data,
  type MarketStatsV2Msg,
  type MarketStatsV2Update,
} from "./wire";

export type MarketStatsV2Adapter = MarketStatsV2Port;

export interface MarketStatsV2AdapterOptions {
  buffer?: number;
}

export const createMarketStatsV2Adapter = (
  ws: WsClient,
  opts?: MarketStatsV2AdapterOptions,
  strictMode?: boolean
): MarketStatsV2Adapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(MarketStatsV2MsgSchema)
    : MarketStatsV2MsgSchema;

  return createUpdateStream<
    MarketStatsV2Msg,
    MarketStatsV2Update,
    [symbols?: MarketStatsV2Selector]
  >(
    ws,
    {
      channel: "marketStatsV2",
      schema,
      buildKey: buildMarketStatsV2RoutingKey,
      buildSubParams: buildMarketStatsV2SubscriptionParams,
      processMessage: (message) => {
        const entries = normalizeMarketStatsEntries(message.stats);
        if (!entries) {
          return null;
        }
        const stats: MarketStatsV2Data[] = entries.map(({ raw, update }) => ({
          ...update,
          stats: {
            ...update.stats,
            ...(raw.midPrice === undefined ? {} : { midPrice: raw.midPrice }),
          },
        }));
        return { symbols: message.symbols, stats };
      },
      schemaErrorMessage: "Failed to parse MarketStatsV2 message",
    },
    opts
  );
};
