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
  createNotificationsPlugin,
  createOrderbookPlugin,
  createSubscriptionStatusPlugin,
  createTraderStatePlugin,
} from "../adapters";
import type { WsServerErrorMessage } from "../types";
import type { PluginFactory } from "./types";

type PluginOptions = {
  onServerErrorFrame?: (message: WsServerErrorMessage) => void;
};

const createErrorPlugin = (
  onServerErrorFrame?: (message: WsServerErrorMessage) => void
): PluginFactory => {
  return () => ({
    channel: "error",
    validate: (message: unknown): boolean => {
      if (!message || typeof message !== "object") return false;
      const candidate = message as { error?: unknown; code?: unknown };
      return (
        typeof candidate.error === "string" &&
        typeof candidate.code === "number"
      );
    },
    getKey: (): string => "error",
    handle: async (message: unknown): Promise<void> => {
      const candidate = message as { error: string; code: number };
      onServerErrorFrame?.({
        channel: "error",
        error: candidate.error,
        code: candidate.code,
      });
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
  createNotificationsPlugin,
  createTraderStatePlugin,
  createSubscriptionStatusPlugin,
  createErrorPlugin(options.onServerErrorFrame),
];
