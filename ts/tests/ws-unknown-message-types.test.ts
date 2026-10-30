import { createExchangeAdapter } from "@/ws/adapters/exchange/adapter";
import { createTraderStateAdapter } from "@/ws/adapters/trader-state/adapter";
import type { WsClient } from "@/ws/types";
import { describe, expect, it, vi } from "vitest";

describe("adapters backed by external transports", () => {
  it.each(["exchange", "traderState"])(
    "%s skips unfamiliar messages and delivers the following delta",
    async (channel) => {
      let onMessage: (raw: unknown) => void = () => {
        throw new Error("No active subscription");
      };
      const unsubscribe = vi.fn();
      const close = vi.fn();
      const ws: WsClient = {
        subscribe(_key, _sub, listener) {
          onMessage = listener;
        },
        unsubscribe,
        close,
        registerChannel() {
          return () => {};
        },
        onServerError() {
          return () => {};
        },
      };
      const errors = vi.spyOn(console, "error").mockImplementation(() => {});
      const abort = new AbortController();
      try {
        const updates =
          channel === "exchange"
            ? createExchangeAdapter(ws)(abort.signal)
            : createTraderStateAdapter(ws)("authority", 0, abort.signal);
        const next = updates[Symbol.asyncIterator]().next();
        onMessage({ channel, messageType: "futureMessage", futureField: true });
        onMessage({
          channel,
          messageType: "delta",
          authority: "authority",
          traderPdaIndex: 0,
          version: 1,
          slot: "11",
          slotIndex: 0,
          sequenceNumber: "2",
          ops: [],
          deltas: [],
        });
        await expect(next).resolves.toMatchObject({
          done: false,
          value: { messageType: "delta", slot: 11n },
        });
        expect(unsubscribe).not.toHaveBeenCalled();
        expect(close).not.toHaveBeenCalled();
        expect(errors).not.toHaveBeenCalled();
      } finally {
        abort.abort();
        errors.mockRestore();
      }
    }
  );
});
