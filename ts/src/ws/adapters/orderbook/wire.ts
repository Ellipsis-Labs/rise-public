import z from "zod";
import { L2OrderbookSchema, type L2Orderbook } from "@/api/markets/types";

export interface OrderbookSnapshotUpdate {
  symbol: string;
  orderbook: L2Orderbook;
  bypassExecutionBand?: boolean;
}

export const OrderbookSnapshotUpdateSchema: z.ZodType<OrderbookSnapshotUpdate> =
  z.object({
    symbol: z.string(),
    orderbook: L2OrderbookSchema,
    bypassExecutionBand: z.boolean().optional(),
  });

export interface OrderbookMsg {
  channel: "orderbook";
  symbol: string;
  orderbook: L2Orderbook;
  bypassExecutionBand?: boolean;
}

export const OrderbookMsgSchema: z.ZodType<OrderbookMsg> = z.object({
  channel: z.literal("orderbook"),
  symbol: z.string(),
  orderbook: L2OrderbookSchema,
  bypassExecutionBand: z.boolean().optional(),
});
