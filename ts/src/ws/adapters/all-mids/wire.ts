import z from "zod";
import { numericBigint } from "@/ws/numericSchemas";

export interface AllMidsUpdate {
  mids: Record<string, number>;
  slot: bigint;
  slotIndex: number;
}

export interface AllMidsMsg extends AllMidsUpdate {
  channel: "allMids";
}

export const AllMidsUpdateSchema: z.ZodType<AllMidsUpdate> = z.object({
  mids: z.record(z.string(), z.number()),
  slot: numericBigint("slot"),
  slotIndex: z.number(),
});

export const AllMidsMsgSchema: z.ZodType<AllMidsMsg> = z.object({
  channel: z.literal("allMids"),
  mids: z.record(z.string(), z.number()),
  slot: numericBigint("slot"),
  slotIndex: z.number(),
});
