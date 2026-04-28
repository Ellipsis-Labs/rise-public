import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getStringField } from "../_utils/messageUtils";

export const createMarketStatsPlugin = (): MessageHandlerPlugin => ({
  channel: "marketStats",
  validate: (message: unknown): boolean => {
    return getStringField(message, "symbol") !== null;
  },
  getKey: (message: unknown): string => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid MarketStats message: missing symbol");
    }
    return `marketStats:${symbol}`;
  },
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid MarketStats message: missing symbol");
    }

    const symbolKey = `marketStats:${symbol}`;
    const symbolSub = registry.get(symbolKey);
    if (symbolSub) {
      symbolSub.onMsg(message);
      return;
    }

    const globalSub = registry.get("marketStats");
    if (globalSub) {
      globalSub.onMsg(message);
    }
  },
});
