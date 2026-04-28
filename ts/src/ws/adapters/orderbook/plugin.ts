import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getBooleanField, getStringField } from "../_utils";
import { buildOrderbookSubscriptionKey } from "./adapter";

export const createOrderbookPlugin = (): MessageHandlerPlugin => ({
  channel: "orderbook",
  validate: (message: unknown): boolean => {
    return getStringField(message, "symbol") !== null;
  },
  getKey: (message: unknown): string => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid Orderbook message: missing symbol");
    }
    return buildOrderbookSubscriptionKey(
      symbol,
      getBooleanField(message, "bypassExecutionBand") ?? undefined
    );
  },
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid Orderbook message: missing symbol");
    }

    const key = buildOrderbookSubscriptionKey(
      symbol,
      getBooleanField(message, "bypassExecutionBand") ?? undefined
    );
    registry.get(key)?.onMsg(message);
  },
});
