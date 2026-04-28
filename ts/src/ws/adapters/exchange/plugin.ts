import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";

export const createExchangePlugin = (): MessageHandlerPlugin => ({
  channel: "exchange",
  validate: (message: unknown): boolean => {
    return typeof message === "object" && message !== null;
  },
  getKey: (): string => "exchange",
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const sub = registry.get("exchange");
    sub?.onMsg(message);
  },
});
