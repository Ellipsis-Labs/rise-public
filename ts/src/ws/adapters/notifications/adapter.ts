import type { WsClient } from "@/ws/types";
import { createUpdateStream } from "@/ws/adapters/_utils";
import { applyStrictModeRecursive } from "@/ws/zodStrictMode";
import type {
  NotificationsPort,
  NotificationsSubscriptionParams,
} from "./ports";
import type { NotificationMsg, NotificationUpdate } from "./wire";
import { NotificationMsgSchema } from "./wire";

export type NotificationsAdapter = NotificationsPort;

export type { NotificationsSubscriptionParams } from "./ports";

export interface NotificationsAdapterOptions {
  buffer?: number;
}

const buildKey = (params: NotificationsSubscriptionParams): string => {
  const { userId, authority } = params;
  if (userId != null && authority != null) {
    throw new Error(
      "Notifications subscription requires exactly one of userId or authority"
    );
  }
  if (userId != null) return `notifications:privy:${userId}`;
  if (authority != null) return `notifications:authority:${authority}`;
  throw new Error(
    "Notifications subscription requires exactly one of userId or authority"
  );
};

const buildSubParams = (
  params: NotificationsSubscriptionParams
): Record<string, unknown> => {
  const { userId, authority } = params;
  if (userId != null && authority != null) {
    throw new Error(
      "Notifications subscription requires exactly one of userId or authority"
    );
  }
  if (userId != null) return { userId };
  if (authority != null) return { authority };
  throw new Error(
    "Notifications subscription requires exactly one of userId or authority"
  );
};

export const createNotificationsAdapter = (
  ws: WsClient,
  opts?: NotificationsAdapterOptions,
  strictMode?: boolean
): NotificationsAdapter => {
  const schema = strictMode
    ? applyStrictModeRecursive(NotificationMsgSchema)
    : NotificationMsgSchema;

  return createUpdateStream<
    NotificationMsg,
    NotificationUpdate,
    [NotificationsSubscriptionParams]
  >(
    ws,
    {
      channel: "notifications",
      schema,
      buildKey,
      buildSubParams,
      processMessage: (message) => message.payload,
      schemaErrorMessage: "Failed to parse Notification message",
    },
    opts
  );
};
