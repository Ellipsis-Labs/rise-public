import z from "zod";

/**
 * Individual fill record from the websocket fills channel.
 * Matches the FillRecord type from phoenix-api-types.
 */
export interface FillData {
  marketSymbol: string;
  baseQty: string;
  quoteQty: string;
  price: string;
  timestamp: bigint | string | number;
  transactionSignature: string;
  instructionType: string;
}

export const FillDataSchema: z.ZodType<FillData> = z.object({
  marketSymbol: z.string(),
  baseQty: z.string(),
  quoteQty: z.string(),
  price: z.string(),
  timestamp: z.union([z.number(), z.string(), z.bigint()]),
  transactionSignature: z.string(),
  instructionType: z.string(),
});

export interface FillUpdate {
  symbol: string;
  fill: FillData & { timestampMs: number };
}

export const FillUpdateSchema: z.ZodType<FillUpdate> = z.object({
  symbol: z.string(),
  fill: FillDataSchema.and(
    z.object({
      timestampMs: z.number(),
    })
  ),
});

export interface FillsMsg {
  channel: "fills";
  symbol: string;
  fills: FillData[];
}

export const FillsMsgSchema: z.ZodType<FillsMsg> = z.object({
  channel: z.literal("fills"),
  symbol: z.string(),
  fills: z.array(FillDataSchema),
});
