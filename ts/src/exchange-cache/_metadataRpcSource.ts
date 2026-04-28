import { debugRise, summarizeDebugError } from "@/internal/_debug";
import type { ExchangeCacheHealthReason } from "./types";
import {
  DEFAULT_RPC_POLL_INTERVAL_MS,
  DEFAULT_RPC_TTL_MS,
  delay,
  loadRpcSnapshot,
  type ExchangeMetadataSourceContext,
} from "./_metadataShared";

export interface RpcSource {
  start(reason: ExchangeCacheHealthReason): Promise<void>;
  maybeScheduleTtlRefresh(): void;
}

export const createRpcSource = (
  context: ExchangeMetadataSourceContext
): RpcSource => {
  let lastLoadedAtMs = 0;
  let refreshInFlight: Promise<void> | null = null;

  const loadAndInstall = async (
    reason: ExchangeCacheHealthReason
  ): Promise<void> => {
    const rpcUrl = context.config.rpc?.url;
    if (!rpcUrl) {
      throw new Error("RPC exchange metadata source is not configured");
    }

    debugRise("cache", "exchange.rpc.snapshot:start", {
      reason,
      rpcUrl,
    });
    const snapshot = await loadRpcSnapshot(rpcUrl, context.pdaClient);
    context.installSnapshot({
      snapshot,
      source: "rpc",
      active: "rpc",
    });
    lastLoadedAtMs = Date.now();
    context.transitionHealth("bootstrapped", reason);
    debugRise("cache", "exchange.rpc.snapshot:done", {
      reason,
      slot: snapshot.slot.toString(),
      slotIndex: snapshot.slotIndex,
      marketCount: snapshot.markets.length,
    });
  };

  const refresh = async (reason: ExchangeCacheHealthReason): Promise<void> => {
    if (refreshInFlight) {
      debugRise("cache", "exchange.rpc.refresh:join", { reason });
      await refreshInFlight;
      return;
    }

    const task = (async () => {
      try {
        debugRise("cache", "exchange.rpc.refresh:start", { reason });
        await loadAndInstall(reason);
      } catch (error) {
        debugRise("cache", "exchange.rpc.refresh:error", {
          reason,
          ...summarizeDebugError(error),
        });
        context.onBackgroundError(error);
      } finally {
        refreshInFlight = null;
      }
    })();

    refreshInFlight = task;
    await task;
  };

  return {
    start: async (reason: ExchangeCacheHealthReason): Promise<void> => {
      if (!context.canUseRpc()) {
        throw new Error("RPC exchange metadata source is not configured");
      }

      debugRise("cache", "exchange.rpc:start", { reason });
      await loadAndInstall(reason);

      const pollIntervalMs =
        context.config.rpc?.pollIntervalMs ?? DEFAULT_RPC_POLL_INTERVAL_MS;
      if (pollIntervalMs <= 0) {
        return;
      }

      while (!context.isClosed() && context.activeSource() === "rpc") {
        await delay(pollIntervalMs);
        if (context.isClosed() || context.activeSource() !== "rpc") {
          return;
        }
        await refresh("pollRefresh");
      }
    },
    maybeScheduleTtlRefresh: (): void => {
      if (context.activeSource() !== "rpc") {
        return;
      }

      const ttlMs = context.config.rpc?.ttlMs ?? DEFAULT_RPC_TTL_MS;
      if (Date.now() - lastLoadedAtMs < ttlMs) {
        return;
      }

      debugRise("cache", "exchange.rpc.refresh:schedule", {
        reason: "ttlRefresh",
        ttlMs,
      });
      void refresh("ttlRefresh");
    },
  };
};
