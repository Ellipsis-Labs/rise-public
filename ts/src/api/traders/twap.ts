import z from "zod";

/** A TWAP order snapshot. Sizes and cumulative fills are reported in lots. */
export interface TwapOrderSnapshot {
  traderAccount: string;
  traderAuthority: string;
  traderPdaIndex: number;
  traderSubaccountIndex: number;
  assetId: number;
  orderSequenceNumber: number;
  cooldownSlots: number;
  /** Total executions, including additional dust executions. */
  nChildOrders: number;
  /** Additional dust executions; absent on older servers. Zero also covers legacy final-child dust. */
  nDustOrders?: number;
  /** Size of each dust execution in base lots; null when unavailable. */
  dustOrderSize?: number | null;
  childOrdersExecuted: number;
  childOrdersRemaining: number;
  quoteLotsFilled: number;
  baseLotsFilled: number;
  dueSlot: number;
  lastExecutedSlot: number;
  lastValidSlot: number;
  latestSequenceNumber: number;
  childOrderMaxSlippageBps: number | null;
  childOrderMinPriceInTicks: number | null;
  childOrderMaxPriceInTicks: number | null;
  childOrderCollateralQuoteLotsToTransfer: number | null;
  transferCollateralAccountCount: number;
  side: string | null;
  priceInTicks: number | null;
  numBaseLots: number | null;
  numQuoteLots: number | null;
  minBaseLotsToFill: number | null;
  minQuoteLotsToFill: number | null;
  selfTradeBehavior: string | null;
  matchLimit: number | null;
  clientOrderId: string | null;
  lastValidSlotFromPacket: number | null;
  orderFlags: number | null;
  cancelExisting: boolean | null;
}

export const TwapOrderSnapshotSchema: z.ZodType<TwapOrderSnapshot> = z.object({
  traderAccount: z.string(),
  traderAuthority: z.string(),
  traderPdaIndex: z.number(),
  traderSubaccountIndex: z.number(),
  assetId: z.number(),
  orderSequenceNumber: z.number(),
  cooldownSlots: z.number(),
  nChildOrders: z.number(),
  nDustOrders: z.number().int().nonnegative().optional(),
  dustOrderSize: z.number().int().nonnegative().nullable().optional(),
  childOrdersExecuted: z.number(),
  childOrdersRemaining: z.number(),
  quoteLotsFilled: z.number(),
  baseLotsFilled: z.number(),
  dueSlot: z.number(),
  lastExecutedSlot: z.number(),
  lastValidSlot: z.number(),
  latestSequenceNumber: z.number(),
  childOrderMaxSlippageBps: z.number().nullable(),
  childOrderMinPriceInTicks: z.number().nullable(),
  childOrderMaxPriceInTicks: z.number().nullable(),
  childOrderCollateralQuoteLotsToTransfer: z.number().nullable(),
  transferCollateralAccountCount: z.number(),
  side: z.string().nullable(),
  priceInTicks: z.number().nullable(),
  numBaseLots: z.number().nullable(),
  numQuoteLots: z.number().nullable(),
  minBaseLotsToFill: z.number().nullable(),
  minQuoteLotsToFill: z.number().nullable(),
  selfTradeBehavior: z.string().nullable(),
  matchLimit: z.number().nullable(),
  clientOrderId: z.string().nullable(),
  lastValidSlotFromPacket: z.number().nullable(),
  orderFlags: z.number().nullable(),
  cancelExisting: z.boolean().nullable(),
});

export interface TwapAccountSnapshot {
  twapAccount: string;
  traderSubaccountIndex: number;
  assetId: number;
  sequenceNumber: number;
  order?: TwapOrderSnapshot;
}

export const TwapAccountSnapshotSchema: z.ZodType<TwapAccountSnapshot> =
  z.object({
    twapAccount: z.string(),
    traderSubaccountIndex: z.number(),
    assetId: z.number(),
    sequenceNumber: z.number(),
    order: TwapOrderSnapshotSchema.optional(),
  });

export interface TwapSnapshot {
  authority: string;
  traderPdaIndex: number;
  slot: number;
  accounts: TwapAccountSnapshot[];
  terminalEvents?: TwapTerminalEvent[];
}

export interface TwapTerminalEvent {
  eventId: string;
  twapAccount: string;
  orderSequenceNumber: number;
  errorCode: string;
}

export const TwapTerminalEventSchema: z.ZodType<TwapTerminalEvent> = z.object({
  eventId: z.string(),
  twapAccount: z.string(),
  orderSequenceNumber: z.number(),
  errorCode: z.string(),
});

export const TwapSnapshotSchema: z.ZodType<TwapSnapshot> = z.object({
  authority: z.string(),
  traderPdaIndex: z.number(),
  slot: z.number(),
  accounts: z.array(TwapAccountSnapshotSchema),
  terminalEvents: z.array(TwapTerminalEventSchema).optional(),
});

/** Filters for a trader's TWAP accounts. The PDA index defaults to zero. */
export interface TwapOrdersQueryParams {
  traderPdaIndex?: number;
  traderSubaccountIndex?: number;
  assetId?: number;
}
