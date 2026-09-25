import { marketSymbolKey } from "../_utils/marketSymbol";
import type { Subscription } from "@/ws/types";
import type { MessageHandlerPlugin } from "@/ws/plugins/types";
import { getStringField, isRecord } from "@/ws/adapters/_utils/messageUtils";

const getSymbol = (message: unknown): string | null =>
  getStringField(message, "symbol");

const hasFillsArray = (message: unknown): boolean =>
  isRecord(message) && Array.isArray(message.fills);

export const createFillsPlugin = (): MessageHandlerPlugin => ({
  channel: "fills",
  validate: (message: unknown): boolean =>
    getSymbol(message) !== null && hasFillsArray(message),
  getKey: (message: unknown): string => {
    const symbol = getSymbol(message);
    if (!symbol) {
      throw new Error("Invalid Fills message: missing symbol");
    }
    return `fills:${marketSymbolKey(symbol)}`;
  },
  handle: async (
    message: unknown,
    registry: Map<string, Subscription>
  ): Promise<void> => {
    const symbol = getSymbol(message);
    if (!symbol || !hasFillsArray(message)) {
      throw new Error("Invalid Fills message: missing symbol or fills array");
    }

    registry.get(`fills:${marketSymbolKey(symbol)}`)?.onMsg(message);
    registry.get("fills")?.onMsg(message);
  },
});
