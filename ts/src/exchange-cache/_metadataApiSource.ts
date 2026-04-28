import { debugRise, summarizeDebugError } from "@/internal/_debug";
import { ExchangeCacheSequenceError } from "./store";
import type { ExchangeCacheHealthReason } from "./types";
import {
  DEFAULT_API_RESYNC_BACKOFF_MS,
  delay,
  type ExchangeMetadataSourceContext,
} from "./_metadataShared";

type ApiSourceResult = "fallbackToRpc" | null;
type StreamRestartReason = "streamEnded" | "streamError" | null;

export interface ApiSource {
  start(): Promise<ApiSourceResult>;
  close(): void;
}

export const createApiSource = (
  context: ExchangeMetadataSourceContext
): ApiSource => {
  let currentAbort: AbortController | null = null;

  const loadSnapshot = async (
    reason: ExchangeCacheHealthReason
  ): Promise<void> => {
    debugRise("cache", "exchange.api.snapshot:start", { reason });
    const snapshot = await context.config.api!.api.getSnapshot();
    context.installSnapshot({
      snapshot,
      source: "http",
      active: "api",
    });
    context.transitionHealth("bootstrapped", reason);
    debugRise("cache", "exchange.api.snapshot:done", {
      reason,
      slot: snapshot.slot.toString(),
      slotIndex: snapshot.slotIndex,
      marketCount: snapshot.markets.length,
    });
  };

  const consumeStream = async (): Promise<StreamRestartReason> => {
    const exchange = context.config.api?.exchange;
    if (!exchange) {
      return null;
    }

    const controller = new AbortController();
    currentAbort = controller;
    const stream = exchange(controller.signal);
    let awaitingSnapshot = false;
    let refreshPending = false;

    debugRise("cache", "exchange.api.stream:start", {
      liveUpdates: context.config.api?.liveUpdates,
    });

    try {
      for await (const message of stream) {
        if (context.isClosed() || controller.signal.aborted) {
          return null;
        }

        if (message.messageType === "snapshot") {
          context.cacheStore.applySnapshotMessage(message);
          context.markSourceSynced();
          awaitingSnapshot = false;
          refreshPending = false;
          context.transitionHealth("live", "websocketSnapshot");
          debugRise("cache", "exchange.api.stream:snapshot", {
            slot: message.slot.toString(),
            slotIndex: message.slotIndex,
          });
          continue;
        }

        if (awaitingSnapshot) {
          if (!refreshPending) {
            refreshPending = true;
            debugRise("cache", "exchange.api.stream:refresh", {
              reason: "awaitingSnapshot",
            });
            stream.refresh();
          }
          continue;
        }

        try {
          context.cacheStore.applyDelta(message);
          context.markSourceSynced();
        } catch (error) {
          if (error instanceof ExchangeCacheSequenceError) {
            awaitingSnapshot = true;
            context.transitionHealth("recovering", "sequenceGap");
            if (!refreshPending) {
              refreshPending = true;
              debugRise("cache", "exchange.api.stream:refresh", {
                reason: "sequenceGap",
              });
              stream.refresh();
            }
            continue;
          }

          throw error;
        }
      }

      if (!context.isClosed() && !controller.signal.aborted) {
        debugRise("cache", "exchange.api.stream:end");
        return "streamEnded";
      }

      return null;
    } catch (error) {
      if (!context.isClosed() && !controller.signal.aborted) {
        debugRise("cache", "exchange.api.stream:error", {
          ...summarizeDebugError(error),
        });
        context.onBackgroundError(error);
        return "streamError";
      }
      return null;
    } finally {
      controller.abort();
      if (currentAbort === controller) {
        currentAbort = null;
      }
    }
  };

  return {
    start: async (): Promise<ApiSourceResult> => {
      if (!context.canUseApi()) {
        throw new Error("API exchange metadata source is not configured");
      }

      debugRise("cache", "exchange.api:start");
      await loadSnapshot("bootstrap");

      if (
        context.config.api?.liveUpdates === "none" ||
        context.config.api?.exchange === undefined
      ) {
        return null;
      }

      context.transitionHealth("live", "websocketSnapshot");

      while (!context.isClosed()) {
        const restartReason = await consumeStream();
        if (context.isClosed() || restartReason === null) {
          return null;
        }

        context.transitionHealth("recovering", restartReason);
        debugRise("cache", "exchange.api:restart", {
          reason: restartReason,
        });

        while (!context.isClosed()) {
          try {
            await loadSnapshot("bootstrap");
            break;
          } catch (error) {
            debugRise("cache", "exchange.api:bootstrap:error", {
              ...summarizeDebugError(error),
            });
            context.onBackgroundError(error);
            if (context.isClosed()) {
              return null;
            }
            if (context.canUseRpc()) {
              debugRise("cache", "exchange.api:fallback", {
                to: "rpc",
              });
              return "fallbackToRpc";
            }
            await delay(DEFAULT_API_RESYNC_BACKOFF_MS);
          }
        }

        if (!context.isClosed()) {
          context.transitionHealth("live", "websocketSnapshot");
        }
      }

      return null;
    },
    close: (): void => {
      debugRise("cache", "exchange.api:close");
      currentAbort?.abort();
      currentAbort = null;
    },
  };
};
