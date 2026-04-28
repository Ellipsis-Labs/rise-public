import z from "zod";

export interface MarketUpdate {
  symbol: string;
  dayNtlVlm: number;
  prevDayPx: number;
  markPx: number;
  midPx?: number | null;
  funding: number;
  openInterest: number;
  oraclePx: number;
}

export interface MarketMsg extends MarketUpdate {
  channel: "market";
}

export const MarketUpdateSchema: z.ZodType<MarketUpdate> = z.object({
  symbol: z.string(),
  dayNtlVlm: z.number(),
  prevDayPx: z.number(),
  markPx: z.number(),
  midPx: z.number().nullable().optional(),
  funding: z.number(),
  openInterest: z.number(),
  oraclePx: z.number(),
});

export const MarketMsgSchema: z.ZodType<MarketMsg> = z.object({
  channel: z.literal("market"),
  symbol: z.string(),
  dayNtlVlm: z.number(),
  prevDayPx: z.number(),
  markPx: z.number(),
  midPx: z.number().nullable().optional(),
  funding: z.number(),
  openInterest: z.number(),
  oraclePx: z.number(),
});
