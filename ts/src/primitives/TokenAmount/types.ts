import z from "zod";

export interface TokenAmount {
  value: number;
  decimals: number;
  ui: string;
}

export const TokenAmountSchema: z.ZodType<TokenAmount> = z.object({
  value: z.number(),
  decimals: z.number(),
  ui: z.string(),
});
