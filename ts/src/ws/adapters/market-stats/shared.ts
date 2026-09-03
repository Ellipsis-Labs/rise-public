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

export interface MarketStatsWireData extends MarketStats {
  symbol: string;
}

type MarketStatsZodShape = {
  [Field in keyof MarketStats]: z.ZodType<MarketStats[Field]>;
};

const marketStatsShape: MarketStatsZodShape = {
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
};

export const marketStatsSchema: z.ZodObject<typeof marketStatsShape> =
  z.object(marketStatsShape);

type MarketStatsWireDataZodShape = MarketStatsZodShape & {
  symbol: z.ZodType<string>;
};

const marketStatsWireDataShape: MarketStatsWireDataZodShape = {
  symbol: z.string(),
  ...marketStatsShape,
};

export const marketStatsWireDataSchema: z.ZodObject<
  typeof marketStatsWireDataShape
> = z.object(marketStatsWireDataShape);
