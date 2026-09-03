import z from "zod";
import {
  type MarketStats,
  type MarketStatsWireData,
  marketStatsSchema,
  marketStatsWireDataSchema,
} from "./shared";

export type { MarketStats } from "./shared";

export interface MarketStatsUpdate {
  symbol: string;
  stats: MarketStats;
}

export const MarketStatsUpdateSchema: z.ZodType<MarketStatsUpdate> = z.object({
  symbol: z.string(),
  stats: marketStatsSchema,
});

export interface MarketStatsMsg extends MarketStatsWireData {
  channel: "marketStats";
}

export const MarketStatsMsgSchema: z.ZodType<MarketStatsMsg> =
  marketStatsWireDataSchema.extend({
    channel: z.literal("marketStats"),
  });
