import { marketSymbolKey } from "../_utils/marketSymbol";
import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getStringField } from "../_utils/messageUtils";

export const createMarkPricePlugin = (): MessageHandlerPlugin => ({
  channel: "markPrice",
  validate: (message: unknown): boolean => {
    return getStringField(message, "symbol") !== null;
  },
  getKey: (message: unknown): string => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid MarkPrice message: missing symbol");
    }
    return `markPrice:${marketSymbolKey(symbol)}`;
  },
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid MarkPrice message: missing symbol");
    }

    const key = `markPrice:${marketSymbolKey(symbol)}`;
    const sub = registry.get(key);
    if (sub) {
      sub.onMsg(message);
    }
  },
});
