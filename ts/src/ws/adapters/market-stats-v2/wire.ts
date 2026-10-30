import z from "zod";
import { numericBigint } from "@/ws/numericSchemas";
import {
  type MarketStats,
  type MarketStatsWireData,
  marketStatsSchema,
  marketStatsWireDataSchema,
} from "../market-stats/shared";
import type { MarketStatsUpdate } from "../market-stats/wire";

export interface MarketStatsV2WireData extends MarketStatsWireData {
  displayName?: string;
  midPrice?: number;
  /**
   * Oldest slot in the newest quorum of oracle updates behind `oraclePrice`;
   * age is the batch `slot` minus this. Omitted until the market has a full
   * oracle quorum.
   */
  oraclePriceSlot?: bigint;
  /**
   * Last slot `markPrice` was updated; age is the batch `slot` minus this.
   * Omitted once the market is closed and `markPrice` is final.
   */
  markPriceSlot?: bigint;
}

export interface MarketStatsV2Data extends MarketStatsUpdate {
  displayName?: string;
  stats: MarketStats & {
    midPrice?: number;
    oraclePriceSlot?: bigint;
    markPriceSlot?: bigint;
  };
}

export interface MarketStatsV2Msg {
  channel: "marketStatsV2";
  symbols?: string[];
  /** Latest chain slot the server had processed when this batch was sent. */
  slot?: bigint;
  slotIndex?: number;
  stats: MarketStatsV2WireData[];
}

export interface MarketStatsV2Update {
  symbols?: string[];
  /** Latest chain slot the server had processed when this batch was sent. */
  slot?: bigint;
  slotIndex?: number;
  stats: MarketStatsV2Data[];
}

export const MarketStatsV2WireDataSchema: z.ZodType<MarketStatsV2WireData> =
  marketStatsWireDataSchema.extend({
    displayName: z.string().optional(),
    midPrice: z.number().optional(),
    oraclePriceSlot: numericBigint("oraclePriceSlot").optional(),
    markPriceSlot: numericBigint("markPriceSlot").optional(),
  });

const marketStatsV2DataSchema: z.ZodType<MarketStatsV2Data> = z.object({
  symbol: z.string(),
  displayName: z.string().optional(),
  stats: marketStatsSchema.extend({
    midPrice: z.number().optional(),
    oraclePriceSlot: numericBigint("oraclePriceSlot").optional(),
    markPriceSlot: numericBigint("markPriceSlot").optional(),
  }),
});

export const MarketStatsV2MsgSchema: z.ZodType<MarketStatsV2Msg> = z.object({
  channel: z.literal("marketStatsV2"),
  symbols: z.array(z.string()).nonempty().optional(),
  slot: numericBigint("slot").optional(),
  slotIndex: z.number().optional(),
  stats: z.array(MarketStatsV2WireDataSchema),
});

export const MarketStatsV2UpdateSchema: z.ZodType<MarketStatsV2Update> =
  z.object({
    symbols: z.array(z.string()).nonempty().optional(),
    slot: numericBigint("slot").optional(),
    slotIndex: z.number().optional(),
    stats: z.array(marketStatsV2DataSchema),
  });
