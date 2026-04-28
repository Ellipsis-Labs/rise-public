import {
  registerSubscription,
  type SubscriptionMessage,
  type WsChannelRegistration,
  type WsClient,
} from "@/index";
import { describe, expect, it } from "vitest";
import z from "zod";

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
      if (channels.has(channel)) {
        throw new Error(`duplicate channel: ${channel}`);
      }
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
    channels,
  };
};

const TraderVolumeMessageSchema = z.object({
  channel: z.literal("traderVolume"),
  authority: z.string(),
  traderPdaIndex: z.number(),
  slot: z.number(),
  markets: z.array(
    z.object({
      symbol: z.string(),
      makerVolumeQuoteLots: z.string(),
      takerVolumeQuoteLots: z.string(),
    })
  ),
});

const getTraderVolumeKey = (message: {
  authority: string;
  traderPdaIndex: number;
}) => `traderVolume:${message.authority}:${message.traderPdaIndex}`;

describe("registerSubscription", () => {
  it("registers a custom message channel and returns an async stream adapter", async () => {
    const mock = createMockWsClient();

    const traderVolume = registerSubscription(mock.ws, {
      subscriptionChannel: "traderVolume",
      schema: TraderVolumeMessageSchema,
      buildKey: (authority: string, traderPdaIndex: number) =>
        `traderVolume:${authority}:${traderPdaIndex}`,
      buildSubParams: (authority: string, traderPdaIndex: number) => ({
        authority,
        traderPdaIndex,
      }),
      getKeyFromMessage: getTraderVolumeKey,
      processMessage: (message, [authority, traderPdaIndex]) => {
        if (
          message.authority !== authority ||
          message.traderPdaIndex !== traderPdaIndex
        ) {
          return null;
        }
        return {
          authority: message.authority,
          traderPdaIndex: message.traderPdaIndex,
          slot: message.slot,
          marketCount: message.markets.length,
        };
      },
    });

    const stream = traderVolume("auth-1", 2);
    const iterator = stream[Symbol.asyncIterator]();

    const [[subscriptionKey, subscription] = []] = [
      ...mock.subscriptions.entries(),
    ];
    expect(subscriptionKey).toMatch(/^traderVolume:auth-1:2::listener:/);
    expect(subscription?.subMsg).toEqual({
      type: "subscribe",
      subscription: {
        channel: "traderVolume",
        authority: "auth-1",
        traderPdaIndex: 2,
      },
    });

    const channel = mock.channels.get("traderVolume");
    expect(channel).toBeDefined();

    const rawMessage = {
      channel: "traderVolume" as const,
      authority: "auth-1",
      traderPdaIndex: 2,
      slot: 91,
      markets: [
        {
          symbol: "SOL",
          makerVolumeQuoteLots: "1",
          takerVolumeQuoteLots: "2",
        },
      ],
    };

    expect(channel?.validate(rawMessage)).toBe(true);
    expect(channel?.getKey(rawMessage)).toBe("traderVolume:auth-1:2");

    subscription?.onMessage(rawMessage);

    await expect(iterator.next()).resolves.toEqual({
      done: false,
      value: {
        authority: "auth-1",
        traderPdaIndex: 2,
        slot: 91,
        marketCount: 1,
      },
    });
  });

  it("reuses the same custom channel for repeated one-at-a-time registrations", () => {
    const mock = createMockWsClient();

    const definition = {
      subscriptionChannel: "traderVolume",
      schema: TraderVolumeMessageSchema,
      buildKey: (authority: string, traderPdaIndex: number) =>
        `traderVolume:${authority}:${traderPdaIndex}`,
      buildSubParams: (authority: string, traderPdaIndex: number) => ({
        authority,
        traderPdaIndex,
      }),
      getKeyFromMessage: getTraderVolumeKey,
      processMessage: (message: z.infer<typeof TraderVolumeMessageSchema>) => ({
        authority: message.authority,
        traderPdaIndex: message.traderPdaIndex,
        slot: message.slot,
      }),
    };

    registerSubscription(mock.ws, definition);
    registerSubscription(mock.ws, definition);

    expect(mock.channels.size).toBe(1);
  });
});
