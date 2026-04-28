import z from "zod";
import { numericBigint } from "@/ws/numericSchemas";

export interface L2BookUpdate {
  market: string;
  ts: number;
  slot: bigint;
  bids: Array<[price: number, size: number]>;
  asks: Array<[price: number, size: number]>;
  bypassExecutionBand?: boolean;
}

export type OrderbookUpdate = L2BookUpdate;

export const L2BookUpdateSchema: z.ZodType<L2BookUpdate> = z.object({
  market: z.string(),
  ts: z.number(),
  slot: numericBigint("slot"),
  bids: z.array(z.tuple([z.number(), z.number()])),
  asks: z.array(z.tuple([z.number(), z.number()])),
  bypassExecutionBand: z.boolean().optional(),
});

export const OrderbookUpdateSchema: z.ZodType<OrderbookUpdate> =
  L2BookUpdateSchema;

export interface L2BookMsg {
  channel: "l2Book";
  coin: string;
  timestamp: bigint;
  slot: bigint;
  bids: Array<[price: number, size: number]>;
  asks: Array<[price: number, size: number]>;
  bypassExecutionBand?: boolean;
}

export const L2BookMsgSchema: z.ZodType<L2BookMsg> = z.object({
  channel: z.literal("l2Book"),
  coin: z.string(),
  timestamp: numericBigint("timestamp"),
  slot: numericBigint("slot"),
  bids: z.array(z.tuple([z.number(), z.number()])),
  asks: z.array(z.tuple([z.number(), z.number()])),
  bypassExecutionBand: z.boolean().optional(),
});
