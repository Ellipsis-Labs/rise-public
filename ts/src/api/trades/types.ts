import z from "zod";

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
    throw new Error(`Missing field ${fieldName} in history response`);
  return value;
};

export interface TradeHistoryRequest {
  pdaIndex?: number;
  marketSymbol?: string;
  limit?: number;
  cursor?: string;
  privyId?: string;
}

export interface MarketTradeHistoryRequest {
  limit?: number;
  cursor?: string;
  startTime?: number;
  endTime?: number;
}

// ---------------------------------------------------------------------------
// Market Fill Record types for /market/{symbol}/fills
// ---------------------------------------------------------------------------

export interface MarketFillRecord {
  marketSymbol: string;
  baseQty: string;
  quoteQty: string;
  price: string;
  timestamp: number;
  transactionSignature: string;
  instructionType: string;
}

const RawMarketFillRecordSchema = z
  .object({
    marketSymbol: z.string(),
    baseQty: z.string(),
    quoteQty: z.string(),
    price: z.string(),
    timestamp: z.union([z.number(), z.string()]),
    transactionSignature: z.string(),
    instructionType: z.string(),
  })
  .loose();

export const MarketFillRecordSchema: z.ZodType<MarketFillRecord> =
  RawMarketFillRecordSchema.transform((raw) => ({
    marketSymbol: requireField(
      raw.marketSymbol,
      "marketFillRecord.marketSymbol"
    ),
    baseQty: requireField(raw.baseQty, "marketFillRecord.baseQty"),
    quoteQty: requireField(raw.quoteQty, "marketFillRecord.quoteQty"),
    price: requireField(raw.price, "marketFillRecord.price"),
    timestamp: toNumber(raw.timestamp, "marketFillRecord.timestamp"),
    transactionSignature: requireField(
      raw.transactionSignature,
      "marketFillRecord.transactionSignature"
    ),
    instructionType: requireField(
      raw.instructionType,
      "marketFillRecord.instructionType"
    ),
  }));

export interface MarketFillsResponse {
  data: MarketFillRecord[];
  prevCursor?: string | null;
  nextCursor: string | null;
  hasMore: boolean;
}

const RawMarketFillsResponseSchema = z
  .object({
    data: z.array(RawMarketFillRecordSchema),
    prevCursor: z.string().nullable().optional(),
    nextCursor: z.string().nullable().optional(),
    hasMore: z.boolean(),
  })
  .loose();

export const MarketFillsResponseSchema: z.ZodType<MarketFillsResponse> =
  RawMarketFillsResponseSchema.transform((raw) => ({
    data: raw.data.map((r) => MarketFillRecordSchema.parse(r)),
    prevCursor: raw.prevCursor ?? null,
    nextCursor: raw.nextCursor ?? null,
    hasMore: requireField(raw.hasMore, "marketFillsResponse.hasMore"),
  }));

// ---------------------------------------------------------------------------
// Trade History Record types for /trader/{authority}/trades-history
// ---------------------------------------------------------------------------

export interface FillRecord {
  userId: number;
  traderId: number;
  traderPdaIndex: number;
  subaccountIndex: number;
  marketSymbol: string;
  signature: string | null;
  fillId: string | null;
  timestamp: number;
  slot: number;
  slotIndex: number;
  eventIndex: number;
  instructionIndex: number;
  instructionType: string;
  baseLotsBefore: string;
  baseLotsAfter: string;
  baseLotsDelta: string;
  virtualQuoteLotsBefore: string;
  virtualQuoteLotsAfter: string;
  virtualQuoteLotsDelta: string;
  price: string;
  realizedPnl: string;
  fees: string;
  liquidity: "maker" | "taker";
  orderSequenceNumber: number | null;
  splineSequenceNumber: number | null;
  tradeType: "limit" | "market" | "liquidation" | "adl";
}

const RawFillRecordSchema = z
  .object({
    userId: z.union([z.number(), z.string()]),
    traderId: z.union([z.number(), z.string()]),
    traderPdaIndex: z.union([z.number(), z.string()]),
    subaccountIndex: z.union([z.number(), z.string()]),
    marketSymbol: z.string(),
    signature: z.string().nullable().optional(),
    fillId: z.string().nullable().optional(),
    timestamp: z.union([z.number(), z.string()]),
    slot: z.union([z.number(), z.string()]),
    slotIndex: z.union([z.number(), z.string()]),
    eventIndex: z.union([z.number(), z.string()]),
    instructionIndex: z.union([z.number(), z.string()]),
    instructionType: z.string(),
    baseLotsBefore: z.string(),
    baseLotsAfter: z.string(),
    baseLotsDelta: z.string(),
    virtualQuoteLotsBefore: z.string(),
    virtualQuoteLotsAfter: z.string(),
    virtualQuoteLotsDelta: z.string(),
    price: z.string(),
    realizedPnl: z.string(),
    fees: z.string(),
    liquidity: z.union([z.enum(["maker", "taker"]), z.string()]),
    orderSequenceNumber: z
      .union([z.number(), z.string()])
      .nullable()
      .optional(),
    splineSequenceNumber: z
      .union([z.number(), z.string()])
      .nullable()
      .optional(),
    tradeType: z.enum(["limit", "market", "liquidation", "adl"]),
  })
  .loose();

export const FillRecordSchema: z.ZodType<FillRecord> =
  RawFillRecordSchema.transform((raw) => ({
    userId: toNumber(raw.userId, "fillRecord.userId"),
    traderId: toNumber(raw.traderId, "fillRecord.traderId"),
    traderPdaIndex: toNumber(raw.traderPdaIndex, "fillRecord.traderPdaIndex"),
    subaccountIndex: toNumber(
      raw.subaccountIndex,
      "fillRecord.subaccountIndex"
    ),
    marketSymbol: requireField(raw.marketSymbol, "fillRecord.marketSymbol"),
    signature: raw.signature ?? null,
    fillId: raw.fillId ?? null,
    timestamp: toNumber(raw.timestamp, "fillRecord.timestamp"),
    slot: toNumber(raw.slot, "fillRecord.slot"),
    slotIndex: toNumber(raw.slotIndex, "fillRecord.slotIndex"),
    eventIndex: toNumber(raw.eventIndex, "fillRecord.eventIndex"),
    instructionIndex: toNumber(
      raw.instructionIndex,
      "fillRecord.instructionIndex"
    ),
    instructionType: requireField(
      raw.instructionType,
      "fillRecord.instructionType"
    ),
    baseLotsBefore: requireField(
      raw.baseLotsBefore,
      "fillRecord.baseLotsBefore"
    ),
    baseLotsAfter: requireField(raw.baseLotsAfter, "fillRecord.baseLotsAfter"),
    baseLotsDelta: requireField(raw.baseLotsDelta, "fillRecord.baseLotsDelta"),
    virtualQuoteLotsBefore: requireField(
      raw.virtualQuoteLotsBefore,
      "fillRecord.virtualQuoteLotsBefore"
    ),
    virtualQuoteLotsAfter: requireField(
      raw.virtualQuoteLotsAfter,
      "fillRecord.virtualQuoteLotsAfter"
    ),
    virtualQuoteLotsDelta: requireField(
      raw.virtualQuoteLotsDelta,
      "fillRecord.virtualQuoteLotsDelta"
    ),
    price: requireField(raw.price, "fillRecord.price"),
    realizedPnl: requireField(raw.realizedPnl, "fillRecord.realizedPnl"),
    fees: requireField(raw.fees, "fillRecord.fees"),
    liquidity:
      raw.liquidity === "taker" || raw.liquidity?.toLowerCase() === "taker"
        ? "taker"
        : "maker",
    orderSequenceNumber:
      raw.orderSequenceNumber === null || raw.orderSequenceNumber === undefined
        ? null
        : toNumber(raw.orderSequenceNumber, "fillRecord.orderSequenceNumber"),
    splineSequenceNumber:
      raw.splineSequenceNumber === null ||
      raw.splineSequenceNumber === undefined
        ? null
        : toNumber(raw.splineSequenceNumber, "fillRecord.splineSequenceNumber"),
    tradeType: requireField(raw.tradeType, "fillRecord.tradeType"),
  }));

export interface FillsResponse {
  data: FillRecord[];
  prevCursor?: string | null;
  nextCursor: string | null;
  hasMore: boolean;
}

const RawFillsResponseSchema = z
  .object({
    data: z.array(RawFillRecordSchema),
    prevCursor: z.string().nullable().optional(),
    nextCursor: z.string().nullable().optional(),
    hasMore: z.boolean(),
  })
  .loose();

export const FillsResponseSchema: z.ZodType<FillsResponse> =
  RawFillsResponseSchema.transform((raw) => ({
    data: raw.data.map((r) => FillRecordSchema.parse(r)),
    prevCursor: raw.prevCursor ?? null,
    nextCursor: raw.nextCursor ?? null,
    hasMore: requireField(raw.hasMore, "fillsResponse.hasMore"),
  }));

// ---------------------------------------------------------------------------
// Trade History V2 types for /traders/{pubkey}/trades_v2
// ---------------------------------------------------------------------------

export interface TradeHistoryV2Item {
  userId: number;
  traderId: number;
  traderPdaIndex: number;
  subaccountIndex: number;
  marketSymbol: string;
  signature: string | null;
  fillId: string | null;
  timestamp: number;
  slot: number;
  slotIndex: number;
  eventIndex: number;
  instructionIndex: number;
  instructionType: string;
  baseLotsBefore: string;
  baseLotsAfter: string;
  baseLotsDelta: string;
  virtualQuoteLotsBefore: string;
  virtualQuoteLotsAfter: string;
  virtualQuoteLotsDelta: string;
  price: string;
  realizedPnl: string;
  fees: string;
  liquidity: "maker" | "taker";
  orderSequenceNumber: number | null;
  splineSequenceNumber: number | null;
  tradeType: "limit" | "market" | "liquidation" | "adl";
}

const RawTradeHistoryV2ItemSchema = z
  .object({
    userId: z.union([z.number(), z.string()]),
    traderId: z.union([z.number(), z.string()]),
    traderPdaIndex: z.union([z.number(), z.string()]),
    subaccountIndex: z.union([z.number(), z.string()]),
    marketSymbol: z.string(),
    signature: z.string().nullable().optional(),
    fillId: z.string().nullable().optional(),
    timestamp: z.union([z.number(), z.string()]),
    slot: z.union([z.number(), z.string()]),
    slotIndex: z.union([z.number(), z.string()]),
    eventIndex: z.union([z.number(), z.string()]),
    instructionIndex: z.union([z.number(), z.string()]),
    instructionType: z.string(),
    baseLotsBefore: z.string(),
    baseLotsAfter: z.string(),
    baseLotsDelta: z.string(),
    virtualQuoteLotsBefore: z.string(),
    virtualQuoteLotsAfter: z.string(),
    virtualQuoteLotsDelta: z.string(),
    price: z.string(),
    realizedPnl: z.string(),
    fees: z.string(),
    liquidity: z.union([z.enum(["maker", "taker"]), z.string()]),
    orderSequenceNumber: z
      .union([z.number(), z.string()])
      .nullable()
      .optional(),
    splineSequenceNumber: z
      .union([z.number(), z.string()])
      .nullable()
      .optional(),
    tradeType: z.enum(["limit", "market", "liquidation", "adl"]),
  })
  .loose();

export const TradeHistoryV2ItemSchema: z.ZodType<TradeHistoryV2Item> =
  RawTradeHistoryV2ItemSchema.transform((raw) => ({
    userId: toNumber(raw.userId, "tradeHistoryV2Item.userId"),
    traderId: toNumber(raw.traderId, "tradeHistoryV2Item.traderId"),
    traderPdaIndex: toNumber(
      raw.traderPdaIndex,
      "tradeHistoryV2Item.traderPdaIndex"
    ),
    subaccountIndex: toNumber(
      raw.subaccountIndex,
      "tradeHistoryV2Item.subaccountIndex"
    ),
    marketSymbol: requireField(
      raw.marketSymbol,
      "tradeHistoryV2Item.marketSymbol"
    ),
    signature: raw.signature ?? null,
    fillId: raw.fillId ?? null,
    timestamp: toNumber(raw.timestamp, "tradeHistoryV2Item.timestamp"),
    slot: toNumber(raw.slot, "tradeHistoryV2Item.slot"),
    slotIndex: toNumber(raw.slotIndex, "tradeHistoryV2Item.slotIndex"),
    eventIndex: toNumber(raw.eventIndex, "tradeHistoryV2Item.eventIndex"),
    instructionIndex: toNumber(
      raw.instructionIndex,
      "tradeHistoryV2Item.instructionIndex"
    ),
    instructionType: requireField(
      raw.instructionType,
      "tradeHistoryV2Item.instructionType"
    ),
    baseLotsBefore: requireField(
      raw.baseLotsBefore,
      "tradeHistoryV2Item.baseLotsBefore"
    ),
    baseLotsAfter: requireField(
      raw.baseLotsAfter,
      "tradeHistoryV2Item.baseLotsAfter"
    ),
    baseLotsDelta: requireField(
      raw.baseLotsDelta,
      "tradeHistoryV2Item.baseLotsDelta"
    ),
    virtualQuoteLotsBefore: requireField(
      raw.virtualQuoteLotsBefore,
      "tradeHistoryV2Item.virtualQuoteLotsBefore"
    ),
    virtualQuoteLotsAfter: requireField(
      raw.virtualQuoteLotsAfter,
      "tradeHistoryV2Item.virtualQuoteLotsAfter"
    ),
    virtualQuoteLotsDelta: requireField(
      raw.virtualQuoteLotsDelta,
      "tradeHistoryV2Item.virtualQuoteLotsDelta"
    ),
    price: requireField(raw.price, "tradeHistoryV2Item.price"),
    realizedPnl: requireField(
      raw.realizedPnl,
      "tradeHistoryV2Item.realizedPnl"
    ),
    fees: requireField(raw.fees, "tradeHistoryV2Item.fees"),
    liquidity:
      raw.liquidity === "taker" || raw.liquidity?.toLowerCase() === "taker"
        ? "taker"
        : "maker",
    orderSequenceNumber:
      raw.orderSequenceNumber === null || raw.orderSequenceNumber === undefined
        ? null
        : toNumber(
            raw.orderSequenceNumber,
            "tradeHistoryV2Item.orderSequenceNumber"
          ),
    splineSequenceNumber:
      raw.splineSequenceNumber === null ||
      raw.splineSequenceNumber === undefined
        ? null
        : toNumber(
            raw.splineSequenceNumber,
            "tradeHistoryV2Item.splineSequenceNumber"
          ),
    tradeType: requireField(raw.tradeType, "tradeHistoryV2Item.tradeType"),
  }));

export interface TradeHistoryV2Response {
  data: TradeHistoryV2Item[];
  prevCursor?: string | null;
  nextCursor: string | null;
  hasMore: boolean;
}

const RawTradeHistoryV2ResponseSchema = z
  .object({
    data: z.array(RawTradeHistoryV2ItemSchema),
    prevCursor: z.string().nullable().optional(),
    nextCursor: z.string().nullable().optional(),
    hasMore: z.boolean(),
  })
  .loose();

export const TradeHistoryV2ResponseSchema: z.ZodType<TradeHistoryV2Response> =
  RawTradeHistoryV2ResponseSchema.transform((raw) => ({
    data: raw.data.map((item) => TradeHistoryV2ItemSchema.parse(item)),
    prevCursor: raw.prevCursor ?? null,
    nextCursor: raw.nextCursor ?? null,
    hasMore: requireField(raw.hasMore, "tradeHistoryV2Response.hasMore"),
  }));
