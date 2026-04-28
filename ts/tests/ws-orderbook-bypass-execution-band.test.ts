import { createL2BookAdapter } from "@/ws/adapters/l2-book/adapter";
import { createOrderbookAdapter } from "@/ws/adapters/orderbook/adapter";
import { createL2BookPlugin } from "@/ws/adapters/l2-book/plugin";
import { createOrderbookPlugin } from "@/ws/adapters/orderbook/plugin";
import type {
  SubscriptionMessage,
  WsChannelRegistration,
  WsClient,
} from "@/ws/types";
import { describe, expect, it } from "vitest";

const createMockWsClient = () => {
  const subscriptions = new Map<
    string,
    {
      subMsg: SubscriptionMessage;
      onMessage: (data: unknown) => void;
    }
  >();
  const channels = new Map<string, WsChannelRegistration>();

  const ws: WsClient = {
    subscribe(key, subMsg, onMessage) {
      subscriptions.set(key, { subMsg, onMessage });
    },
    unsubscribe(key) {
      subscriptions.delete(key);
    },
    registerChannel(channel, registration) {
      channels.set(channel, registration);
      return () => {
        channels.delete(channel);
      };
    },
    close() {},
    onServerError() {
      return () => {};
    },
  };

  return {
    ws,
    subscriptions,
  };
};

describe("orderbook bypassExecutionBand subscriptions", () => {
  const findSubscription = (
    subscriptions: Map<
      string,
      {
        subMsg: SubscriptionMessage;
        onMessage: (data: unknown) => void;
      }
    >,
    routingKey: string
  ) =>
    [...subscriptions.entries()].find(([key]) =>
      key.startsWith(`${routingKey}::listener:`)
    )?.[1];

  it("sends l2Book bypassExecutionBand in subscribe payload and key", () => {
    const mock = createMockWsClient();
    const l2Book = createL2BookAdapter(mock.ws);

    l2Book("SOL-PERP", { bypassExecutionBand: true });

    const subscription = findSubscription(
      mock.subscriptions,
      "l2Book:SOL-PERP:bypassExecutionBand"
    );
    expect(subscription?.subMsg).toEqual({
      type: "subscribe",
      subscription: {
        channel: "l2Book",
        coin: "SOL-PERP",
        bypassExecutionBand: true,
      },
    });
  });

  it("preserves explicit false bypassExecutionBand in l2Book payload", () => {
    const mock = createMockWsClient();
    const l2Book = createL2BookAdapter(mock.ws);

    l2Book("SOL-PERP", { bypassExecutionBand: false });

    const subscription = findSubscription(
      mock.subscriptions,
      "l2Book:SOL-PERP"
    );
    expect(subscription?.subMsg).toEqual({
      type: "subscribe",
      subscription: {
        channel: "l2Book",
        coin: "SOL-PERP",
        bypassExecutionBand: false,
      },
    });
  });

  it("keeps backwards-compatible l2Book call shape with AbortSignal", () => {
    const mock = createMockWsClient();
    const l2Book = createL2BookAdapter(mock.ws);
    const controller = new AbortController();

    l2Book("SOL-PERP", controller.signal);

    const subscription = findSubscription(
      mock.subscriptions,
      "l2Book:SOL-PERP"
    );
    expect(subscription?.subMsg).toEqual({
      type: "subscribe",
      subscription: {
        channel: "l2Book",
        coin: "SOL-PERP",
      },
    });
  });

  it("sends orderbook bypassExecutionBand in subscribe payload and key", () => {
    const mock = createMockWsClient();
    const orderbook = createOrderbookAdapter(mock.ws);

    orderbook("SOL-PERP", { bypassExecutionBand: true });

    const subscription = findSubscription(
      mock.subscriptions,
      "orderbook:SOL-PERP:bypassExecutionBand"
    );
    expect(subscription?.subMsg).toEqual({
      type: "subscribe",
      subscription: {
        channel: "orderbook",
        symbol: "SOL-PERP",
        bypassExecutionBand: true,
      },
    });
  });

  it("preserves explicit false bypassExecutionBand in orderbook payload", () => {
    const mock = createMockWsClient();
    const orderbook = createOrderbookAdapter(mock.ws);

    orderbook("SOL-PERP", { bypassExecutionBand: false });

    const subscription = findSubscription(
      mock.subscriptions,
      "orderbook:SOL-PERP"
    );
    expect(subscription?.subMsg).toEqual({
      type: "subscribe",
      subscription: {
        channel: "orderbook",
        symbol: "SOL-PERP",
        bypassExecutionBand: false,
      },
    });
  });

  it("routes plugin keys by bypassExecutionBand mode", () => {
    const l2BookPlugin = createL2BookPlugin();
    const orderbookPlugin = createOrderbookPlugin();

    expect(l2BookPlugin.getKey({ channel: "l2Book", coin: "SOL-PERP" })).toBe(
      "l2Book:SOL-PERP"
    );
    expect(
      l2BookPlugin.getKey({
        channel: "l2Book",
        coin: "SOL-PERP",
        bypassExecutionBand: true,
      })
    ).toBe("l2Book:SOL-PERP:bypassExecutionBand");

    expect(
      orderbookPlugin.getKey({ channel: "orderbook", symbol: "SOL-PERP" })
    ).toBe("orderbook:SOL-PERP");
    expect(
      orderbookPlugin.getKey({
        channel: "orderbook",
        symbol: "SOL-PERP",
        bypassExecutionBand: true,
      })
    ).toBe("orderbook:SOL-PERP:bypassExecutionBand");
  });
});
