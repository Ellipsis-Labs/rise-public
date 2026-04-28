import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getStringField } from "../_utils/messageUtils";

export const createCandlesPlugin = (): MessageHandlerPlugin => ({
  channel: "candle",
  validate: (message: unknown): boolean => {
    return (
      getStringField(message, "symbol") !== null &&
      getStringField(message, "timeframe") !== null
    );
  },
  getKey: (message: unknown): string => {
    const symbol = getStringField(message, "symbol");
    const timeframe = getStringField(message, "timeframe");
    if (!symbol || !timeframe) {
      throw new Error("Invalid Candle message: missing symbol/timeframe");
    }
    return `candles:${symbol}:${timeframe}`;
  },
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const symbol = getStringField(message, "symbol");
    const timeframe = getStringField(message, "timeframe");
    if (!symbol || !timeframe) {
      throw new Error("Invalid Candle message: missing symbol/timeframe");
    }
    const key = `candles:${symbol}:${timeframe}`;
    const sub = registry.get(key);
    sub?.onMsg(message);
  },
});
