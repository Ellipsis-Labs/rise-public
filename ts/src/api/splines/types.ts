import z from "zod";
import { TokenAmountSchema, type TokenAmount } from "@/primitives/TokenAmount";

export interface SplineRegion {
  start: number;
  end: number;
  density: number;
  totalSize: TokenAmount;
  filledSize: TokenAmount;
}

export const SplineRegionSchema: z.ZodType<SplineRegion> = z.object({
  start: z.number(),
  end: z.number(),
  density: z.number(),
  totalSize: TokenAmountSchema,
  filledSize: TokenAmountSchema,
});

export interface APISpline {
  traderKey: string;
  midPrice: number;
  bidFilledAmount: TokenAmount;
  askFilledAmount: TokenAmount;
  bidRegions: SplineRegion[];
  askRegions: SplineRegion[];
}

export const SplineSchema: z.ZodType<APISpline> = z.object({
  traderKey: z.string(),
  midPrice: z.number(),
  bidFilledAmount: TokenAmountSchema,
  askFilledAmount: TokenAmountSchema,
  bidRegions: z.array(SplineRegionSchema),
  askRegions: z.array(SplineRegionSchema),
});

export interface SplinesView {
  slot: number;
  symbol: string;
  splines: APISpline[];
}

export const SplinesViewSchema: z.ZodType<SplinesView> = z.object({
  slot: z.number(),
  symbol: z.string(),
  splines: z.array(SplineSchema),
});
