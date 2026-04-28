import { createPhoenixPdaClient } from "@/pdaClient";
import type {
  ExchangeMarketSnapshot,
  ExchangeSnapshotView,
} from "@/api/exchange/types";
import { debugRise, summarizeDebugError } from "@/internal/_debug";
import { createPhoenixExchangeCacheStore } from "./store";
import { createApiSource } from "./_metadataApiSource";
import { createRpcSource } from "./_metadataRpcSource";
import {
  emitExchangeDiffEvents,
  type ExchangeMetadataSnapshotInstallParams,
  type ExchangeMetadataSourceContext,
} from "./_metadataShared";
import type {
  ExchangeCacheEvent,
  ExchangeCacheHealth,
  ExchangeCacheHealthReason,
  ExchangeMetadataPriority,
  ExchangeMetadataSource,
  PhoenixExchangeMetadata,
  PhoenixExchangeMetadataConfig,
  PhoenixExchangeMetadataSourceState,
} from "./types";

class PhoenixExchangeMetadataImpl implements PhoenixExchangeMetadata {
  private readonly listeners = new Set<(event: ExchangeCacheEvent) => void>();
  private readonly pdaClient: ReturnType<typeof createPhoenixPdaClient>;
  private readonly priority: ExchangeMetadataPriority;
  private readonly sourceState: PhoenixExchangeMetadataSourceState;
  private readonly cacheStore = createPhoenixExchangeCacheStore();
  private readonly readyPromise: Promise<Readonly<ExchangeSnapshotView>>;
  private readonly apiSource: ReturnType<typeof createApiSource>;
  private readonly rpcSource: ReturnType<typeof createRpcSource>;

  private readySettled = false;
  private currentHealth: ExchangeCacheHealth = "cold";
  private closed = false;
  private started = false;
  private readyResolve!: (snapshot: Readonly<ExchangeSnapshotView>) => void;
  private readyReject!: (error: unknown) => void;

  constructor(private readonly config: PhoenixExchangeMetadataConfig) {
    this.pdaClient = this.config.pda ?? createPhoenixPdaClient();
    this.priority = this.config.priority ?? "api";
    this.sourceState = {
      active: "none",
      priority: this.priority,
      fallbackAvailable:
        this.config.rpc?.enabled !== false && !!this.config.rpc?.url,
    };
    this.readyPromise = new Promise<Readonly<ExchangeSnapshotView>>(
      (resolve, reject) => {
        this.readyResolve = resolve;
        this.readyReject = reject;
      }
    );
    const sourceContext = this.createSourceContext();
    this.apiSource = createApiSource(sourceContext);
    this.rpcSource = createRpcSource(sourceContext);
    this.cacheStore.setHealth(this.currentHealth);
    this.cacheStore.setSource(this.sourceState);
    this.cacheStore.onEvent((event) => this.emit(event));
  }

  get store() {
    return this.cacheStore.store;
  }

  ready(): Promise<Readonly<ExchangeSnapshotView>> {
    this.ensureStarted();
    return this.readyPromise;
  }

  health(): ExchangeCacheHealth {
    this.ensureStarted();
    return this.currentHealth;
  }

  source(): PhoenixExchangeMetadataSourceState {
    this.ensureStarted();
    return { ...this.sourceState };
  }

  snapshot(): Readonly<ExchangeSnapshotView> {
    this.ensureStarted();
    const snapshot = this.cacheStore.store.getState().snapshot;
    if (!snapshot) {
      throw new Error(
        "Exchange metadata is not ready yet. Await client.exchange.ready() first."
      );
    }
    this.rpcSource.maybeScheduleTtlRefresh();
    return snapshot;
  }

  market(symbol: string): ExchangeMarketSnapshot | undefined {
    this.ensureStarted();
    this.rpcSource.maybeScheduleTtlRefresh();
    return this.cacheStore.market(symbol);
  }

  marketByAssetId(assetId: number): ExchangeMarketSnapshot | undefined {
    this.ensureStarted();
    this.rpcSource.maybeScheduleTtlRefresh();
    return this.cacheStore.marketByAssetId(assetId);
  }

  marketByPubkey(pubkey: string): ExchangeMarketSnapshot | undefined {
    this.ensureStarted();
    this.rpcSource.maybeScheduleTtlRefresh();
    return this.cacheStore.marketByPubkey(pubkey);
  }

  instructionContext(symbol: string) {
    this.ensureStarted();
    this.rpcSource.maybeScheduleTtlRefresh();
    return this.cacheStore.instructionContext(symbol);
  }

  subscribe(listener: (event: ExchangeCacheEvent) => void): () => void {
    this.ensureStarted();
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  onEvent(listener: (event: ExchangeCacheEvent) => void): () => void {
    return this.subscribe(listener);
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    debugRise("cache", "exchange.close", {
      health: this.currentHealth,
      source: this.sourceState.active,
    });
    this.apiSource.close();
    this.transitionHealth("closed", "closed");
    this.listeners.clear();
  }

  private emit(event: ExchangeCacheEvent): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }

  private transitionHealth(
    current: ExchangeCacheHealth,
    reason: ExchangeCacheHealthReason
  ): void {
    if (this.currentHealth === current) return;
    const previous = this.currentHealth;
    this.currentHealth = current;
    this.cacheStore.setHealth(current);
    this.emit({
      type: "healthChanged",
      previous,
      current,
      reason,
    });
    debugRise("cache", "exchange.health", {
      previous,
      current,
      reason,
    });
  }

  private canUseApi(): boolean {
    return (
      this.config.api?.enabled !== false && this.config.api?.api !== undefined
    );
  }

  private canUseRpc(): boolean {
    return this.config.rpc?.enabled !== false && !!this.config.rpc?.url;
  }

  private setSource(active: ExchangeMetadataSource): void {
    const previous = this.sourceState.active;
    if (previous === active) {
      return;
    }
    this.sourceState.active = active;
    this.cacheStore.setSource(this.sourceState);
    this.emit({
      type: "sourceChanged",
      previous,
      current: active,
    });
    debugRise("cache", "exchange.source", {
      previous,
      current: active,
    });
  }

  private markSourceSynced(): void {
    this.sourceState.lastSyncAt = Date.now();
    this.cacheStore.setSource(this.sourceState);
  }

  private settleReady(snapshot: Readonly<ExchangeSnapshotView>): void {
    if (this.readySettled) {
      return;
    }
    this.readySettled = true;
    debugRise("cache", "exchange.ready:resolved", {
      health: this.currentHealth,
      source: this.sourceState.active,
      slot: snapshot.slot.toString(),
      slotIndex: snapshot.slotIndex,
      marketCount: snapshot.markets.length,
    });
    this.readyResolve(snapshot);
  }

  private failReady(error: unknown): void {
    if (this.readySettled) {
      return;
    }
    this.readySettled = true;
    debugRise("cache", "exchange.ready:error", {
      health: this.currentHealth,
      source: this.sourceState.active,
      ...summarizeDebugError(error),
    });
    this.readyReject(error);
  }

  private installSnapshot({
    snapshot,
    source,
    active,
  }: ExchangeMetadataSnapshotInstallParams): void {
    emitExchangeDiffEvents({
      current: this.cacheStore.store.getState().snapshot,
      next: snapshot,
      emit: (event) => this.emit(event),
    });
    this.cacheStore.applySnapshot(snapshot, source);
    this.setSource(active);
    this.markSourceSynced();
    this.settleReady(this.cacheStore.snapshot());
    debugRise("cache", "exchange.snapshot", {
      source,
      active,
      slot: snapshot.slot.toString(),
      slotIndex: snapshot.slotIndex,
      marketCount: snapshot.markets.length,
    });
  }

  private async start(): Promise<void> {
    if (this.priority === "rpc") {
      await this.startRpcThenApi();
      return;
    }

    await this.startApiThenRpc();
  }

  private ensureStarted(): void {
    if (this.started || this.closed) return;
    this.started = true;
    debugRise("cache", "exchange.start", {
      priority: this.priority,
      canUseApi: this.canUseApi(),
      canUseRpc: this.canUseRpc(),
    });
    void this.start().catch((error) => {
      debugRise("cache", "exchange.start:error", {
        ...summarizeDebugError(error),
      });
      this.failReady(error);
      this.config.onBackgroundError?.(error);
    });
  }

  private async startApiThenRpc(): Promise<void> {
    try {
      debugRise("cache", "exchange.strategy", {
        strategy: "api-then-rpc",
      });
      const result = await this.apiSource.start();
      if (result === "fallbackToRpc") {
        debugRise("cache", "exchange.fallback", {
          from: "api",
          to: "rpc",
        });
        await this.rpcSource.start("rpcBootstrap");
      }
    } catch (error) {
      this.transitionHealth("recovering", "apiUnavailable");
      if (this.canUseRpc()) {
        debugRise("cache", "exchange.fallback", {
          from: "api",
          to: "rpc",
          reason: "apiUnavailable",
        });
        await this.rpcSource.start("rpcBootstrap");
        return;
      }
      this.config.onBackgroundError?.(error);
      throw error;
    }
  }

  private async startRpcThenApi(): Promise<void> {
    try {
      debugRise("cache", "exchange.strategy", {
        strategy: "rpc-then-api",
      });
      await this.rpcSource.start("rpcBootstrap");
    } catch (error) {
      this.transitionHealth("recovering", "rpcUnavailable");
      if (this.canUseApi()) {
        debugRise("cache", "exchange.fallback", {
          from: "rpc",
          to: "api",
          reason: "rpcUnavailable",
        });
        const result = await this.apiSource.start();
        if (result === "fallbackToRpc" && this.canUseRpc()) {
          debugRise("cache", "exchange.fallback", {
            from: "api",
            to: "rpc",
            reason: "apiStreamFallback",
          });
          await this.rpcSource.start("rpcBootstrap");
        }
        return;
      }
      this.config.onBackgroundError?.(error);
      throw error;
    }
  }

  private createSourceContext(): ExchangeMetadataSourceContext {
    return {
      cacheStore: this.cacheStore,
      config: this.config,
      pdaClient: this.pdaClient,
      isClosed: () => this.closed,
      canUseApi: () => this.canUseApi(),
      canUseRpc: () => this.canUseRpc(),
      activeSource: () => this.sourceState.active,
      installSnapshot: (params) => this.installSnapshot(params),
      markSourceSynced: () => this.markSourceSynced(),
      transitionHealth: (current, reason) =>
        this.transitionHealth(current, reason),
      onBackgroundError: (error) => this.config.onBackgroundError?.(error),
    };
  }
}

export const createPhoenixExchangeMetadata = (
  config: PhoenixExchangeMetadataConfig
): PhoenixExchangeMetadata => new PhoenixExchangeMetadataImpl(config);
