import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";

export const createExchangeStatusPlugin = (): MessageHandlerPlugin => ({
  channel: "exchangeStatus",
  validate: (message: unknown): boolean => {
    return typeof message === "object" && message !== null;
  },
  getKey: (): string => "exchangeStatus",
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const sub = registry.get("exchangeStatus");
    sub?.onMsg(message);
  },
});
