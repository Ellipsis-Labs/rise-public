import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getStringField } from "../_utils/messageUtils";

export const createMarketPlugin = (): MessageHandlerPlugin => ({
  channel: "market",
  validate: (message: unknown): boolean => {
    return getStringField(message, "symbol") !== null;
  },
  getKey: (message: unknown): string => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid Market message: missing symbol");
    }
    return `market:${symbol}`;
  },
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid Market message: missing symbol");
    }

    registry.get(`market:${symbol}`)?.onMsg(message);
  },
});
