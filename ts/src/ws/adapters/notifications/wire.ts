import z from "zod";
import {
  NotificationItemSchema,
  type NotificationItem,
} from "@/api/notifications/types";

export type NotificationMsg = {
  channel: "notification";
  payload: NotificationItem;
  authority: string;
  traderPubkey: string;
  traderPdaIndex: number;
};

export const NotificationMsgSchema: z.ZodType<NotificationMsg> = z.object({
  channel: z.literal("notification"),
  payload: NotificationItemSchema,
  authority: z.string(),
  traderPubkey: z.string(),
  traderPdaIndex: z.number(),
});

export type NotificationUpdate = NotificationItem;
