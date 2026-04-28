import z from "zod";

export interface FundingRateUpdate {
  symbol: string;
  funding: number;
}

export interface FundingRateMsg extends FundingRateUpdate {
  channel: "fundingRate";
}

export const FundingRateUpdateSchema: z.ZodType<FundingRateUpdate> = z.object({
  symbol: z.string(),
  funding: z.number(),
});

export const FundingRateMsgSchema: z.ZodType<FundingRateMsg> = z.object({
  channel: z.literal("fundingRate"),
  symbol: z.string(),
  funding: z.number(),
});
