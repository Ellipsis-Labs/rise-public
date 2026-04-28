import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getStringField } from "../_utils/messageUtils";

export const createFundingRatePlugin = (): MessageHandlerPlugin => ({
  channel: "fundingRate",
  validate: (message: unknown): boolean => {
    return getStringField(message, "symbol") !== null;
  },
  getKey: (message: unknown): string => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid FundingRate message: missing symbol");
    }
    return `fundingRate:${symbol}`;
  },
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const symbol = getStringField(message, "symbol");
    if (!symbol) {
      throw new Error("Invalid FundingRate message: missing symbol");
    }

    registry.get(`fundingRate:${symbol}`)?.onMsg(message);
  },
});
