import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getBooleanField, getStringField } from "../_utils";
import { buildL2BookSubscriptionKey } from "./adapter";

export const createL2BookPlugin = (): MessageHandlerPlugin => ({
  channel: "l2Book",
  validate: (message: unknown): boolean => {
    return getStringField(message, "coin") !== null;
  },
  getKey: (message: unknown): string => {
    const coin = getStringField(message, "coin");
    if (!coin) {
      throw new Error("Invalid L2Book message: missing coin");
    }
    return buildL2BookSubscriptionKey(
      coin,
      getBooleanField(message, "bypassExecutionBand") ?? undefined
    );
  },
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const coin = getStringField(message, "coin");
    if (!coin) {
      throw new Error("Invalid L2Book message: missing coin");
    }

    const key = buildL2BookSubscriptionKey(
      coin,
      getBooleanField(message, "bypassExecutionBand") ?? undefined
    );
    registry.get(key)?.onMsg(message);
  },
});
