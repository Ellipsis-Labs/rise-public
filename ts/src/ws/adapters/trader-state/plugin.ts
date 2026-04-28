import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getNumberField, getStringField } from "../_utils/messageUtils";
import { buildTraderStateSubscriptionKey } from "./adapter";

export const createTraderStatePlugin = (): MessageHandlerPlugin => ({
  channel: "traderState",
  validate: (message: unknown): boolean => {
    return (
      getStringField(message, "authority") !== null &&
      getNumberField(message, "traderPdaIndex") !== null
    );
  },
  getKey: (message: unknown): string => {
    const authority = getStringField(message, "authority");
    const traderPdaIndex = getNumberField(message, "traderPdaIndex");
    if (!authority || traderPdaIndex === null) {
      throw new Error("Invalid TraderState message: missing fields");
    }
    return buildTraderStateSubscriptionKey(authority, traderPdaIndex);
  },
  handle: async (message: unknown, registry: Map<string, Subscription>) => {
    const authority = getStringField(message, "authority");
    const traderPdaIndex = getNumberField(message, "traderPdaIndex");
    if (!authority || traderPdaIndex === null) {
      throw new Error("Invalid TraderState message: missing fields");
    }
    const key = buildTraderStateSubscriptionKey(authority, traderPdaIndex);
    const sub = registry.get(key);
    sub?.onMsg(message);
  },
});
