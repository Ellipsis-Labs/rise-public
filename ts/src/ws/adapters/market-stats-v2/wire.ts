import z from "zod";
import {
  type MarketStats,
  type MarketStatsWireData,
  marketStatsSchema,
  marketStatsWireDataSchema,
} from "../market-stats/shared";
import type { MarketStatsUpdate } from "../market-stats/wire";

export interface MarketStatsV2WireData extends MarketStatsWireData {
  midPrice?: number;
}

export interface MarketStatsV2Data extends MarketStatsUpdate {
  stats: MarketStats & { midPrice?: number };
}

export interface MarketStatsV2Msg {
  channel: "marketStatsV2";
  symbols?: string[];
  stats: MarketStatsV2WireData[];
}

export interface MarketStatsV2Update {
  symbols?: string[];
  stats: MarketStatsV2Data[];
}

export const MarketStatsV2WireDataSchema: z.ZodType<MarketStatsV2WireData> =
  marketStatsWireDataSchema.extend({
    midPrice: z.number().optional(),
  });

const marketStatsV2DataSchema: z.ZodType<MarketStatsV2Data> = z.object({
  symbol: z.string(),
  stats: marketStatsSchema.extend({
    midPrice: z.number().optional(),
  }),
});

export const MarketStatsV2MsgSchema: z.ZodType<MarketStatsV2Msg> = z.object({
  channel: z.literal("marketStatsV2"),
  symbols: z.array(z.string()).nonempty().optional(),
  stats: z.array(MarketStatsV2WireDataSchema),
});

export const MarketStatsV2UpdateSchema: z.ZodType<MarketStatsV2Update> =
  z.object({
    symbols: z.array(z.string()).nonempty().optional(),
    stats: z.array(marketStatsV2DataSchema),
  });
