import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";

export const createAllMidsPlugin = (): MessageHandlerPlugin => ({
  channel: "allMids",
  validate: (message: unknown): boolean =>
    typeof message === "object" && message !== null,
  getKey: (): string => "allMids",
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    registry.get("allMids")?.onMsg(message);
  },
});
