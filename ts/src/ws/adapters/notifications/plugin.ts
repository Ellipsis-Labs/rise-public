import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "../../plugins/types";
import {
  getBooleanField,
  getNumberField,
  getStringField,
  isRecord,
} from "../_utils/messageUtils";

const isNotificationMessage = (message: unknown): boolean => {
  if (!isRecord(message)) return false;
  const payload = message.payload;
  if (!isRecord(payload)) return false;

  return (
    getStringField(payload, "source") !== null &&
    getNumberField(payload, "id") !== null &&
    getStringField(payload, "notificationType") !== null &&
    getBooleanField(payload, "acked") !== null &&
    getStringField(payload, "createdAt") !== null
  );
};

export const createNotificationsPlugin = (): MessageHandlerPlugin => ({
  channel: "notification",
  validate: (message: unknown): boolean => isNotificationMessage(message),
  getKey: (): string => "notification",
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    if (!isNotificationMessage(message)) {
      throw new Error("Invalid Notification message");
    }

    for (const [key, sub] of registry.entries()) {
      if (key.startsWith("notifications:")) {
        sub.onMsg(message);
      }
    }
  },
});
