import z from "zod";
import {
  createAllMidsPlugin,
  createCandlesPlugin,
  createExchangePlugin,
  createExchangeStatusPlugin,
  createFillsPlugin,
  createFundingRatePlugin,
  createL2BookPlugin,
  createMarkPricePlugin,
  createMarketPlugin,
  createMarketStatsPlugin,
  createMarketStatsV2Plugin,
  createNotificationsPlugin,
  createOrderbookPlugin,
  createSubscriptionStatusPlugin,
  createTraderStatePlugin,
} from "../adapters";
import type {
  WsServerErrorMessage,
  WsSubscriptionStatusMessage,
} from "../types";
import type { PluginFactory } from "./types";

/**
 * A server `error` frame. `subscription` is the server's echo of the
 * subscribe or unsubscribe request the error rejects; servers that predate
 * the echo omit it.
 */
export const WsServerErrorMessageSchema: z.ZodType<WsServerErrorMessage> =
  z.object({
    channel: z.literal("error"),
    error: z.string(),
    code: z.number(),
    subscription: z.record(z.string(), z.unknown()).optional(),
  });

type PluginOptions = {
  onServerErrorFrame?: (message: WsServerErrorMessage) => void;
  onSubscriptionStatusFrame?: (message: WsSubscriptionStatusMessage) => void;
};

const createErrorPlugin = (
  onServerErrorFrame?: (message: WsServerErrorMessage) => void
): PluginFactory => {
  return () => ({
    channel: "error",
    validate: (message: unknown): boolean =>
      WsServerErrorMessageSchema.safeParse(message).success,
    getKey: (): string => "error",
    handle: async (message: unknown): Promise<void> => {
      const parsed = WsServerErrorMessageSchema.safeParse(message);
      if (parsed.success) {
        onServerErrorFrame?.(parsed.data);
      }
    },
  });
};

export const createDefaultPlugins = (
  options: PluginOptions = {}
): PluginFactory[] => [
  createAllMidsPlugin,
  createFundingRatePlugin,
  createMarketPlugin,
  createOrderbookPlugin,
  createL2BookPlugin,
  createCandlesPlugin,
  createExchangePlugin,
  createExchangeStatusPlugin,
  createFillsPlugin,
  createMarkPricePlugin,
  createMarketStatsPlugin,
  createMarketStatsV2Plugin,
  createNotificationsPlugin,
  createTraderStatePlugin,
  () => createSubscriptionStatusPlugin(options.onSubscriptionStatusFrame),
  createErrorPlugin(options.onServerErrorFrame),
];
