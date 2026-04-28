import { createStore } from "zustand/vanilla";
import { createResourceRegistry } from "@/internal/_keyedResourceManager";
import {
  createLifecycle,
  normalizeBackgroundError,
  runRetryLoop,
} from "@/internal/_resourceRuntime";
import type { L2BookUpdate } from "@/ws/adapters/l2-book/wire";
import type {
  PhoenixOrderbookBootstrapSnapshot,
  PhoenixOrderbookHealth,
  PhoenixOrderbookLevel,
  PhoenixOrderbookManager,
  PhoenixOrderbookManagerConfig,
  PhoenixOrderbookRequest,
  PhoenixOrderbookRequestOptions,
  PhoenixOrderbookResource,
  PhoenixOrderbookResourceInput,
  PhoenixOrderbookSnapshot,
  PhoenixOrderbookStatus,
  PhoenixOrderbookStoreState,
} from "./types";

const DEFAULT_RESYNC_BACKOFF_MS = 1_000;

const EMPTY_LEVELS: readonly PhoenixOrderbookLevel[] = [];

const normalizeSymbol = (symbol: string) => symbol.trim().toUpperCase();

type NormalizedOrderbookRequest = {
  symbol: string;
  bypassExecutionBand: boolean;
  requestOptions?: PhoenixOrderbookRequestOptions;
};

const toRequestOptions = (
  bypassExecutionBand: boolean | undefined
): PhoenixOrderbookRequestOptions | undefined =>
  bypassExecutionBand === undefined ? undefined : { bypassExecutionBand };

const normalizeOrderbookRequest = (
  input: PhoenixOrderbookResourceInput,
  options?: PhoenixOrderbookRequestOptions
): NormalizedOrderbookRequest => {
  const request =
    typeof input === "string"
      ? { symbol: input, ...options }
      : ({ ...input, ...options } satisfies PhoenixOrderbookRequest);
  const symbol = normalizeSymbol(request.symbol);
  return {
    symbol,
    bypassExecutionBand: request.bypassExecutionBand === true,
    requestOptions: toRequestOptions(request.bypassExecutionBand),
  };
};

const getOrderbookStoreKey = (request: {
  symbol: string;
  bypassExecutionBand: boolean;
}) =>
  request.bypassExecutionBand
    ? `orderbook:${request.symbol}:bypassExecutionBand`
    : `orderbook:${request.symbol}`;

const toTimestampMs = (value: number | bigint | null | undefined): number =>
  value === null || value === undefined
    ? Date.now()
    : Number(value) > 10_000_000_000
      ? Math.floor(Number(value))
      : Math.floor(Number(value) * 1000);

const sortLevels = (
  levels: readonly PhoenixOrderbookLevel[],
  side: "bids" | "asks"
): readonly PhoenixOrderbookLevel[] =>
  [...levels].sort((left, right) =>
    side === "bids" ? right.price - left.price : left.price - right.price
  );

const toLevels = (
  levels: PhoenixOrderbookBootstrapSnapshot["bids"],
  side: "bids" | "asks"
): readonly PhoenixOrderbookLevel[] =>
  sortLevels(
    levels.map((level) => ({
      price: Number(level[0] ?? 0),
      size: Number(level[1] ?? 0),
    })),
    side
  );

const toLevelsFromUpdate = (
  levels: L2BookUpdate["bids"],
  side: "bids" | "asks"
): readonly PhoenixOrderbookLevel[] =>
  sortLevels(
    levels.map(([price, size]) => ({
      price,
      size,
    })),
    side
  );

const levelsEqual = (
  left: readonly PhoenixOrderbookLevel[],
  right: readonly PhoenixOrderbookLevel[]
): boolean =>
  left.length === right.length &&
  left.every(
    (level, index) =>
      level.price === right[index]?.price && level.size === right[index]?.size
  );

const buildBook = (params: {
  symbol: string;
  bypassExecutionBand: boolean;
  slot: bigint | null;
  timestampMs: number;
  bids: readonly PhoenixOrderbookLevel[];
  asks: readonly PhoenixOrderbookLevel[];
  mid?: number | null;
}): PhoenixOrderbookSnapshot => {
  const bestBid = params.bids[0] ?? null;
  const bestAsk = params.asks[0] ?? null;
  const derivedMid =
    bestBid && bestAsk ? (bestBid.price + bestAsk.price) / 2 : 0;
  const mid = params.mid ?? derivedMid;
  const spread =
    bestBid && bestAsk ? Math.max(0, bestAsk.price - bestBid.price) : 0;

  return {
    symbol: params.symbol,
    bypassExecutionBand: params.bypassExecutionBand,
    timestamp: Math.floor(params.timestampMs / 1000),
    timestampMs: params.timestampMs,
    slot: params.slot,
    bids: params.bids,
    asks: params.asks,
    bestBid,
    bestAsk,
    mid,
    spread,
  };
};

const preserveBookReference = (
  previous: PhoenixOrderbookSnapshot | null,
  next: PhoenixOrderbookSnapshot
): PhoenixOrderbookSnapshot => {
  if (!previous) {
    return next;
  }

  if (
    previous.symbol === next.symbol &&
    previous.slot === next.slot &&
    previous.timestampMs === next.timestampMs &&
    previous.mid === next.mid &&
    previous.spread === next.spread &&
    levelsEqual(previous.bids, next.bids) &&
    levelsEqual(previous.asks, next.asks)
  ) {
    return previous;
  }

  return {
    ...next,
    bids: levelsEqual(previous.bids, next.bids) ? previous.bids : next.bids,
    asks: levelsEqual(previous.asks, next.asks) ? previous.asks : next.asks,
    bestBid:
      previous.bestBid &&
      next.bestBid &&
      previous.bestBid.price === next.bestBid.price &&
      previous.bestBid.size === next.bestBid.size
        ? previous.bestBid
        : next.bestBid,
    bestAsk:
      previous.bestAsk &&
      next.bestAsk &&
      previous.bestAsk.price === next.bestAsk.price &&
      previous.bestAsk.size === next.bestAsk.size
        ? previous.bestAsk
        : next.bestAsk,
  };
};

const createInitialStoreState = (
  key: string,
  symbol: string,
  bypassExecutionBand: boolean
): PhoenixOrderbookStoreState => ({
  key,
  symbol,
  bypassExecutionBand,
  status: {
    health: "cold",
    isConnected: false,
    isLoading: false,
    error: null,
  },
  book: null,
  bids: EMPTY_LEVELS,
  asks: EMPTY_LEVELS,
  bestBid: null,
  bestAsk: null,
  mid: null,
  spread: null,
  slot: null,
  timestamp: null,
  timestampMs: null,
  lastUpdatedMs: null,
});

class PhoenixOrderbookResourceImpl implements PhoenixOrderbookResource {
  readonly key: string;
  readonly symbol: string;
  readonly bypassExecutionBand: boolean;
  readonly store;

  private bootstrapPromise: Promise<PhoenixOrderbookSnapshot | null> | null =
    null;
  private streamPromise: Promise<void> | null = null;
  private streamAbort: AbortController | null = null;
  private readonly requestOptions?: PhoenixOrderbookRequestOptions;
  private readonly lifecycle = createLifecycle({
    onDemandStop: () => {
      this.stopStream();
      this.updateStatus((status) => ({
        ...status,
        health: this.snapshot() ? "bootstrapped" : status.health,
        isConnected: false,
      }));
    },
  });

  constructor(
    request: NormalizedOrderbookRequest,
    private readonly config: PhoenixOrderbookManagerConfig,
    private readonly disposeFromManager: () => void
  ) {
    this.symbol = request.symbol;
    this.bypassExecutionBand = request.bypassExecutionBand;
    this.requestOptions = request.requestOptions;
    this.key = getOrderbookStoreKey(request);
    this.store = createStore<PhoenixOrderbookStoreState>(() =>
      createInitialStoreState(this.key, this.symbol, this.bypassExecutionBand)
    );
  }

  retain(): () => void {
    if (this.lifecycle.isClosed()) {
      throw new Error(`Orderbook resource ${this.key} is closed`);
    }

    const release = this.lifecycle.retain();
    void this.ensureBootstrapped(false);
    this.ensureStreamLoop();
    return release;
  }

  async ready(): Promise<PhoenixOrderbookSnapshot | null> {
    return this.ensureBootstrapped(false);
  }

  status(): PhoenixOrderbookStatus {
    return this.store.getState().status;
  }

  snapshot(): PhoenixOrderbookSnapshot | null {
    return this.store.getState().book;
  }

  async refresh(): Promise<PhoenixOrderbookSnapshot | null> {
    return this.ensureBootstrapped(true);
  }

  reconnect(): void {
    if (this.lifecycle.isClosed()) return;
    this.stopStream();
    this.updateStatus((status) => ({
      ...status,
      health: this.snapshot() ? "recovering" : "cold",
      isLoading: this.snapshot() === null,
      error: null,
    }));
    void this.ensureBootstrapped(true).finally(() => {
      this.ensureStreamLoop();
    });
  }

  subscribe(
    listener: (
      state: PhoenixOrderbookStoreState,
      previousState: PhoenixOrderbookStoreState
    ) => void
  ): () => void {
    return this.store.subscribe(listener);
  }

  close(): void {
    if (!this.lifecycle.close()) return;
    this.stopStream();
    this.store.setState(
      {
        ...this.store.getState(),
        status: {
          ...this.store.getState().status,
          health: "closed",
          isConnected: false,
          isLoading: false,
        },
      },
      true
    );
    this.disposeFromManager();
  }

  private updateStatus(
    updater: (status: PhoenixOrderbookStatus) => PhoenixOrderbookStatus
  ): void {
    const current = this.store.getState();
    this.store.setState(
      {
        ...current,
        status: updater(current.status),
      },
      true
    );
  }

  private stopStream(): void {
    this.streamAbort?.abort();
    this.streamAbort = null;
  }

  private ensureStreamLoop(): void {
    if (
      this.lifecycle.isClosed() ||
      !this.config.l2Book ||
      !this.lifecycle.hasDemand() ||
      this.streamPromise
    ) {
      return;
    }

    this.streamPromise = this.runStreamLoop().finally(() => {
      this.streamPromise = null;
    });
  }

  private async runStreamLoop(): Promise<void> {
    await runRetryLoop({
      shouldContinue: () =>
        !this.lifecycle.isClosed() &&
        this.lifecycle.hasDemand() &&
        this.config.l2Book !== undefined,
      backoffMs: this.config.resyncBackoffMs ?? DEFAULT_RESYNC_BACKOFF_MS,
      onController: (controller) => {
        this.streamAbort = controller;
      },
      stream: (signal) =>
        this.config.l2Book!(this.symbol, this.requestOptions, signal),
      onMessage: (update) => {
        this.applyBookFromUpdate(update, "live");
      },
      onDisconnect: () => {
        this.updateStatus((status) => ({
          ...status,
          health: this.snapshot() ? "recovering" : "cold",
          isConnected: false,
        }));
      },
      onError: (error) => {
        const normalizedError = normalizeBackgroundError(
          error,
          `Orderbook websocket stream failed for ${this.symbol}`
        );
        this.config.onBackgroundError?.(normalizedError);
        this.updateStatus((status) => ({
          ...status,
          health: this.snapshot() ? "recovering" : "cold",
          isConnected: this.snapshot() !== null,
          error: normalizedError,
        }));
      },
      afterBackoff: async () => {
        try {
          await this.ensureBootstrapped(true);
        } catch (error) {
          this.config.onBackgroundError?.(error);
        }
      },
    });
  }

  private applyBook(
    nextBook: PhoenixOrderbookSnapshot,
    health: PhoenixOrderbookHealth
  ): void {
    const current = this.store.getState();
    const book = preserveBookReference(current.book, nextBook);
    this.store.setState(
      {
        ...current,
        status: {
          health,
          isConnected: health === "live",
          isLoading: false,
          error: null,
        },
        book,
        bids: book.bids,
        asks: book.asks,
        bestBid: book.bestBid,
        bestAsk: book.bestAsk,
        mid: book.mid,
        spread: book.spread,
        slot: book.slot,
        timestamp: book.timestamp,
        timestampMs: book.timestampMs,
        lastUpdatedMs: Date.now(),
      },
      true
    );
  }

  private applyBookFromBootstrap(
    snapshot: PhoenixOrderbookBootstrapSnapshot
  ): PhoenixOrderbookSnapshot {
    const book = buildBook({
      symbol: normalizeSymbol(snapshot.symbol || this.symbol),
      bypassExecutionBand: this.bypassExecutionBand,
      slot:
        snapshot.slot === null || snapshot.slot === undefined
          ? null
          : BigInt(snapshot.slot),
      timestampMs: toTimestampMs(snapshot.timestamp),
      bids: toLevels(snapshot.bids, "bids"),
      asks: toLevels(snapshot.asks, "asks"),
      mid: snapshot.mid ?? null,
    });
    this.applyBook(book, "bootstrapped");
    return book;
  }

  private applyBookFromUpdate(
    update: L2BookUpdate,
    health: PhoenixOrderbookHealth
  ): PhoenixOrderbookSnapshot {
    const book = buildBook({
      symbol: normalizeSymbol(update.market || this.symbol),
      bypassExecutionBand: this.bypassExecutionBand,
      slot: update.slot ?? null,
      timestampMs: toTimestampMs(update.ts),
      bids: toLevelsFromUpdate(update.bids, "bids"),
      asks: toLevelsFromUpdate(update.asks, "asks"),
    });
    this.applyBook(book, health);
    return book;
  }

  private async ensureBootstrapped(
    force: boolean
  ): Promise<PhoenixOrderbookSnapshot | null> {
    if (this.lifecycle.isClosed()) {
      return this.snapshot();
    }
    if (this.bootstrapPromise) {
      return this.bootstrapPromise;
    }
    if (!force && this.snapshot()) {
      return this.snapshot();
    }
    if (!this.config.api) {
      this.ensureStreamLoop();
      if (this.snapshot()) {
        return this.snapshot();
      }
      throw new Error(
        `Orderbook snapshot API is not configured for ${this.symbol}`
      );
    }

    const current = this.store.getState();
    this.store.setState(
      {
        ...current,
        status: {
          ...current.status,
          isLoading: current.book === null,
          error: null,
          health:
            current.book === null
              ? "cold"
              : current.status.health === "live"
                ? "live"
                : "bootstrapped",
        },
      },
      true
    );

    this.bootstrapPromise = this.config.api
      .getOrderbook(this.symbol, this.requestOptions)
      .then((response) => {
        const snapshot = this.applyBookFromBootstrap(response);
        this.ensureStreamLoop();
        return snapshot;
      })
      .catch((error) => {
        const normalizedError =
          error instanceof Error
            ? error
            : new Error(`Failed to bootstrap orderbook for ${this.symbol}`);
        this.store.setState(
          {
            ...this.store.getState(),
            status: {
              ...this.store.getState().status,
              isLoading: false,
              isConnected: this.snapshot() !== null,
              error: normalizedError,
              health: this.snapshot() ? "recovering" : "cold",
            },
          },
          true
        );
        throw normalizedError;
      })
      .finally(() => {
        this.bootstrapPromise = null;
      });

    return this.bootstrapPromise;
  }
}

class PhoenixOrderbookManagerImpl implements PhoenixOrderbookManager {
  private readonly resources =
    createResourceRegistry<PhoenixOrderbookResourceImpl>();

  constructor(private readonly config: PhoenixOrderbookManagerConfig) {}

  resource(
    input: PhoenixOrderbookResourceInput,
    options?: PhoenixOrderbookRequestOptions
  ): PhoenixOrderbookResource {
    const request = normalizeOrderbookRequest(input, options);
    const key = getOrderbookStoreKey(request);
    return this.resources.getOrCreate(
      key,
      () =>
        new PhoenixOrderbookResourceImpl(request, this.config, () => {
          this.resources.delete(key);
        })
    );
  }

  async prefetch(
    input: PhoenixOrderbookResourceInput,
    options?: PhoenixOrderbookRequestOptions
  ): Promise<PhoenixOrderbookResource> {
    const resource = this.resource(input, options);
    await resource.ready();
    return resource;
  }

  peek(
    input: PhoenixOrderbookResourceInput,
    options?: PhoenixOrderbookRequestOptions
  ): PhoenixOrderbookResource | undefined {
    return this.resources.peek(
      getOrderbookStoreKey(normalizeOrderbookRequest(input, options))
    );
  }

  clear(
    input: PhoenixOrderbookResourceInput,
    options?: PhoenixOrderbookRequestOptions
  ): void {
    this.resources
      .peek(getOrderbookStoreKey(normalizeOrderbookRequest(input, options)))
      ?.close();
  }

  close(): void {
    for (const resource of Array.from(this.resources.values())) {
      resource.close();
    }
    this.resources.clear();
  }
}

export const createPhoenixOrderbookManager = (
  config: PhoenixOrderbookManagerConfig
): PhoenixOrderbookManager => new PhoenixOrderbookManagerImpl(config);
