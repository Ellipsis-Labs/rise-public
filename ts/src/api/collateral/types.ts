import z from "zod";
import { numericBigint } from "@/ws/numericSchemas";

export interface SpotAssetConfig {
  isActive: boolean;
  perpAssetIndex?: number | null;
  maxPerTraderBalance: bigint;
  maxGlobalBalance: bigint;
  currGlobalBalance: bigint;
  minMarginDiscountBps: number;
  maxMarginDiscountBps: number;
  maxLiquidationDiscountBps: number;
  minLiquidationSlippageBps: number;
  maxLiquidationSize: bigint;
}

export interface CollateralAssetMetadata {
  assetIndex: number;
  symbol: string;
  decimals: number;
  spot?: SpotAssetConfig | null;
}

export interface CollateralAssetsResponse {
  assets: CollateralAssetMetadata[];
}

export const SpotAssetConfigSchema: z.ZodType<SpotAssetConfig> = z.object({
  isActive: z.boolean(),
  perpAssetIndex: z.number().int().nonnegative().nullable().optional(),
  maxPerTraderBalance: numericBigint("maxPerTraderBalance"),
  maxGlobalBalance: numericBigint("maxGlobalBalance"),
  currGlobalBalance: numericBigint("currGlobalBalance"),
  minMarginDiscountBps: z.number().int().nonnegative(),
  maxMarginDiscountBps: z.number().int().nonnegative(),
  maxLiquidationDiscountBps: z.number().int().nonnegative(),
  minLiquidationSlippageBps: z.number().int().nonnegative(),
  maxLiquidationSize: numericBigint("maxLiquidationSize"),
});

export const CollateralAssetMetadataSchema: z.ZodType<CollateralAssetMetadata> =
  z.object({
    assetIndex: z.number().int().nonnegative(),
    symbol: z.string(),
    decimals: z.number().int().nonnegative(),
    spot: SpotAssetConfigSchema.nullable().optional(),
  });

export const CollateralAssetsResponseSchema: z.ZodType<CollateralAssetsResponse> =
  z.object({ assets: z.array(CollateralAssetMetadataSchema) });

// Native-unit quantities are i64 on the wire; JSON parsing rounds anything
// past 2^53 - 1, so reject those instead of returning a wrong value.
const nativeAmountSchema = z.number().int().refine(Number.isSafeInteger, {
  message: "Expected safe integer",
});

// ---------------------------------------------------------------------------
// Collateral Totals Types
// ---------------------------------------------------------------------------

export interface CollateralFlowTotal {
  /** Native units of the asset, as a positive magnitude. */
  amount: number;
  /** USD value, each event valued at its event-time price. */
  value: number;
}

export interface CollateralAssetTotals {
  assetIndex: number;
  symbol: string;
  decimals: number;
  deposited: CollateralFlowTotal;
  withdrawn: CollateralFlowTotal;
}

/**
 * Lifetime deposit and withdrawal totals for a user across all subaccounts
 * and collateral assets. Transfers, swaps and liquidations are excluded.
 */
export interface CollateralTotalsResponse {
  /** USD; sum of `assets[].deposited.value`. */
  totalDeposited: number;
  /** USD; sum of `assets[].withdrawn.value`. */
  totalWithdrawn: number;
  /** Only assets with at least one deposit or withdrawal appear. */
  assets: CollateralAssetTotals[];
}

export const CollateralFlowTotalSchema: z.ZodType<CollateralFlowTotal> =
  z.object({
    amount: nativeAmountSchema.nonnegative(),
    value: z.number(),
  });

export const CollateralAssetTotalsSchema: z.ZodType<CollateralAssetTotals> =
  z.object({
    assetIndex: z.number().int().nonnegative(),
    symbol: z.string(),
    decimals: z.number().int().nonnegative(),
    deposited: CollateralFlowTotalSchema,
    withdrawn: CollateralFlowTotalSchema,
  });

export const CollateralTotalsResponseSchema: z.ZodType<CollateralTotalsResponse> =
  z.object({
    totalDeposited: z.number(),
    totalWithdrawn: z.number(),
    assets: z.array(CollateralAssetTotalsSchema),
  });

// ---------------------------------------------------------------------------
// Collateral History Types
// ---------------------------------------------------------------------------

export interface CollateralHistoryRequest {
  /** Trader PDA index for legacy authority-based collateral history calls. */
  pdaIndex?: number;
  /** Number of items to return (max 1000) */
  limit?: number;
  /** Cursor for fetching older events (base64-encoded) */
  nextCursor?: string;
  /** Cursor for fetching newer events (base64-encoded) */
  prevCursor?: string;
  /** Legacy cursor param (deprecated, use nextCursor) */
  cursor?: string;
}

export type CollateralEventType = "deposit" | "withdrawal" | "transfer";

export interface CollateralEvent {
  slot: number;
  slotIndex: number;
  eventIndex: number;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  eventType: CollateralEventType;
  amount: number;
  collateralAfter: number;
  timestamp: number;
}

export interface CollateralHistoryResponse {
  data: CollateralEvent[];
  nextCursor: string | null;
  prevCursor: string | null;
  hasMore: boolean;
}

// ---------------------------------------------------------------------------
// Zod Schemas
// ---------------------------------------------------------------------------

const toNumber = (value: unknown, fieldName: string): number => {
  if (typeof value === "number") return value;
  if (typeof value === "string") {
    if (value.trim().length > 0) {
      const parsed = Number(value);
      if (!Number.isNaN(parsed)) return parsed;
    }
    const dateParsed = Date.parse(value);
    if (!Number.isNaN(dateParsed)) return dateParsed;
  }
  throw new Error(`Invalid numeric value for ${fieldName}`);
};

const requireField = <T>(value: T | null | undefined, fieldName: string): T => {
  if (value === undefined || value === null)
    throw new Error(
      `Missing field ${fieldName} in collateral history response`
    );
  return value;
};

const parseEventType = (value: unknown): CollateralEventType => {
  if (typeof value === "string") {
    const normalized = value.toLowerCase();
    if (
      normalized === "deposit" ||
      normalized === "withdrawal" ||
      normalized === "transfer"
    ) {
      return normalized;
    }
  }
  throw new Error(`Invalid event type: ${String(value)}`);
};

const RawCollateralEventSchema = z
  .object({
    slot: z.union([z.number(), z.string()]),
    slotIndex: z.union([z.number(), z.string()]),
    eventIndex: z.union([z.number(), z.string()]),
    traderPdaIndex: z.union([z.number(), z.string()]),
    traderSubaccountIndex: z.union([z.number(), z.string()]),
    eventType: z.string(),
    amount: z.union([z.number(), z.string()]),
    collateralAfter: z.union([z.number(), z.string()]),
    timestamp: z.union([z.number(), z.string()]),
  })
  .loose();

export const CollateralEventSchema: z.ZodType<CollateralEvent> =
  RawCollateralEventSchema.transform((raw) => ({
    slot: toNumber(raw.slot, "collateralEvent.slot"),
    slotIndex: toNumber(raw.slotIndex, "collateralEvent.slotIndex"),
    eventIndex: toNumber(raw.eventIndex, "collateralEvent.eventIndex"),
    traderPdaIndex: toNumber(
      raw.traderPdaIndex,
      "collateralEvent.traderPdaIndex"
    ),
    traderSubaccountIndex: toNumber(
      raw.traderSubaccountIndex,
      "collateralEvent.traderSubaccountIndex"
    ),
    eventType: parseEventType(raw.eventType),
    amount: toNumber(raw.amount, "collateralEvent.amount"),
    collateralAfter: toNumber(
      raw.collateralAfter,
      "collateralEvent.collateralAfter"
    ),
    timestamp: toNumber(raw.timestamp, "collateralEvent.timestamp"),
  }));

const RawCollateralHistoryResponseSchema = z
  .object({
    data: z.array(RawCollateralEventSchema),
    nextCursor: z.string().nullable().optional(),
    prevCursor: z.string().nullable().optional(),
    hasMore: z.boolean(),
  })
  .loose();

export const CollateralHistoryResponseSchema: z.ZodType<CollateralHistoryResponse> =
  RawCollateralHistoryResponseSchema.transform((raw) => ({
    data: raw.data.map((event) => CollateralEventSchema.parse(event)),
    nextCursor: raw.nextCursor ?? null,
    prevCursor: raw.prevCursor ?? null,
    hasMore: requireField(raw.hasMore, "collateralHistory.hasMore"),
  }));

// ---------------------------------------------------------------------------
// Mixed (multi-asset) Collateral History Types
// ---------------------------------------------------------------------------

/** The server requires `limit` on the mixed history endpoints. */
export type CollateralHistoryV2Request = Omit<
  CollateralHistoryRequest,
  "limit"
> & {
  limit: number;
};

export type CollateralEventCategory =
  | "deposit"
  | "withdrawal"
  | "transfer"
  | "swap"
  | "liquidation";

export interface CollateralEventV2 {
  slot: number;
  slotIndex: number;
  eventIndex: number;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  assetIndex: number;
  symbol: string;
  category: CollateralEventCategory;
  /** Signed native units of the asset; outflows are negative. */
  amount: number;
  /** Balance after this event, native units of the asset. */
  balanceAfter: number;
  /** Uncounted native units beyond `amount`; zero for quote events. */
  excess: number;
  signature?: string;
  /** Unix milliseconds. */
  timestamp: number;
}

export interface CollateralHistoryV2Response {
  /** Newest first. */
  data: CollateralEventV2[];
  nextCursor: string | null;
  prevCursor: string | null;
  hasMore: boolean;
}

export const CollateralEventV2Schema: z.ZodType<CollateralEventV2> = z.object({
  slot: z.number().int(),
  slotIndex: z.number().int(),
  eventIndex: z.number().int(),
  traderPdaIndex: z.number().int(),
  traderSubaccountIndex: z.number().int(),
  assetIndex: z.number().int().nonnegative(),
  symbol: z.string(),
  category: z.enum([
    "deposit",
    "withdrawal",
    "transfer",
    "swap",
    "liquidation",
  ]),
  amount: nativeAmountSchema,
  balanceAfter: nativeAmountSchema,
  excess: nativeAmountSchema,
  signature: z.string().optional(),
  timestamp: z
    .union([z.number(), z.string()])
    .transform((value) => toNumber(value, "collateralEventV2.timestamp")),
});

export const CollateralHistoryV2ResponseSchema: z.ZodType<CollateralHistoryV2Response> =
  z
    .object({
      data: z.array(CollateralEventV2Schema),
      nextCursor: z.string().nullable().optional(),
      prevCursor: z.string().nullable().optional(),
      hasMore: z.boolean(),
    })
    .transform((raw) => ({
      data: raw.data,
      nextCursor: raw.nextCursor ?? null,
      prevCursor: raw.prevCursor ?? null,
      hasMore: raw.hasMore,
    }));
