import z from "zod";
import { numericBigint } from "@/ws/numericSchemas";

export interface MarketStats {
  timestamp: bigint;
  openInterest: number;
  markPrice: number;
  oraclePrice: number;
  prevDayMarkPrice: number;
  dayVolumeUsd: number;
  dayVolumeBase: number;
  currentFundingRate: number;
  eightHourFundingRate: number;
  annualizedFundingRate: number;
}

export interface MarketStatsUpdate {
  symbol: string;
  stats: MarketStats;
}

export const MarketStatsUpdateSchema: z.ZodType<MarketStatsUpdate> = z.object({
  symbol: z.string(),
  stats: z.object({
    timestamp: numericBigint("timestamp"),
    openInterest: z.number(),
    markPrice: z.number(),
    oraclePrice: z.number(),
    prevDayMarkPrice: z.number(),
    dayVolumeUsd: z.number(),
    dayVolumeBase: z.number(),
    currentFundingRate: z.number(),
    eightHourFundingRate: z.number(),
    annualizedFundingRate: z.number(),
  }),
});

export interface MarketStatsMsg {
  channel: "marketStats";
  symbol: string;
  timestamp: bigint;
  openInterest: number;
  markPrice: number;
  oraclePrice: number;
  prevDayMarkPrice: number;
  dayVolumeUsd: number;
  dayVolumeBase: number;
  currentFundingRate: number;
  eightHourFundingRate: number;
  annualizedFundingRate: number;
}

export const MarketStatsMsgSchema: z.ZodType<MarketStatsMsg> = z.object({
  channel: z.literal("marketStats"),
  symbol: z.string(),
  timestamp: numericBigint("timestamp"),
  openInterest: z.number(),
  markPrice: z.number(),
  oraclePrice: z.number(),
  prevDayMarkPrice: z.number(),
  dayVolumeUsd: z.number(),
  dayVolumeBase: z.number(),
  currentFundingRate: z.number(),
  eightHourFundingRate: z.number(),
  annualizedFundingRate: z.number(),
});
