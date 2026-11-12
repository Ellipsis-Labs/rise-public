import z from "zod";
import type { MessageHandlerPlugin } from "../../plugins/types";
import type { WsSubscriptionStatusMessage } from "../../types";

export const WsSubscriptionStatusMessageSchema: z.ZodType<WsSubscriptionStatusMessage> =
  z.object({
    channel: z.literal("subscriptionStatus"),
    status: z.string(),
    subscription: z.record(z.string(), z.unknown()),
    clientId: z.string().optional(),
  });

export const createSubscriptionStatusPlugin = (
  onSubscriptionStatusFrame?: (message: WsSubscriptionStatusMessage) => void
): MessageHandlerPlugin => ({
  channel: "subscriptionStatus",
  validate: (message: unknown): boolean =>
    WsSubscriptionStatusMessageSchema.safeParse(message).success,
  getKey: (): string => "",
  handle: async (message: unknown): Promise<void> => {
    const parsed = WsSubscriptionStatusMessageSchema.safeParse(message);
    if (parsed.success) {
      onSubscriptionStatusFrame?.(parsed.data);
    }
  },
});
