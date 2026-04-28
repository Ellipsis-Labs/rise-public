import type { NotificationItem } from "@/api/notifications/types";

export type NotificationsSubscriptionParams = {
  userId?: string;
  authority?: string;
};

export type NotificationsPort = (
  params: NotificationsSubscriptionParams,
  signal?: AbortSignal
) => AsyncIterable<NotificationItem>;
