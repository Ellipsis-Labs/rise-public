import z from "zod";
import {
  type ApiInstructionResponse,
  ApiInstructionResponseSchema,
} from "@/api/orders/types";

/** SOL, quote collateral, or SPL collateral identified by its mint. */
export type SwapAsset = "sol" | "quote" | { spot: string };

export const SwapAssetSchema: z.ZodType<SwapAsset> = z.union([
  z.literal("sol"),
  z.literal("quote"),
  z.object({ spot: z.string() }),
]);

const unsignedInteger = z.number().int().min(0).max(Number.MAX_SAFE_INTEGER);
const accountIndex = z.number().int().min(0).max(255);

/** Exact-in swap request. Supply exactly one of amountAtoms or quantity. */
export interface BuildSwapTransactionRequest {
  authority: string;
  positionAuthority?: string;
  assetIn: SwapAsset;
  assetOut: SwapAsset;
  /** Native units of assetIn; must be a safe JavaScript integer. */
  amountAtoms?: number;
  /** Whole units of assetIn, converted using its mint decimals by the API. */
  quantity?: number;
  traderPdaIndex?: number;
  /** Defaults to 0 (cross-margin). Requires API support for subaccounts. */
  traderSubaccountIndex?: number;
  feePayer?: string;
  slippageBps?: number;
  venue?: string;
  /** Beta quote-to-SOL collateral debit, separate from the venue amount. */
  collateralAmountAtoms?: number;
  computeUnitPriceMicroLamports?: number;
  /** 0 uses lookup tables; 1 uses an inline v1 message. Defaults to 0. */
  transactionVersion?: 0 | 1;
}

export const BuildSwapTransactionRequestSchema: z.ZodType<BuildSwapTransactionRequest> =
  z
    .object({
      authority: z.string(),
      positionAuthority: z.string().optional(),
      assetIn: SwapAssetSchema,
      assetOut: SwapAssetSchema,
      amountAtoms: unsignedInteger.positive().optional(),
      quantity: z.number().positive().optional(),
      traderPdaIndex: accountIndex.optional(),
      traderSubaccountIndex: accountIndex.optional(),
      feePayer: z.string().optional(),
      slippageBps: z.number().int().min(0).max(65535).optional(),
      venue: z.string().optional(),
      collateralAmountAtoms: unsignedInteger.positive().optional(),
      computeUnitPriceMicroLamports: unsignedInteger.optional(),
      transactionVersion: z.union([z.literal(0), z.literal(1)]).optional(),
    })
    .refine(
      (request) =>
        (request.amountAtoms !== undefined) !==
        (request.quantity !== undefined),
      { message: "Supply exactly one of amountAtoms or quantity" }
    );

/** Unsigned transaction, its venue quote, and optional sponsorship proof. */
export interface BuildSwapTransactionResponse {
  /** Base64 wire bytes. Sign verbatim to preserve the sponsorship proof. */
  transaction: string;
  transactionVersion: 0 | 1;
  instructions: ApiInstructionResponse[];
  addressLookupTableAddresses: string[];
  venue: string;
  amountInAtoms: number;
  swapNativeAmountInAtoms: number;
  expectedAmountOutAtoms: number;
  minAmountOutAtoms: number;
  priceImpactBps: number;
  contextSlot: number;
  computeUnitLimit: number;
  blockhash: string;
  lastValidBlockHeight: number;
  sponsorshipAttestation?: string | null;
  attestationExpiresAt?: number | null;
}

export const BuildSwapTransactionResponseSchema: z.ZodType<BuildSwapTransactionResponse> =
  z.object({
    transaction: z.string(),
    transactionVersion: z.union([z.literal(0), z.literal(1)]),
    instructions: z.array(ApiInstructionResponseSchema),
    addressLookupTableAddresses: z.array(z.string()),
    venue: z.string(),
    amountInAtoms: unsignedInteger,
    swapNativeAmountInAtoms: unsignedInteger,
    expectedAmountOutAtoms: unsignedInteger,
    minAmountOutAtoms: unsignedInteger,
    priceImpactBps: z.number().int().min(0).max(4294967295),
    contextSlot: unsignedInteger,
    computeUnitLimit: z.number().int().min(0).max(4294967295),
    blockhash: z.string(),
    lastValidBlockHeight: unsignedInteger,
    sponsorshipAttestation: z.string().nullable().optional(),
    attestationExpiresAt: z.number().int().nullable().optional(),
  });
