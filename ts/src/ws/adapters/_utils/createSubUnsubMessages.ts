import type { SubscriptionMessage } from "@/ws/types";

export const createSubUnsubMessages = (
  topic: string,
  data: Record<string, unknown>
): {
  sub: SubscriptionMessage;
  unsub: SubscriptionMessage;
} => {
  const subscription = {
    channel: topic,
    ...data,
  };
  return {
    sub: {
      type: "subscribe",
      subscription,
    },
    unsub: {
      type: "unsubscribe",
      subscription,
    },
  };
};
