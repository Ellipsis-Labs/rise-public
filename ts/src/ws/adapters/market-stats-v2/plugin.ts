import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { isRecord } from "../_utils/messageUtils";
import { buildMarketStatsV2RoutingKey } from "./routing";

const getRoutingKey = (message: unknown): string | null => {
  if (!isRecord(message) || !Array.isArray(message.stats)) {
    return null;
  }

  if (message.symbols === undefined) {
    return buildMarketStatsV2RoutingKey();
  }
  if (
    !Array.isArray(message.symbols) ||
    !message.symbols.every(
      (symbol): symbol is string => typeof symbol === "string"
    )
  ) {
    return null;
  }
  try {
    return buildMarketStatsV2RoutingKey(message.symbols);
  } catch {
    return null;
  }
};

const requireRoutingKey = (message: unknown): string => {
  const key = getRoutingKey(message);
  if (key === null) {
    throw new Error("Invalid MarketStatsV2 message routing fields");
  }
  return key;
};

export const createMarketStatsV2Plugin = (): MessageHandlerPlugin => ({
  channel: "marketStatsV2",
  validate: (message: unknown): boolean => getRoutingKey(message) !== null,
  getKey: requireRoutingKey,
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    registry.get(requireRoutingKey(message))?.onMsg(message);
  },
});
