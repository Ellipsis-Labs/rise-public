import z from "zod";

export interface ExchangeStatusPayload {
  exchangeStatusBits: number;
  previousExchangeStatusBits: number;
  active: boolean;
  gated: boolean;
  authority: string;
  [key: string]: unknown;
}

type ExchangeStatusBase = {
  id: string;
  notificationType: string;
  data: ExchangeStatusPayload;
  isRead: boolean;
  createdAt: string;
  traderId?: number | null;
  readAt?: string | null;
};

export type ExchangeStatusUpdate = ExchangeStatusBase;

export type ExchangeStatusMsg = ExchangeStatusBase & {
  channel: "exchangeStatus";
};

const ExchangeStatusPayloadSchema: z.ZodType<ExchangeStatusPayload> = z
  .object({
    exchangeStatusBits: z
      .number()
      .int()
      .refine((n) => Number.isSafeInteger(n) && n >= 0 && n <= 0xff, {
        message: "exchangeStatusBits must be a u8",
      }),
    previousExchangeStatusBits: z
      .number()
      .int()
      .refine((n) => Number.isSafeInteger(n) && n >= 0 && n <= 0xff, {
        message: "previousExchangeStatusBits must be a u8",
      }),
    active: z.boolean(),
    gated: z.boolean(),
    authority: z.string(),
  })
  .passthrough();

export const ExchangeStatusMsgSchema: z.ZodType<ExchangeStatusMsg> = z.object({
  channel: z.literal("exchangeStatus"),
  id: z.string(),
  notificationType: z.string(),
  data: ExchangeStatusPayloadSchema,
  isRead: z.boolean(),
  createdAt: z.string(),
  traderId: z.number().nullable().optional(),
  readAt: z.string().nullable().optional(),
});
