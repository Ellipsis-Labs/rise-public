import { createCandlesPlugin } from "@/ws/adapters/candles/plugin";
import { createFillsPlugin } from "@/ws/adapters/fills/plugin";
import { createFundingRatePlugin } from "@/ws/adapters/funding-rate/plugin";
import { createL2BookPlugin } from "@/ws/adapters/l2-book/plugin";
import { createMarkPricePlugin } from "@/ws/adapters/mark-price/plugin";
import { createMarketPlugin } from "@/ws/adapters/market/plugin";
import { createOrderbookPlugin } from "@/ws/adapters/orderbook/plugin";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import type { Subscription } from "@/ws/types";
import { describe, expect, it, vi } from "vitest";

describe("mixed-case market symbol routing", () => {
  it.each<{
    plugin: MessageHandlerPlugin;
    message: Record<string, unknown>;
    key: string;
  }>([
    {
      plugin: createMarketPlugin(),
      message: { symbol: "kBONK" },
      key: "market:kbonk",
    },
    {
      plugin: createFundingRatePlugin(),
      message: { symbol: "kBONK" },
      key: "fundingRate:kbonk",
    },
    {
      plugin: createMarkPricePlugin(),
      message: { symbol: "kBONK" },
      key: "markPrice:kbonk",
    },
    {
      plugin: createFillsPlugin(),
      message: { symbol: "kBONK", fills: [] },
      key: "fills:kbonk",
    },
    {
      plugin: createCandlesPlugin(),
      message: { symbol: "kBONK", timeframe: "1m" },
      key: "candles:kbonk:1m",
    },
    {
      plugin: createOrderbookPlugin(),
      message: { symbol: "kBONK" },
      key: "orderbook:kbonk",
    },
    {
      plugin: createL2BookPlugin(),
      message: { coin: "kBONK" },
      key: "l2Book:kbonk",
    },
  ])(
    "routes $key from a canonical payload",
    async ({ plugin, message, key }) => {
      const onMsg = vi.fn();
      const registry = new Map<string, Subscription>([[key, { onMsg }]]);
      expect(plugin.getKey(message)).toBe(key);
      await plugin.handle(message, registry);
      expect(onMsg).toHaveBeenCalledWith(message);
    }
  );
});
