import { Side } from "@/primitives/Side";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
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

const isSideValue = (value: unknown): value is Side =>
  typeof value === "number" && Side[value as Side] !== undefined;

const parseOrderSide = (value: unknown): Side => {
  if (isSideValue(value)) {
    return value;
  }

  if (typeof value === "string") {
    const normalized = value.toLowerCase();
    const sideFromString: Record<string, Side> = {
      buy: Side.Bid,
      bid: Side.Bid,
      sell: Side.Ask,
      ask: Side.Ask,
    };
    if (normalized in sideFromString) {
      return sideFromString[normalized];
    }
  }

  throw new Error(`Unsupported order side value: ${String(value)}`);
};

const isOrderStatusValue = (value: unknown): value is OrderStatus =>
  Object.values(OrderStatus).includes(value as OrderStatus);

const parseOrderStatus = (value: unknown): OrderStatus => {
  if (typeof value === "string") {
    const normalized = value.toLowerCase();
    const statusFromString: Record<string, OrderStatus> = {
      active: OrderStatus.Active,
      open: OrderStatus.Active,
      [OrderStatus.Filled]: OrderStatus.Filled,
      [OrderStatus.Cancelled]: OrderStatus.Cancelled,
      [OrderStatus.Expired]: OrderStatus.Expired,
    };
    const status = statusFromString[normalized];
    if (status !== undefined) return status;
  }

  if (isOrderStatusValue(value)) {
    return value;
  }

  throw new Error(`Unsupported order status value: ${String(value)}`);
};

const nullableTimestampSchema = z
  .union([z.number(), z.string()])
  .transform((val) => toNumber(val, "timestamp"))
  .nullable();

export interface OrderHistoryRequest {
  traderPdaIndex?: number;
  marketSymbol?: string;
  limit?: number;
  cursor?: string;
  privyId?: string;
  orderStatus?: string;
}

export enum OrderStatus {
  Active = "active",
  Filled = "filled",
  Cancelled = "cancelled",
  Expired = "expired",
}

export interface OrderHistoryItem {
  orderSequenceNumber: string;
  marketSymbol: string;
  status: OrderStatus;
  side: Side;
  isReduceOnly: boolean;
  isStopLoss?: boolean;
  isStopLossDirection?: boolean;
  price: string;
  baseQty: string;
  remainingBaseQty: string;
  filledBaseQty: string;
  placedAt: number | null;
  completedAt: number | null;
}

const RawOrderHistoryItemSchema = z
  .object({
    orderSequenceNumber: z.string(),
    marketSymbol: z.string(),
    status: z.union([z.nativeEnum(OrderStatus), z.string()]),
    side: z.union([z.nativeEnum(Side), z.string(), z.number()]),
    price: z.string(),
    baseQty: z.string(),
    remainingBaseQty: z.string(),
    filledBaseQty: z.string(),
    isReduceOnly: z.boolean().nullable().optional().default(false),
    isStopLoss: z.boolean().nullable().optional().default(false),
    isStopLossDirection: z.boolean().nullable().optional().default(false),
    placedAt: nullableTimestampSchema.optional(),
    completedAt: nullableTimestampSchema.optional(),
  })
  .loose();

export const OrderHistoryItemSchema: z.ZodType<OrderHistoryItem> =
  RawOrderHistoryItemSchema.transform((raw) => ({
    orderSequenceNumber: requireField(
      raw.orderSequenceNumber,
      "orderHistory.data.orderSequenceNumber"
    ),
    marketSymbol: requireField(
      raw.marketSymbol,
      "orderHistory.data.marketSymbol"
    ),
    status: parseOrderStatus(raw.status),
    side: parseOrderSide(raw.side),
    price: requireField(raw.price, "orderHistory.data.price"),
    baseQty: requireField(raw.baseQty, "orderHistory.data.baseQty"),
    remainingBaseQty: requireField(
      raw.remainingBaseQty,
      "orderHistory.data.remainingBaseQty"
    ),
    filledBaseQty: requireField(
      raw.filledBaseQty,
      "orderHistory.data.filledBaseQty"
    ),
    isReduceOnly: raw.isReduceOnly ?? false,
    isStopLoss: raw.isStopLoss ?? false,
    isStopLossDirection: raw.isStopLossDirection ?? false,
    placedAt: raw.placedAt ?? null,
    completedAt: raw.completedAt ?? null,
  }));

export interface OrderHistoryResponse {
  data: OrderHistoryItem[];
  prevCursor?: string | null;
  nextCursor: string | null;
  hasMore: boolean;
}

const RawOrderHistoryResponseSchema = z
  .object({
    data: z.array(OrderHistoryItemSchema),
    prevCursor: z.string().nullable().optional(),
    nextCursor: z.string().nullable().optional(),
    hasMore: z.boolean(),
  })
  .loose();

export const OrderHistoryResponseSchema: z.ZodType<OrderHistoryResponse> =
  RawOrderHistoryResponseSchema.transform((raw) => ({
    data: raw.data,
    prevCursor: raw.prevCursor ?? null,
    nextCursor: raw.nextCursor ?? null,
    hasMore: requireField(raw.hasMore, "orderHistory.hasMore"),
  }));

export interface ApiInstructionAccountMeta {
  pubkey: string;
  isSigner: boolean;
  isWritable: boolean;
}

export const ApiInstructionAccountMetaSchema: z.ZodType<ApiInstructionAccountMeta> =
  z.object({
    pubkey: z.string(),
    isSigner: z.boolean(),
    isWritable: z.boolean(),
  });

export interface ApiInstructionResponse {
  data: number[];
  keys: ApiInstructionAccountMeta[];
  programId: string;
}

export const ApiInstructionResponseSchema: z.ZodType<ApiInstructionResponse> =
  z.object({
    data: z.array(z.number().int().min(0).max(255)),
    keys: z.array(ApiInstructionAccountMetaSchema),
    programId: z.string(),
  });

export interface PlaceIsolatedOrderEnhancedResponse {
  instructions: ApiInstructionResponse[];
  estimatedLiquidationPriceUsd: number | null;
}

export interface PlaceIsolatedOrderEnhancedInstructionsResponse {
  instructions: InstructionsWithAccountsAndData[];
  estimatedLiquidationPriceUsd: number | null;
}

export const PlaceIsolatedOrderEnhancedResponseSchema: z.ZodType<PlaceIsolatedOrderEnhancedResponse> =
  z
    .object({
      instructions: z.array(ApiInstructionResponseSchema),
      estimatedLiquidationPriceUsd: z.number().nullable().optional(),
    })
    .transform((raw) => ({
      instructions: raw.instructions,
      estimatedLiquidationPriceUsd: raw.estimatedLiquidationPriceUsd ?? null,
    }));

export interface TpSlOrderConfig {
  takeProfitTriggerPrice?: number | null;
  takeProfitTriggerPriceInTicks?: number | null;
  takeProfitExecutionPrice?: number | null;
  takeProfitExecutionPriceInTicks?: number | null;
  stopLossTriggerPrice?: number | null;
  stopLossTriggerPriceInTicks?: number | null;
  stopLossExecutionPrice?: number | null;
  stopLossExecutionPriceInTicks?: number | null;
  orderKind?: string | null;
  numBaseLots?: number | null;
  quantity?: number | null;
}

const TpSlOrderConfigObjectSchema = z.object({
  takeProfitTriggerPrice: z.number().nullable().optional(),
  takeProfitTriggerPriceInTicks: z
    .number()
    .int()
    .nonnegative()
    .nullable()
    .optional(),
  takeProfitExecutionPrice: z.number().nullable().optional(),
  takeProfitExecutionPriceInTicks: z
    .number()
    .int()
    .nonnegative()
    .nullable()
    .optional(),
  stopLossTriggerPrice: z.number().nullable().optional(),
  stopLossTriggerPriceInTicks: z
    .number()
    .int()
    .nonnegative()
    .nullable()
    .optional(),
  stopLossExecutionPrice: z.number().nullable().optional(),
  stopLossExecutionPriceInTicks: z
    .number()
    .int()
    .nonnegative()
    .nullable()
    .optional(),
  orderKind: z.string().nullable().optional(),
  numBaseLots: z.number().int().nonnegative().nullable().optional(),
  quantity: z.number().nullable().optional(),
});

export const TpSlOrderConfigSchema: z.ZodType<TpSlOrderConfig> =
  TpSlOrderConfigObjectSchema;

export interface PlaceStopLossOrderRequest extends TpSlOrderConfig {
  authority: string;
  positionAuthority?: string | null;
  traderPdaIndex: number;
  traderSubaccountIndex?: number | null;
  isIsolated?: boolean;
  symbol: string;
  side: string;
  feePayer?: string | null;
  flightBuilderAuthority?: string | null;
  flightFeeCollectorTrader?: string | null;
}

export const PlaceStopLossOrderRequestSchema: z.ZodType<PlaceStopLossOrderRequest> =
  TpSlOrderConfigObjectSchema.extend({
    authority: z.string(),
    positionAuthority: z.string().nullable().optional(),
    traderPdaIndex: z.number().int().nonnegative(),
    traderSubaccountIndex: z.number().int().nonnegative().nullable().optional(),
    isIsolated: z.boolean().optional(),
    symbol: z.string(),
    side: z.string(),
    feePayer: z.string().nullable().optional(),
    flightBuilderAuthority: z.string().nullable().optional(),
    flightFeeCollectorTrader: z.string().nullable().optional(),
  });

export type CancelStopLossOrderExecutionDirection =
  | "greater_than"
  | "less_than";

export const CancelStopLossOrderExecutionDirectionSchema: z.ZodType<CancelStopLossOrderExecutionDirection> =
  z.enum(["greater_than", "less_than"]);

export interface CancelStopLossOrderRequest {
  authority: string;
  positionAuthority?: string | null;
  traderPdaIndex: number;
  traderSubaccountIndex?: number | null;
  isIsolated?: boolean;
  symbol: string;
  executionDirection: CancelStopLossOrderExecutionDirection;
}

export const CancelStopLossOrderRequestSchema: z.ZodType<CancelStopLossOrderRequest> =
  z.object({
    authority: z.string(),
    positionAuthority: z.string().nullable().optional(),
    traderPdaIndex: z.number().int().nonnegative(),
    traderSubaccountIndex: z.number().int().nonnegative().nullable().optional(),
    isIsolated: z.boolean().optional(),
    symbol: z.string(),
    executionDirection: CancelStopLossOrderExecutionDirectionSchema,
  });

export interface ConditionalTriggerRequest {
  side: string;
  orderKind?: string | null;
  triggerPrice?: number | null;
  triggerPriceInTicks?: number | null;
  executionPrice?: number | null;
  executionPriceInTicks?: number | null;
}

export const ConditionalTriggerRequestSchema: z.ZodType<ConditionalTriggerRequest> =
  z.object({
    side: z.string(),
    orderKind: z.string().nullable().optional(),
    triggerPrice: z.number().nullable().optional(),
    triggerPriceInTicks: z.number().int().nonnegative().nullable().optional(),
    executionPrice: z.number().nullable().optional(),
    executionPriceInTicks: z.number().int().nonnegative().nullable().optional(),
  });

const hasConditionalTrigger = (request: {
  greaterTrigger?: ConditionalTriggerRequest | null;
  lessTrigger?: ConditionalTriggerRequest | null;
}): boolean => request.greaterTrigger != null || request.lessTrigger != null;

const requiresConditionalTrigger = {
  message: "At least one of greaterTrigger or lessTrigger is required",
  path: ["greaterTrigger"],
};

export interface PlaceAttachedConditionalOrderRequest {
  authority: string;
  positionAuthority?: string | null;
  traderPdaIndex: number;
  traderSubaccountIndex?: number | null;
  isIsolated?: boolean;
  symbol: string;
  feePayer?: string | null;
  orderSequenceNumber: string;
  orderPriceInTicks: number;
  greaterTrigger?: ConditionalTriggerRequest | null;
  lessTrigger?: ConditionalTriggerRequest | null;
  flightBuilderAuthority?: string | null;
  flightFeeCollectorTrader?: string | null;
}

export const PlaceAttachedConditionalOrderRequestSchema: z.ZodType<PlaceAttachedConditionalOrderRequest> =
  z
    .object({
      authority: z.string(),
      positionAuthority: z.string().nullable().optional(),
      traderPdaIndex: z.number().int().nonnegative(),
      traderSubaccountIndex: z
        .number()
        .int()
        .nonnegative()
        .nullable()
        .optional(),
      isIsolated: z.boolean().optional(),
      symbol: z.string(),
      feePayer: z.string().nullable().optional(),
      orderSequenceNumber: z.string(),
      orderPriceInTicks: z.number().int().nonnegative(),
      greaterTrigger: ConditionalTriggerRequestSchema.nullable().optional(),
      lessTrigger: ConditionalTriggerRequestSchema.nullable().optional(),
      flightBuilderAuthority: z.string().nullable().optional(),
      flightFeeCollectorTrader: z.string().nullable().optional(),
    })
    .refine(hasConditionalTrigger, requiresConditionalTrigger);

export interface PlacePositionConditionalOrderRequest {
  authority: string;
  positionAuthority?: string | null;
  traderPdaIndex: number;
  traderSubaccountIndex?: number | null;
  isIsolated?: boolean;
  symbol: string;
  feePayer?: string | null;
  greaterTrigger?: ConditionalTriggerRequest | null;
  lessTrigger?: ConditionalTriggerRequest | null;
  sizePercent?: number | null;
  numBaseLots?: number | null;
  quantity?: number | null;
  flightBuilderAuthority?: string | null;
  flightFeeCollectorTrader?: string | null;
}

export const PlacePositionConditionalOrderRequestSchema: z.ZodType<PlacePositionConditionalOrderRequest> =
  z
    .object({
      authority: z.string(),
      positionAuthority: z.string().nullable().optional(),
      traderPdaIndex: z.number().int().nonnegative(),
      traderSubaccountIndex: z
        .number()
        .int()
        .nonnegative()
        .nullable()
        .optional(),
      isIsolated: z.boolean().optional(),
      symbol: z.string(),
      feePayer: z.string().nullable().optional(),
      greaterTrigger: ConditionalTriggerRequestSchema.nullable().optional(),
      lessTrigger: ConditionalTriggerRequestSchema.nullable().optional(),
      sizePercent: z.number().int().min(1).max(100).nullable().optional(),
      numBaseLots: z.number().int().nonnegative().nullable().optional(),
      quantity: z.number().nullable().optional(),
      flightBuilderAuthority: z.string().nullable().optional(),
      flightFeeCollectorTrader: z.string().nullable().optional(),
    })
    .refine(hasConditionalTrigger, requiresConditionalTrigger);

export type CancelConditionalOrderExecutionDirection =
  | "greater_than"
  | "less_than";

export const CancelConditionalOrderExecutionDirectionSchema: z.ZodType<CancelConditionalOrderExecutionDirection> =
  z.enum(["greater_than", "less_than"]);

export interface CancelConditionalOrderRequest {
  authority: string;
  positionAuthority?: string;
  traderPdaIndex: number;
  traderSubaccountIndex?: number;
  isIsolated?: boolean;
  symbol: string;
  conditionalOrderIndex: number;
  executionDirection: CancelConditionalOrderExecutionDirection;
}

export const CancelConditionalOrderRequestSchema: z.ZodType<CancelConditionalOrderRequest> =
  z.object({
    authority: z.string(),
    positionAuthority: z.string().optional(),
    traderPdaIndex: z.number().int().nonnegative(),
    traderSubaccountIndex: z.number().int().nonnegative().optional(),
    isIsolated: z.boolean().optional(),
    symbol: z.string(),
    conditionalOrderIndex: z.number().int().positive().max(191),
    executionDirection: CancelConditionalOrderExecutionDirectionSchema,
  });

export interface PlaceIsolatedLimitOrderRequest {
  authority: string;
  positionAuthority?: string;
  symbol: string;
  side: string;
  priceInTicks?: number;
  price?: number;
  numBaseLots?: number;
  quantity?: number;
  transferAmount?: number;
  transferSpotCollateralAmounts?: Record<string, number>;
  pdaIndex?: number;
  allowCrossAndIsolatedForAsset?: boolean;
  feePayer?: string;
  isReduceOnly?: boolean;
  isPostOnly?: boolean;
  slide?: boolean;
  skipTransferToParent?: boolean;
  flightBuilderAuthority?: string;
  flightFeeCollectorTrader?: string;
  tpSl?: TpSlOrderConfig;
}

export const PlaceIsolatedLimitOrderRequestSchema: z.ZodType<PlaceIsolatedLimitOrderRequest> =
  z.object({
    authority: z.string(),
    positionAuthority: z.string().optional(),
    symbol: z.string(),
    side: z.string(),
    priceInTicks: z.number().int().nonnegative().optional(),
    price: z.number().optional(),
    numBaseLots: z.number().int().nonnegative().optional(),
    quantity: z.number().optional(),
    transferAmount: z.number().int().nonnegative().optional(),
    transferSpotCollateralAmounts: z
      .record(z.string(), z.number().int().nonnegative())
      .optional(),
    pdaIndex: z.number().int().nonnegative().optional(),
    allowCrossAndIsolatedForAsset: z.boolean().optional(),
    feePayer: z.string().optional(),
    isReduceOnly: z.boolean().optional(),
    isPostOnly: z.boolean().optional(),
    slide: z.boolean().optional(),
    skipTransferToParent: z.boolean().optional(),
    flightBuilderAuthority: z.string().optional(),
    flightFeeCollectorTrader: z.string().optional(),
    tpSl: TpSlOrderConfigSchema.optional(),
  });

export interface PlaceIsolatedLimitOrderWithConditionalsRequest {
  authority: string;
  positionAuthority?: string | null;
  symbol: string;
  side: string;
  priceInTicks?: number | null;
  price?: number | null;
  numBaseLots?: number | null;
  quantity?: number | null;
  transferAmount?: number;
  transferSpotCollateralAmounts?: Record<string, number>;
  pdaIndex?: number | null;
  allowCrossAndIsolatedForAsset?: boolean | null;
  feePayer?: string | null;
  isPostOnly?: boolean | null;
  slide?: boolean | null;
  skipTransferToParent?: boolean | null;
  greaterTrigger?: ConditionalTriggerRequest | null;
  lessTrigger?: ConditionalTriggerRequest | null;
  flightBuilderAuthority?: string | null;
  flightFeeCollectorTrader?: string | null;
}

export const PlaceIsolatedLimitOrderWithConditionalsRequestSchema: z.ZodType<PlaceIsolatedLimitOrderWithConditionalsRequest> =
  z
    .object({
      authority: z.string(),
      positionAuthority: z.string().nullable().optional(),
      symbol: z.string(),
      side: z.string(),
      priceInTicks: z.number().int().nonnegative().nullable().optional(),
      price: z.number().nullable().optional(),
      numBaseLots: z.number().int().nonnegative().nullable().optional(),
      quantity: z.number().nullable().optional(),
      transferAmount: z.number().int().nonnegative().optional(),
      transferSpotCollateralAmounts: z
        .record(z.string(), z.number().int().nonnegative())
        .optional(),
      pdaIndex: z.number().int().nonnegative().nullable().optional(),
      allowCrossAndIsolatedForAsset: z.boolean().nullable().optional(),
      feePayer: z.string().nullable().optional(),
      isPostOnly: z.boolean().nullable().optional(),
      slide: z.boolean().nullable().optional(),
      skipTransferToParent: z.boolean().nullable().optional(),
      greaterTrigger: ConditionalTriggerRequestSchema.nullable().optional(),
      lessTrigger: ConditionalTriggerRequestSchema.nullable().optional(),
      flightBuilderAuthority: z.string().nullable().optional(),
      flightFeeCollectorTrader: z.string().nullable().optional(),
    })
    .refine(hasConditionalTrigger, requiresConditionalTrigger);

export interface PlaceIsolatedMarketOrderRequest {
  authority: string;
  positionAuthority?: string;
  symbol: string;
  side: string;
  numBaseLots?: number;
  minBaseLotsToFill?: number;
  minQuoteLotsToFill?: number;
  quantity?: number;
  transferAmount?: number;
  transferSpotCollateralAmounts?: Record<string, number>;
  maxPriceInTicks?: number;
  pdaIndex?: number;
  allowCrossAndIsolatedForAsset?: boolean;
  feePayer?: string;
  isReduceOnly?: boolean;
  skipTransferToParent?: boolean;
  flightBuilderAuthority?: string;
  flightFeeCollectorTrader?: string;
  tpSl?: TpSlOrderConfig;
}

export const PlaceIsolatedMarketOrderRequestSchema: z.ZodType<PlaceIsolatedMarketOrderRequest> =
  z.object({
    authority: z.string(),
    positionAuthority: z.string().optional(),
    symbol: z.string(),
    side: z.string(),
    numBaseLots: z.number().int().nonnegative().optional(),
    minBaseLotsToFill: z.number().int().nonnegative().optional(),
    minQuoteLotsToFill: z.number().int().nonnegative().optional(),
    quantity: z.number().optional(),
    transferAmount: z.number().int().nonnegative().optional(),
    transferSpotCollateralAmounts: z
      .record(z.string(), z.number().int().nonnegative())
      .optional(),
    maxPriceInTicks: z.number().int().nonnegative().optional(),
    pdaIndex: z.number().int().nonnegative().optional(),
    allowCrossAndIsolatedForAsset: z.boolean().optional(),
    feePayer: z.string().optional(),
    isReduceOnly: z.boolean().optional(),
    skipTransferToParent: z.boolean().optional(),
    flightBuilderAuthority: z.string().optional(),
    flightFeeCollectorTrader: z.string().optional(),
    tpSl: TpSlOrderConfigSchema.optional(),
  });

export type ServerBuiltInstruction = InstructionsWithAccountsAndData;

// ============================================================================
// Order History V2 Types
// ============================================================================

export interface OrderHistoryV2Request {
  marketSymbol?: string;
  limit?: number;
  cursor?: string;
  traderPdaIndex?: number;
  startTime?: Date;
  endTime?: Date;
}

export interface IntendedOrder {
  orderKind: string;
  side: string;
  priceInTicks: number | null;
  priceUsd: string | null;
  hasPriceLimit: boolean;
  baseLots: number;
  baseQty: string;
  quoteLotBudget: number | null;
  matchLimit: number | null;
  lastValidSlot: number | null;
  clientOrderId: string | null;
  orderFlags: number;
  isReduceOnly: boolean;
  isStopLoss: boolean;
  isStopLossDirection?: boolean;
  isConditionalOrder: boolean;
  isPostOnly: boolean;
  isTakeOnly: boolean;
  cancelExisting: boolean;
}

export interface PlacedOrder {
  orderSequenceNumber: number;
  eventIndex: number;
  side: string;
  price: string;
  baseQty: string;
  lastValidSlot: number | null;
  initialSlot: number | null;
  orderFlags: number | null;
  clientOrderId?: string | null;
  transactionTimestamp: string;
}

export interface CurrentOrderState {
  orderSequenceNumber: number;
  side: string;
  price: string;
  baseQty: string;
  remainingBaseQty: string;
  filledBaseQty: string;
  releasedBaseQty: string;
  orderStatus: string;
  placedAt: string | null;
  cancelledAt: string | null;
  lastFillAt: string | null;
}

export interface AggregatedTrades {
  totalBaseQtyFilled: string;
  totalQuoteQtyFilled: string;
  totalTakerFee: string;
  totalNumFills: number;
  numTrades: number;
}

export interface OrderHistoryV2Item {
  userId: number;
  traderId: number;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  slot: number;
  slotIndex: number;
  instructionIndex: number;
  eventIndex: number;
  marketSymbol: string;
  instructionType: string;
  orderType: string;
  createdAt: string;
  price?: string;
  intended: IntendedOrder;
  fillBeforePlacement: AggregatedTrades | null;
  placedOrder: PlacedOrder | null;
  currentOrderState: CurrentOrderState | null;
  isReduceOnly: boolean;
  isStopLoss: boolean;
  isStopLossDirection?: boolean;
  isConditionalOrder: boolean;
  status: string;
}

export interface OrderHistoryV2Response {
  data: OrderHistoryV2Item[];
  prevCursor?: string | null;
  nextCursor: string | null;
  hasMore: boolean;
}

const IntendedOrderSchema = z.object({
  orderKind: z.string(),
  side: z.string(),
  priceInTicks: z.number().nullable(),
  priceUsd: z.string().nullable(),
  hasPriceLimit: z.boolean(),
  baseLots: z.number(),
  baseQty: z.string(),
  quoteLotBudget: z.number().nullable(),
  matchLimit: z.number().nullable(),
  lastValidSlot: z.number().nullable(),
  clientOrderId: z.string().nullable(),
  orderFlags: z.number(),
  isReduceOnly: z.boolean(),
  isStopLoss: z.boolean(),
  isStopLossDirection: z.boolean().optional().default(false),
  isConditionalOrder: z.boolean().optional().default(false),
  isPostOnly: z.boolean(),
  isTakeOnly: z.boolean(),
  cancelExisting: z.boolean(),
});

const PlacedOrderSchema = z.object({
  orderSequenceNumber: z.number(),
  eventIndex: z.number(),
  side: z.string(),
  price: z.string(),
  baseQty: z.string(),
  lastValidSlot: z.number().nullable(),
  initialSlot: z.number().nullable(),
  orderFlags: z.number().nullable(),
  clientOrderId: z.string().nullable().optional(),
  transactionTimestamp: z.string(),
});

const CurrentOrderStateSchema = z.object({
  orderSequenceNumber: z.number(),
  side: z.string(),
  price: z.string(),
  baseQty: z.string(),
  remainingBaseQty: z.string(),
  filledBaseQty: z.string(),
  releasedBaseQty: z.string(),
  orderStatus: z.string(),
  placedAt: z.string().nullable(),
  cancelledAt: z.string().nullable(),
  lastFillAt: z.string().nullable(),
});

const OrderHistoryV2ItemSchema = z.object({
  userId: z.number(),
  traderId: z.number(),
  traderPdaIndex: z.number(),
  traderSubaccountIndex: z.number(),
  slot: z.number(),
  slotIndex: z.number(),
  instructionIndex: z.number(),
  eventIndex: z.number(),
  marketSymbol: z.string(),
  instructionType: z.string(),
  orderType: z.string(),
  createdAt: z.string(),
  price: z.string().optional(),
  intended: IntendedOrderSchema,
  fillBeforePlacement: z
    .object({
      totalBaseQtyFilled: z.string(),
      totalQuoteQtyFilled: z.string(),
      totalTakerFee: z.string(),
      totalNumFills: z.number(),
      numTrades: z.number(),
    })
    .nullable(),
  placedOrder: PlacedOrderSchema.nullable(),
  currentOrderState: CurrentOrderStateSchema.nullable(),
  isReduceOnly: z.boolean(),
  isStopLoss: z.boolean(),
  isStopLossDirection: z.boolean().optional().default(false),
  isConditionalOrder: z.boolean().optional().default(false),
  status: z.string(),
});

export const OrderHistoryV2ResponseSchema: z.ZodType<OrderHistoryV2Response> = z
  .object({
    data: z.array(OrderHistoryV2ItemSchema),
    prevCursor: z.string().nullable().optional(),
    nextCursor: z.string().nullable().optional(),
    hasMore: z.boolean(),
  })
  .transform((raw) => ({
    data: raw.data as OrderHistoryV2Item[],
    prevCursor: raw.prevCursor ?? null,
    nextCursor: raw.nextCursor ?? null,
    hasMore: raw.hasMore,
  }));

// ============================================================================
// Order Details Types
// ============================================================================

export interface OrderFill {
  slot: number;
  slotIndex?: number | null;
  eventIndex?: number | null;
  tradeSequenceNumber?: number | null;
  fillBaseQty: string;
  fillQuoteQty: string;
  makerFee?: string | null;
  takerFee?: string | null;
  transactionTimestamp: string;
}

export interface OrderModifiedEvent {
  eventIndex?: number | null;
  slot: number;
  slotIndex?: number | null;
  price?: string | null;
  baseLotsReleased?: number | null;
  reason?: string | null;
  transactionTimestamp: string;
}

export interface PnLEvent {
  slot: number;
  slotIndex?: number | null;
  eventIndex?: number | null;
  realizedPnl?: string | null;
  fundingPayment?: string | null;
  baseLotsBefore?: number | null;
  baseLotsAfter?: number | null;
  virtualQuoteLotsBefore?: number | null;
  virtualQuoteLotsAfter?: number | null;
  transactionTimestamp: string;
}

export interface OrderDetailsResponse {
  userId?: number | null;
  traderId?: number | null;
  traderPdaIndex?: number | null;
  traderSubaccountIndex?: number | null;
  slot?: number | null;
  slotIndex?: number | null;
  instructionIndex?: number | null;
  eventIndex?: number | null;
  marketSymbol?: string | null;
  instructionType?: string | null;
  orderType?: string | null;
  createdAt?: string | null;
  intended?: IntendedOrder | null;
  fillBeforePlacement?: unknown[] | null;
  placedOrder?: PlacedOrder | null;
  currentOrderState?: CurrentOrderState | null;
  fills?: OrderFill[] | null;
  events?: OrderModifiedEvent[] | null;
  pnls?: PnLEvent[] | null;
}

const OrderFillSchema = z.object({
  slot: z.any(),
  slotIndex: z.any().nullable().optional(),
  eventIndex: z.any().nullable().optional(),
  tradeSequenceNumber: z.any().nullable().optional(),
  fillBaseQty: z.any(),
  fillQuoteQty: z.any(),
  makerFee: z.any().nullable().optional(),
  takerFee: z.any().nullable().optional(),
  transactionTimestamp: z.any(),
});

const OrderModifiedEventSchema = z.object({
  eventIndex: z.any().nullable().optional(),
  slot: z.any(),
  slotIndex: z.any().nullable().optional(),
  price: z.any().nullable().optional(),
  baseLotsReleased: z.any().nullable().optional(),
  reason: z.string().nullable().optional(),
  transactionTimestamp: z.any(),
});

const PnLEventSchema = z.object({
  slot: z.any(),
  slotIndex: z.any().nullable().optional(),
  eventIndex: z.any().nullable().optional(),
  realizedPnl: z.any().nullable().optional(),
  fundingPayment: z.any().nullable().optional(),
  baseLotsBefore: z.any().nullable().optional(),
  baseLotsAfter: z.any().nullable().optional(),
  virtualQuoteLotsBefore: z.any().nullable().optional(),
  virtualQuoteLotsAfter: z.any().nullable().optional(),
  transactionTimestamp: z.any(),
});

export const OrderDetailsResponseSchema: z.ZodType<OrderDetailsResponse> = z
  .object({
    userId: z.number().nullable().optional(),
    traderId: z.number().nullable().optional(),
    traderPdaIndex: z.number().nullable().optional(),
    traderSubaccountIndex: z.number().nullable().optional(),
    slot: z.number().nullable().optional(),
    slotIndex: z.number().nullable().optional(),
    instructionIndex: z.number().nullable().optional(),
    eventIndex: z.number().nullable().optional(),
    marketSymbol: z.string().nullable().optional(),
    instructionType: z.string().nullable().optional(),
    orderType: z.string().nullable().optional(),
    createdAt: z.string().nullable().optional(),
    intended: IntendedOrderSchema.nullable().optional(),
    fillBeforePlacement: z.array(z.unknown()).nullable().optional(),
    placedOrder: PlacedOrderSchema.nullable().optional(),
    currentOrderState: CurrentOrderStateSchema.nullable().optional(),
    fills: z.array(OrderFillSchema).nullable().optional(),
    events: z.array(OrderModifiedEventSchema).nullable().optional(),
    pnls: z.array(PnLEventSchema).nullable().optional(),
  })
  .transform((raw) => raw as OrderDetailsResponse);
