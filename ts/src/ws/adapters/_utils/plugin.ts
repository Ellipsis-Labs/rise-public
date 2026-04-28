import type { MessageHandlerPlugin } from "../../plugins/types";

export const createSubscriptionStatusPlugin = (): MessageHandlerPlugin => ({
  channel: "subscriptionStatus",
  validate: (): boolean => true,
  getKey: (): string => "",
  handle: async (): Promise<void> => {
    return;
  },
});
