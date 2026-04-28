import z from "zod";
import { numericBigint } from "@/ws/numericSchemas";

export interface MarkPriceUpdate {
  symbol: string;
  slot: bigint;
  slotIndex: number;
  markPrice: number;
}

export interface MarkPriceMsg {
  channel: "markPrice";
  symbol: string;
  slot: bigint;
  slotIndex: number;
  markPrice: number;
}

export const MarkPriceMsgSchema: z.ZodType<MarkPriceMsg> = z.object({
  channel: z.literal("markPrice"),
  symbol: z.string(),
  slot: numericBigint("slot"),
  slotIndex: z.number(),
  markPrice: z.number(),
});
