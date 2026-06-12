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

const requireNumberField = (value: unknown, fieldName: string): number =>
  toNumber(requireField(value, fieldName), fieldName);

const requireLiquidationType = (
  actual: UserLiquidationHistoryType,
  expected: UserLiquidationHistoryType,
  fieldName: string
): void => {
  if (actual !== expected) {
    throw new Error(
      `Invalid ${fieldName} in history response: expected ${expected}, received ${actual}`
    );
  }
};

export interface TradeHistoryRequest {
  pdaIndex?: number;
  marketSymbol?: string;
  limit?: number;
  cursor?: string;
  privyId?: string;
}

export interface UserLiquidationHistoryRequest {
  pdaIndex?: number;
  subaccountIndex?: number;
  symbol?: string;
  /** Number of items to return (max 100, API default 100). */
  limit?: number;
  cursor?: string;
}

export type UserLiquidationHistoryKind = "market_order" | "adl" | "backstop";

export type UserLiquidationHistoryType = "market" | "adl" | "backstop";

export type UserLiquidationHistoryRole =
  | "liquidatee"
  | "backstop_liquidatee"
  | "adl_closed_short"
  | "adl_closed_long"
  | "adl_in_profit"
  | "adl_caller";

export interface UserLiquidationHistoryBasePoint {
  ixName: string;
  role: UserLiquidationHistoryRole;
  slot: number;
  slotIndex: number;
  eventIndex: number;
  timestamp: number;
  signature: string | null;
  symbol: string;
  market: string;
  subaccountIndex: number | null;
}

export interface UserMarketLiquidationHistoryPoint extends UserLiquidationHistoryBasePoint {
  kind: "market_order";
  type: "market";
  liquidatee: string;
  liquidator: string;
  side: "LONG" | "SHORT";
  size: string;
  price: string;
  positionClosed: boolean;
  baseLotsFilled: string;
  quoteLotsFilled: string;
}

export interface UserBackstopLiquidationHistoryPoint extends UserLiquidationHistoryBasePoint {
  kind: "backstop";
  type: "backstop";
  liquidatee: string;
  liquidator: string;
  size: string;
  quoteSize: string;
  haircutRateBps: number;
  liquidateeCollateralChange: string;
  liquidatorCollateralChange: string;
}

export interface UserAdlLiquidationHistoryPoint extends UserLiquidationHistoryBasePoint {
  kind: "adl";
  type: "adl";
  caller: string;
  closedShort: string;
  closedLong: string;
  inProfitAccount: string;
  size: string;
  atLossCloseValue: string;
  inProfitCloseValue: string;
  atLossCollateralChange: string;
  inProfitCollateralChange: string;
}

export type UserLiquidationHistoryPoint =
  | UserMarketLiquidationHistoryPoint
  | UserBackstopLiquidationHistoryPoint
  | UserAdlLiquidationHistoryPoint;

const RawUserLiquidationHistoryPointSchema = z
  .object({
    kind: z.enum(["market_order", "adl", "backstop"]),
    type: z.enum(["market", "adl", "backstop"]),
    ixName: z.string(),
    role: z.enum([
      "liquidatee",
      "backstop_liquidatee",
      "adl_closed_short",
      "adl_closed_long",
      "adl_in_profit",
      "adl_caller",
    ]),
    slot: z.union([z.number(), z.string()]),
    slotIndex: z.union([z.number(), z.string()]),
    eventIndex: z.union([z.number(), z.string()]),
    timestamp: z.union([z.number(), z.string()]),
    signature: z.string().nullable().optional(),
    symbol: z.string(),
    market: z.string(),
    subaccountIndex: z.union([z.number(), z.string()]).nullable().optional(),
    liquidatee: z.string().optional(),
    liquidator: z.string().optional(),
    side: z.enum(["LONG", "SHORT"]).optional(),
    size: z.string().optional(),
    price: z.string().optional(),
    positionClosed: z.boolean().optional(),
    baseLotsFilled: z.string().optional(),
    quoteLotsFilled: z.string().optional(),
    quoteSize: z.string().optional(),
    haircutRateBps: z.union([z.number(), z.string()]).optional(),
    liquidateeCollateralChange: z.string().optional(),
    liquidatorCollateralChange: z.string().optional(),
    caller: z.string().optional(),
    closedShort: z.string().optional(),
    closedLong: z.string().optional(),
    inProfitAccount: z.string().optional(),
    atLossCloseValue: z.string().optional(),
    inProfitCloseValue: z.string().optional(),
    atLossCollateralChange: z.string().optional(),
    inProfitCollateralChange: z.string().optional(),
  })
  .loose();

export const UserLiquidationHistoryPointSchema: z.ZodType<UserLiquidationHistoryPoint> =
  RawUserLiquidationHistoryPointSchema.transform((raw) => {
    const common: UserLiquidationHistoryBasePoint = {
      ixName: requireField(raw.ixName, "userLiquidationHistoryPoint.ixName"),
      role: requireField(raw.role, "userLiquidationHistoryPoint.role"),
      slot: toNumber(raw.slot, "userLiquidationHistoryPoint.slot"),
      slotIndex: toNumber(
        raw.slotIndex,
        "userLiquidationHistoryPoint.slotIndex"
      ),
      eventIndex: toNumber(
        raw.eventIndex,
        "userLiquidationHistoryPoint.eventIndex"
      ),
      timestamp: toNumber(
        raw.timestamp,
        "userLiquidationHistoryPoint.timestamp"
      ),
      signature: raw.signature ?? null,
      symbol: requireField(raw.symbol, "userLiquidationHistoryPoint.symbol"),
      market: requireField(raw.market, "userLiquidationHistoryPoint.market"),
      subaccountIndex:
        raw.subaccountIndex === null || raw.subaccountIndex === undefined
          ? null
          : toNumber(
              raw.subaccountIndex,
              "userLiquidationHistoryPoint.subaccountIndex"
            ),
    };

    switch (raw.kind) {
      case "market_order":
        requireLiquidationType(
          raw.type,
          "market",
          "userLiquidationHistoryPoint.type"
        );
        return {
          ...common,
          kind: "market_order",
          type: "market",
          liquidatee: requireField(
            raw.liquidatee,
            "userLiquidationHistoryPoint.liquidatee"
          ),
          liquidator: requireField(
            raw.liquidator,
            "userLiquidationHistoryPoint.liquidator"
          ),
          side: requireField(raw.side, "userLiquidationHistoryPoint.side"),
          size: requireField(raw.size, "userLiquidationHistoryPoint.size"),
          price: requireField(raw.price, "userLiquidationHistoryPoint.price"),
          positionClosed: requireField(
            raw.positionClosed,
            "userLiquidationHistoryPoint.positionClosed"
          ),
          baseLotsFilled: requireField(
            raw.baseLotsFilled,
            "userLiquidationHistoryPoint.baseLotsFilled"
          ),
          quoteLotsFilled: requireField(
            raw.quoteLotsFilled,
            "userLiquidationHistoryPoint.quoteLotsFilled"
          ),
        };
      case "backstop":
        requireLiquidationType(
          raw.type,
          "backstop",
          "userLiquidationHistoryPoint.type"
        );
        return {
          ...common,
          kind: "backstop",
          type: "backstop",
          liquidatee: requireField(
            raw.liquidatee,
            "userLiquidationHistoryPoint.liquidatee"
          ),
          liquidator: requireField(
            raw.liquidator,
            "userLiquidationHistoryPoint.liquidator"
          ),
          size: requireField(raw.size, "userLiquidationHistoryPoint.size"),
          quoteSize: requireField(
            raw.quoteSize,
            "userLiquidationHistoryPoint.quoteSize"
          ),
          haircutRateBps: requireNumberField(
            raw.haircutRateBps,
            "userLiquidationHistoryPoint.haircutRateBps"
          ),
          liquidateeCollateralChange: requireField(
            raw.liquidateeCollateralChange,
            "userLiquidationHistoryPoint.liquidateeCollateralChange"
          ),
          liquidatorCollateralChange: requireField(
            raw.liquidatorCollateralChange,
            "userLiquidationHistoryPoint.liquidatorCollateralChange"
          ),
        };
      case "adl":
        requireLiquidationType(
          raw.type,
          "adl",
          "userLiquidationHistoryPoint.type"
        );
        return {
          ...common,
          kind: "adl",
          type: "adl",
          caller: requireField(
            raw.caller,
            "userLiquidationHistoryPoint.caller"
          ),
          closedShort: requireField(
            raw.closedShort,
            "userLiquidationHistoryPoint.closedShort"
          ),
          closedLong: requireField(
            raw.closedLong,
            "userLiquidationHistoryPoint.closedLong"
          ),
          inProfitAccount: requireField(
            raw.inProfitAccount,
            "userLiquidationHistoryPoint.inProfitAccount"
          ),
          size: requireField(raw.size, "userLiquidationHistoryPoint.size"),
          atLossCloseValue: requireField(
            raw.atLossCloseValue,
            "userLiquidationHistoryPoint.atLossCloseValue"
          ),
          inProfitCloseValue: requireField(
            raw.inProfitCloseValue,
            "userLiquidationHistoryPoint.inProfitCloseValue"
          ),
          atLossCollateralChange: requireField(
            raw.atLossCollateralChange,
            "userLiquidationHistoryPoint.atLossCollateralChange"
          ),
          inProfitCollateralChange: requireField(
            raw.inProfitCollateralChange,
            "userLiquidationHistoryPoint.inProfitCollateralChange"
          ),
        };
    }
  });

export interface UserLiquidationHistoryResponse {
  data: UserLiquidationHistoryPoint[];
  prevCursor: string | null;
  nextCursor: string | null;
  hasMore: boolean;
}

const RawUserLiquidationHistoryResponseSchema = z
  .object({
    data: z.array(RawUserLiquidationHistoryPointSchema),
    prevCursor: z.string().nullable().optional(),
    nextCursor: z.string().nullable().optional(),
    hasMore: z.boolean(),
  })
  .loose();

export const UserLiquidationHistoryResponseSchema: z.ZodType<UserLiquidationHistoryResponse> =
  RawUserLiquidationHistoryResponseSchema.transform((raw) => ({
    data: raw.data.map((item) => UserLiquidationHistoryPointSchema.parse(item)),
    prevCursor: raw.prevCursor ?? null,
    nextCursor: raw.nextCursor ?? null,
    hasMore: requireField(raw.hasMore, "userLiquidationHistory.hasMore"),
  }));

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
