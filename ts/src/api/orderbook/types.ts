import z from "zod";
import { type APISpline, SplineSchema } from "@/api/splines/types";

export interface OrderbookView {
  slot: number;
  symbol: string;
  bids: [number, number][];
  asks: [number, number][];
  mid?: number | null;
  splines?: APISpline[];
}

export const OrderbookViewSchema: z.ZodType<OrderbookView> = z.object({
  slot: z.number(),
  symbol: z.string(),
  bids: z.array(z.tuple([z.number(), z.number()])),
  asks: z.array(z.tuple([z.number(), z.number()])),
  mid: z.number().nullable().optional(),
  splines: z.array(SplineSchema).optional(),
});

export interface OrderbookQueryParams {
  includeSplines?: boolean;
  bypassExecutionBand?: boolean;
}

export const OrderbookQueryParamsSchema: z.ZodType<OrderbookQueryParams> =
  z.object({
    includeSplines: z.boolean().optional(),
    bypassExecutionBand: z.boolean().optional(),
  });

export type OrderbookLevelTuple = [number, number, number];

export interface OrderbookResponse {
  bids: OrderbookLevelTuple[];
  asks: OrderbookLevelTuple[];
  mid: number;
  symbol: string;
  timestamp: number;
}

export const OrderbookResponseSchema: z.ZodType<OrderbookResponse> = z.object({
  bids: z.array(z.tuple([z.number(), z.number(), z.number()])),
  asks: z.array(z.tuple([z.number(), z.number(), z.number()])),
  mid: z.number(),
  symbol: z.string(),
  timestamp: z.number(),
});
