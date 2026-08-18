import z from "zod";

const safeIntegerSchema = z.number().int().refine(Number.isSafeInteger, {
  message: "Expected safe integer",
});

const bigintLikeSchema = z
  .bigint()
  .or(z.string().regex(/^-?\d+$/))
  .or(safeIntegerSchema)
  .transform((value: bigint | string | number) =>
    typeof value === "bigint" ? value : BigInt(value)
  );

export interface TradeEventData {
  slot: bigint;
  slotIndex: number;
  timestamp: bigint;
  symbol: string;
  taker: string;
  tradeSequenceNumber: bigint;
  side: "bid" | "ask";
  baseLotsFilled: bigint;
  quoteLotsFilled: bigint;
  feeInQuoteLots: bigint;
  baseAmount: number;
  quoteAmount: number;
  numFills: number;
}

export const TradeEventDataSchema: z.ZodType<TradeEventData> = z.object({
  slot: bigintLikeSchema,
  slotIndex: safeIntegerSchema,
  timestamp: bigintLikeSchema,
  symbol: z.string(),
  taker: z.string(),
  tradeSequenceNumber: bigintLikeSchema,
  side: z.enum(["bid", "ask"]),
  baseLotsFilled: bigintLikeSchema,
  quoteLotsFilled: bigintLikeSchema,
  feeInQuoteLots: bigintLikeSchema,
  baseAmount: z.number(),
  quoteAmount: z.number(),
  numFills: safeIntegerSchema,
});

export interface OrderFillEventData {
  slot: bigint;
  slotIndex: number;
  timestamp: bigint;
  symbol: string;
  maker: string;
  taker: string;
  orderSequenceNumber: bigint;
  tradeSequenceNumber: bigint;
  makerFee: bigint;
  takerFee: bigint;
  priceInTicks: bigint;
  price: number;
  side: "bid" | "ask";
  baseLotsFilled: bigint;
  quoteLotsFilled: bigint;
  quantityRemaining: bigint;
  orderbookSequenceNumber: bigint;
}

export const OrderFillEventDataSchema: z.ZodType<OrderFillEventData> = z.object(
  {
    slot: bigintLikeSchema,
    slotIndex: safeIntegerSchema,
    timestamp: bigintLikeSchema,
    symbol: z.string(),
    maker: z.string(),
    taker: z.string(),
    orderSequenceNumber: bigintLikeSchema,
    tradeSequenceNumber: bigintLikeSchema,
    makerFee: bigintLikeSchema,
    takerFee: bigintLikeSchema,
    priceInTicks: bigintLikeSchema,
    price: z.number(),
    side: z.enum(["bid", "ask"]),
    baseLotsFilled: bigintLikeSchema,
    quoteLotsFilled: bigintLikeSchema,
    quantityRemaining: bigintLikeSchema,
    orderbookSequenceNumber: bigintLikeSchema,
  }
);

export interface OrderModifiedEventData {
  slot: bigint;
  slotIndex: number;
  timestamp: bigint;
  symbol: string;
  orderSequenceNumber: bigint;
  side: "bid" | "ask";
  trader: string;
  priceInTicks: bigint;
  baseLotsReleased: bigint;
  quoteLotsReleased: bigint;
  baseLotsRemaining: bigint;
  orderbookSequenceNumber?: bigint | null;
  reason?: string;
}

export const OrderModifiedEventDataSchema: z.ZodType<OrderModifiedEventData> =
  z.object({
    slot: bigintLikeSchema,
    slotIndex: safeIntegerSchema,
    timestamp: bigintLikeSchema,
    symbol: z.string(),
    orderSequenceNumber: bigintLikeSchema,
    side: z.enum(["bid", "ask"]),
    trader: z.string(),
    priceInTicks: bigintLikeSchema,
    baseLotsReleased: bigintLikeSchema,
    quoteLotsReleased: bigintLikeSchema,
    baseLotsRemaining: bigintLikeSchema,
    orderbookSequenceNumber: bigintLikeSchema.nullable().optional(),
    reason: z.string().optional(),
  });

export interface LiquidationTransferEventData {
  slot: bigint;
  slotIndex: number;
  timestamp: bigint;
  assetId: bigint;
  liquidatee: string;
  liquidator: string;
  baseLotsTransferred: bigint;
  virtualQuoteLotsTransferred: bigint;
  haircutRate: number;
  liquidateeCollateralChange: bigint;
  liquidatorCollateralChange: bigint;
}

export const LiquidationTransferEventDataSchema: z.ZodType<LiquidationTransferEventData> =
  z.object({
    slot: bigintLikeSchema,
    slotIndex: safeIntegerSchema,
    timestamp: bigintLikeSchema,
    assetId: bigintLikeSchema,
    liquidatee: z.string(),
    liquidator: z.string(),
    baseLotsTransferred: bigintLikeSchema,
    virtualQuoteLotsTransferred: bigintLikeSchema,
    haircutRate: safeIntegerSchema,
    liquidateeCollateralChange: bigintLikeSchema,
    liquidatorCollateralChange: bigintLikeSchema,
  });

export interface CloseMatchedPositionsEventData {
  slot: bigint;
  slotIndex: number;
  timestamp: bigint;
  assetId: bigint;
  caller: string;
  closedShort: string;
  closedLong: string;
  inProfitAccount: string;
  baseLotsClosed: bigint;
  atLossCloseValue: bigint;
  inProfitCloseValue: bigint;
  atLossCollateralChange: bigint;
  inProfitCollateralChange: bigint;
}

export const CloseMatchedPositionsEventDataSchema: z.ZodType<CloseMatchedPositionsEventData> =
  z.object({
    slot: bigintLikeSchema,
    slotIndex: safeIntegerSchema,
    timestamp: bigintLikeSchema,
    assetId: bigintLikeSchema,
    caller: z.string(),
    closedShort: z.string(),
    closedLong: z.string(),
    inProfitAccount: z.string(),
    baseLotsClosed: bigintLikeSchema,
    atLossCloseValue: bigintLikeSchema,
    inProfitCloseValue: bigintLikeSchema,
    atLossCollateralChange: bigintLikeSchema,
    inProfitCollateralChange: bigintLikeSchema,
  });

export interface OrderPlacedEventData {
  slot: bigint;
  slotIndex: number;
  timestamp: bigint;
  symbol: string;
  orderSequenceNumber: bigint;
  trader: string;
  /** Bit-packed order flags (serialized as a single bare u8). */
  orderFlags: number;
  clientOrderId?: string | null;
  orderId?: string | null;
  price: bigint;
  /** Signed quantity. */
  quantity: bigint;
  side: "bid" | "ask";
  lastValidSlot?: bigint | null;
  initialSlot: bigint;
  orderbookSequenceNumber: bigint;
}

export const OrderPlacedEventDataSchema: z.ZodType<OrderPlacedEventData> =
  z.object({
    slot: bigintLikeSchema,
    slotIndex: safeIntegerSchema,
    timestamp: bigintLikeSchema,
    symbol: z.string(),
    orderSequenceNumber: bigintLikeSchema,
    trader: z.string(),
    orderFlags: safeIntegerSchema,
    clientOrderId: z.string().nullable().optional(),
    orderId: z.string().nullable().optional(),
    price: bigintLikeSchema,
    quantity: bigintLikeSchema,
    side: z.enum(["bid", "ask"]),
    lastValidSlot: bigintLikeSchema.nullable().optional(),
    initialSlot: bigintLikeSchema,
    orderbookSequenceNumber: bigintLikeSchema,
  });

export interface GetNotificationsQuery {
  limit?: number;
  cursor?: string;
  unackedOnly?: boolean;
}

export const GetNotificationsQuerySchema: z.ZodType<GetNotificationsQuery> =
  z.object({
    limit: z.number().int().positive().max(100).optional(),
    cursor: z.string().optional(),
    unackedOnly: z.boolean().optional(),
  });

export const EVENT_NOTIFICATION_TYPES = [
  "order_filled",
  "liquidation",
  "backstop_liquidation",
  "adl",
  "risk_engine_cancel_order",
  "stop_loss_executed",
  "conditional_order_executed",
  "stop_loss_order_placed",
] as const;

export type EventNotificationType = (typeof EVENT_NOTIFICATION_TYPES)[number];

interface EventNotificationBase {
  source: "event";
  id: number;
  slot: number;
  slotIndex: number;
  instructionIndex: number;
  eventIndex: number;
  recipientIndex: number;
  createdAt: string;
  acked: boolean;
}

export interface OrderFilledDetails {
  type: "orderFilled";
  symbol: string;
  side: string;
  baseLotsFilled: number;
  price: number;
}

export interface RiskEngineCancelOrderDetails {
  type: "riskEngineCancelOrder";
  symbol: string;
  side: string;
  baseLotsReleased: number;
}

export interface StopLossExecutedDetails {
  type: "stopLossExecuted";
  symbol: string;
  side: string;
  baseAmount: number;
  quoteAmount: number;
  triggerType?: "take_profit" | "stop_loss";
}

export interface ConditionalOrderExecutedDetails {
  type: "conditionalOrderExecuted";
  symbol: string;
  side: string;
  baseAmount: number;
  quoteAmount: number;
  triggerType?: "take_profit" | "stop_loss";
}

export interface StopLossOrderPlacedDetails {
  type: "stopLossOrderPlaced";
  symbol: string;
  side: string;
  triggerType: "take_profit" | "stop_loss";
  /** Raw price in ticks. */
  priceInTicks: number;
  /** Signed by side. */
  baseLotsRemaining: number;
  /** Omitted when the order rested in full. */
  filledBaseAmount?: number;
  /** Omitted when the order rested in full. */
  filledQuoteAmount?: number;
}

export interface LiquidationDetails {
  type: "liquidation";
  symbol: string;
  side: string;
  baseAmount: number;
  quoteAmount: number;
}

export interface AdlDetails {
  type: "adl";
  assetId: number;
  baseLotsClosed: number;
}

export interface BackstopLiquidationDetails {
  type: "backstopLiquidation";
  assetId: number;
  baseLotsTransferred: number;
  virtualQuoteLotsTransferred: number;
  haircutRate: number;
}

export type OrderFilledNotification = EventNotificationBase & {
  notificationType: "order_filled";
  data: OrderFillEventData;
  details?: OrderFilledDetails;
};

export type LiquidationNotification = EventNotificationBase & {
  notificationType: "liquidation";
  data: TradeEventData;
  details?: LiquidationDetails;
};

export type BackstopLiquidationNotification = EventNotificationBase & {
  notificationType: "backstop_liquidation";
  data: LiquidationTransferEventData;
  details?: BackstopLiquidationDetails;
};

export type AdlNotification = EventNotificationBase & {
  notificationType: "adl";
  data: CloseMatchedPositionsEventData;
  details?: AdlDetails;
};

export type RiskEngineCancelOrderNotification = EventNotificationBase & {
  notificationType: "risk_engine_cancel_order";
  data: OrderModifiedEventData;
  details?: RiskEngineCancelOrderDetails;
};

export type StopLossExecutedNotification = EventNotificationBase & {
  notificationType: "stop_loss_executed";
  data: TradeEventData;
  details?: StopLossExecutedDetails;
};

export type ConditionalOrderExecutedNotification = EventNotificationBase & {
  notificationType: "conditional_order_executed";
  data: TradeEventData;
  details?: ConditionalOrderExecutedDetails;
};

export type StopLossOrderPlacedNotification = EventNotificationBase & {
  notificationType: "stop_loss_order_placed";
  data: OrderPlacedEventData;
  details?: StopLossOrderPlacedDetails;
};

/**
 * Event notification whose `notificationType` this SDK version does not know.
 * The original wire type is preserved in `rawNotificationType`.
 */
export type UnknownEventNotification = EventNotificationBase & {
  notificationType: "unknown";
  rawNotificationType: string;
  data: unknown;
  details?: unknown;
};

export type EventNotificationItem =
  | OrderFilledNotification
  | LiquidationNotification
  | BackstopLiquidationNotification
  | AdlNotification
  | RiskEngineCancelOrderNotification
  | StopLossExecutedNotification
  | ConditionalOrderExecutedNotification
  | StopLossOrderPlacedNotification
  | UnknownEventNotification;

export interface AdminNotificationItem {
  source: "admin";
  id: number;
  notificationType: string;
  title?: string | null;
  body: string | null;
  data: unknown;
  createdAt: string;
  acked: boolean;
}

export interface GeneralNotificationItem {
  source: "general";
  id: number;
  notificationType: string;
  title?: string | null;
  body: string | null;
  data: unknown;
  createdAt: string;
  acked: boolean;
}

export type NotificationItem =
  | EventNotificationItem
  | AdminNotificationItem
  | GeneralNotificationItem;

const EventNotificationBaseSchema = z.object({
  source: z.literal("event"),
  id: z.number(),
  slot: z.number(),
  slotIndex: z.number(),
  instructionIndex: z.number(),
  eventIndex: z.number(),
  recipientIndex: z.number(),
  createdAt: z.string(),
  acked: z.boolean(),
});

const OrderFilledDetailsSchema = z.object({
  type: z.literal("orderFilled"),
  symbol: z.string(),
  side: z.string(),
  baseLotsFilled: z.number(),
  price: z.number(),
});

const RiskEngineCancelOrderDetailsSchema = z.object({
  type: z.literal("riskEngineCancelOrder"),
  symbol: z.string(),
  side: z.string(),
  baseLotsReleased: z.number(),
});

const StopLossExecutedDetailsSchema = z.object({
  type: z.literal("stopLossExecuted"),
  symbol: z.string(),
  side: z.string(),
  baseAmount: z.number(),
  quoteAmount: z.number(),
  triggerType: z.enum(["take_profit", "stop_loss"]).optional(),
});

const ConditionalOrderExecutedDetailsSchema = z.object({
  type: z.literal("conditionalOrderExecuted"),
  symbol: z.string(),
  side: z.string(),
  baseAmount: z.number(),
  quoteAmount: z.number(),
  triggerType: z.enum(["take_profit", "stop_loss"]).optional(),
});

const StopLossOrderPlacedDetailsSchema = z.object({
  type: z.literal("stopLossOrderPlaced"),
  symbol: z.string(),
  side: z.string(),
  triggerType: z.enum(["take_profit", "stop_loss"]),
  priceInTicks: z.number(),
  baseLotsRemaining: z.number(),
  filledBaseAmount: z.number().optional(),
  filledQuoteAmount: z.number().optional(),
});

const LiquidationDetailsSchema = z.object({
  type: z.literal("liquidation"),
  symbol: z.string(),
  side: z.string(),
  baseAmount: z.number(),
  quoteAmount: z.number(),
});

const AdlDetailsSchema = z.object({
  type: z.literal("adl"),
  assetId: z.number(),
  baseLotsClosed: z.number(),
});

const BackstopLiquidationDetailsSchema = z.object({
  type: z.literal("backstopLiquidation"),
  assetId: z.number(),
  baseLotsTransferred: z.number(),
  virtualQuoteLotsTransferred: z.number(),
  haircutRate: z.number(),
});

const KnownEventNotificationItemSchema = z.discriminatedUnion(
  "notificationType",
  [
    EventNotificationBaseSchema.extend({
      notificationType: z.literal("order_filled"),
      data: OrderFillEventDataSchema,
      details: OrderFilledDetailsSchema.optional(),
    }),
    EventNotificationBaseSchema.extend({
      notificationType: z.literal("liquidation"),
      data: TradeEventDataSchema,
      details: LiquidationDetailsSchema.optional(),
    }),
    EventNotificationBaseSchema.extend({
      notificationType: z.literal("backstop_liquidation"),
      data: LiquidationTransferEventDataSchema,
      details: BackstopLiquidationDetailsSchema.optional(),
    }),
    EventNotificationBaseSchema.extend({
      notificationType: z.literal("adl"),
      data: CloseMatchedPositionsEventDataSchema,
      details: AdlDetailsSchema.optional(),
    }),
    EventNotificationBaseSchema.extend({
      notificationType: z.literal("risk_engine_cancel_order"),
      data: OrderModifiedEventDataSchema,
      details: RiskEngineCancelOrderDetailsSchema.optional(),
    }),
    EventNotificationBaseSchema.extend({
      notificationType: z.literal("stop_loss_executed"),
      data: TradeEventDataSchema,
      details: StopLossExecutedDetailsSchema.optional(),
    }),
    EventNotificationBaseSchema.extend({
      notificationType: z.literal("conditional_order_executed"),
      data: TradeEventDataSchema,
      details: ConditionalOrderExecutedDetailsSchema.optional(),
    }),
    EventNotificationBaseSchema.extend({
      notificationType: z.literal("stop_loss_order_placed"),
      data: OrderPlacedEventDataSchema,
      details: StopLossOrderPlacedDetailsSchema.optional(),
    }),
  ]
);

const KNOWN_EVENT_NOTIFICATION_TYPES: ReadonlySet<string> = new Set(
  EVENT_NOTIFICATION_TYPES
);

// Forward-compat fallback: event items with a notificationType this SDK does
// not know parse as "unknown" instead of failing the whole message/response.
// Known types that fail their typed schema still fail loudly.
const UnknownEventNotificationSchema: z.ZodType<UnknownEventNotification> =
  EventNotificationBaseSchema.extend({
    notificationType: z
      .string()
      .refine((value) => !KNOWN_EVENT_NOTIFICATION_TYPES.has(value), {
        message: "known notificationType must match its typed schema",
      }),
    data: z.unknown(),
    details: z.unknown().optional(),
  }).transform(({ notificationType, ...rest }) => ({
    ...rest,
    notificationType: "unknown" as const,
    rawNotificationType: notificationType,
  }));

const EventNotificationItemSchema = z.union([
  KnownEventNotificationItemSchema,
  UnknownEventNotificationSchema,
]);

const AdminNotificationItemSchema = z.object({
  source: z.literal("admin"),
  id: z.number(),
  notificationType: z.string(),
  title: z.string().nullable().optional(),
  body: z.string().nullable(),
  data: z.unknown(),
  createdAt: z.string(),
  acked: z.boolean(),
});

const GeneralNotificationItemSchema = z.object({
  source: z.literal("general"),
  id: z.number(),
  notificationType: z.string(),
  title: z.string().nullable().optional(),
  body: z.string().nullable(),
  data: z.unknown(),
  createdAt: z.string(),
  acked: z.boolean(),
});

export const NotificationItemSchema: z.ZodType<NotificationItem> = z.union([
  EventNotificationItemSchema,
  AdminNotificationItemSchema,
  GeneralNotificationItemSchema,
]) as z.ZodType<NotificationItem>;

export interface GetNotificationsResponse {
  items: NotificationItem[];
  nextCursor: string | null;
}

export const GetNotificationsResponseSchema: z.ZodType<GetNotificationsResponse> =
  z.object({
    items: z.array(NotificationItemSchema),
    nextCursor: z.string().nullable(),
  });

export interface AckBeforeTimestampBody {
  beforeTimestamp: string;
}

export const AckBeforeTimestampBodySchema: z.ZodType<AckBeforeTimestampBody> =
  z.object({
    beforeTimestamp: z.string(),
  });

export type AckNotificationItem =
  | {
      type: "event";
      id?: number;
      slot?: number;
      slotIndex?: number;
      instructionIndex?: number;
      eventIndex?: number;
      recipientIndex?: number;
    }
  | { type: "admin"; id: number }
  | { type: "general"; id: number };

export const AckNotificationItemSchema: z.ZodType<AckNotificationItem> =
  z.discriminatedUnion("type", [
    z.object({
      type: z.literal("event"),
      id: z.number().optional(),
      slot: z.number().optional(),
      slotIndex: z.number().optional(),
      instructionIndex: z.number().optional(),
      eventIndex: z.number().optional(),
      recipientIndex: z.number().optional(),
    }),
    z.object({
      type: z.literal("admin"),
      id: z.number(),
    }),
    z.object({
      type: z.literal("general"),
      id: z.number(),
    }),
  ]);

export interface AckNotificationsBody {
  items: AckNotificationItem[];
}

export const AckNotificationsBodySchema: z.ZodType<AckNotificationsBody> =
  z.object({
    items: z.array(AckNotificationItemSchema),
  });
