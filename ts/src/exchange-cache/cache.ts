import type { ExchangeSnapshotView } from "@/api/exchange/types";
import type {
  ExchangeCacheEvent,
  ExchangeCacheHealth,
  ExchangeCacheHealthReason,
  PhoenixExchangeCache,
  PhoenixExchangeCacheConfig,
  PhoenixExchangeMetadataSourceState,
} from "./types";
import {
  createPhoenixExchangeCacheStore,
  ExchangeCacheSequenceError,
} from "./store";

const DEFAULT_RESYNC_BACKOFF_MS = 1_000;

const API_SOURCE_STATE: PhoenixExchangeMetadataSourceState = {
  active: "api",
  priority: "api",
  fallbackAvailable: false,
};

type StreamRestartReason = "sequenceGap" | "streamEnded" | "streamError";

const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

class PhoenixExchangeCacheImpl implements PhoenixExchangeCache {
  private readonly listeners = new Set<(event: ExchangeCacheEvent) => void>();
  private readonly readyPromise: Promise<Readonly<ExchangeSnapshotView>>;
  private readonly resyncBackoffMs: number;
  private currentAbort: AbortController | null = null;
  private closed = false;
  private readonly cacheStore = createPhoenixExchangeCacheStore();

  constructor(
    private readonly config: PhoenixExchangeCacheConfig,
    initialSnapshot: ExchangeSnapshotView
  ) {
    this.resyncBackoffMs = Math.max(
      1,
      config.resyncBackoffMs ?? DEFAULT_RESYNC_BACKOFF_MS
    );

    this.cacheStore.onEvent((event) => this.emit(event));
    this.cacheStore.setSource(API_SOURCE_STATE);
    this.cacheStore.applySnapshot(initialSnapshot, "http");
    this.readyPromise = Promise.resolve(this.cacheStore.snapshot());
    this.transitionHealth("bootstrapped", "bootstrap");
  }

  start(): void {
    void this.runLoop();
  }

  ready(): Promise<Readonly<ExchangeSnapshotView>> {
    return this.readyPromise;
  }

  health(): ExchangeCacheHealth {
    return this.cacheStore.store.getState().health;
  }

  snapshot(): Readonly<ExchangeSnapshotView> {
    return this.cacheStore.snapshot();
  }

  spotCollaterals() {
    return this.cacheStore.spotCollaterals();
  }

  market(symbol: string) {
    return this.cacheStore.market(symbol);
  }

  marketByAssetId(assetId: number) {
    return this.cacheStore.marketByAssetId(assetId);
  }

  marketByPubkey(pubkey: string) {
    return this.cacheStore.marketByPubkey(pubkey);
  }

  marketMetadata(symbol: string) {
    return this.cacheStore.marketMetadata(symbol);
  }

  marketMetadataByAssetId(assetId: number) {
    return this.cacheStore.marketMetadataByAssetId(assetId);
  }

  marketMetadataByPubkey(pubkey: string) {
    return this.cacheStore.marketMetadataByPubkey(pubkey);
  }

  instructionContext(symbol: string) {
    return this.cacheStore.instructionContext(symbol);
  }

  onEvent(listener: (event: ExchangeCacheEvent) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  subscribe(listener: (event: ExchangeCacheEvent) => void): () => void {
    return this.onEvent(listener);
  }

  source(): PhoenixExchangeMetadataSourceState {
    return { ...this.cacheStore.store.getState().source };
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.currentAbort?.abort();
    this.transitionHealth("closed", "closed");
    this.listeners.clear();
  }

  private emit(event: ExchangeCacheEvent): void {
    for (const listener of this.listeners) {
      try {
        listener(event);
      } catch (error) {
        this.config.onBackgroundError?.(error);
      }
    }
  }

  private transitionHealth(
    next: ExchangeCacheHealth,
    reason: ExchangeCacheHealthReason
  ): void {
    const previous = this.cacheStore.store.getState().health;
    if (previous === next) return;
    this.cacheStore.setHealth(next);
    this.emit({
      type: "healthChanged",
      previous,
      current: next,
      reason,
    });
  }

  private async runLoop(): Promise<void> {
    let needsHttpResync = false;
    let websocketRefreshAttempted = false;

    while (!this.closed) {
      if (needsHttpResync) {
        await this.recoverFromHttp();
        if (this.closed) return;
        websocketRefreshAttempted = false;
      }

      const restartReason = await this.consumeStream();
      if (this.closed || !restartReason) {
        return;
      }

      if (restartReason === "sequenceGap" && !websocketRefreshAttempted) {
        needsHttpResync = false;
        websocketRefreshAttempted = true;
        continue;
      }

      needsHttpResync = true;
      websocketRefreshAttempted = false;
      if (restartReason !== "sequenceGap") {
        await delay(this.resyncBackoffMs);
      }
    }
  }

  private async recoverFromHttp(): Promise<void> {
    while (!this.closed) {
      try {
        this.cacheStore.applySnapshot(
          await this.config.api.getSnapshot(),
          "http"
        );
        return;
      } catch (error) {
        this.config.onBackgroundError?.(error);
        await delay(this.resyncBackoffMs);
      }
    }
  }

  private async consumeStream(): Promise<StreamRestartReason | null> {
    const abort = new AbortController();
    this.currentAbort = abort;
    let awaitingSnapshot = true;
    let refreshPending = false;
    const stream = this.config.exchange(abort.signal);

    try {
      for await (const message of stream) {
        if (this.closed || abort.signal.aborted) {
          return null;
        }

        if (message.messageType === "snapshot") {
          this.cacheStore.applySnapshotMessage(message);
          awaitingSnapshot = false;
          refreshPending = false;
          this.transitionHealth("live", "websocketSnapshot");
          continue;
        }

        if (awaitingSnapshot) {
          this.transitionHealth("recovering", "sequenceGap");
          if (!refreshPending) {
            refreshPending = true;
            stream.refresh();
          }
          continue;
        }

        try {
          this.cacheStore.applyDelta(message);
        } catch (error) {
          if (error instanceof ExchangeCacheSequenceError) {
            this.transitionHealth("recovering", "sequenceGap");
            awaitingSnapshot = true;
            if (!refreshPending) {
              refreshPending = true;
              stream.refresh();
            }
            continue;
          }
          throw error;
        }
      }

      if (!this.closed && !abort.signal.aborted) {
        this.transitionHealth("recovering", "streamEnded");
        return "streamEnded";
      }

      return null;
    } catch (error) {
      if (!this.closed && !abort.signal.aborted) {
        this.config.onBackgroundError?.(error);
        this.transitionHealth("recovering", "streamError");
        return "streamError";
      }
      return null;
    } finally {
      abort.abort();
      if (this.currentAbort === abort) {
        this.currentAbort = null;
      }
    }
  }
}

export const createPhoenixExchangeCache = async (
  config: PhoenixExchangeCacheConfig
): Promise<PhoenixExchangeCache> => {
  const initialSnapshot = await config.api.getSnapshot();
  const cache = new PhoenixExchangeCacheImpl(config, initialSnapshot);
  cache.start();
  return cache;
};
