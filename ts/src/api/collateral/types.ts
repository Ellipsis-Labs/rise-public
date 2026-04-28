import z from "zod";

// ---------------------------------------------------------------------------
// Collateral History Types
// ---------------------------------------------------------------------------

export interface CollateralHistoryRequest {
  /** Trader PDA index (default 0) */
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
