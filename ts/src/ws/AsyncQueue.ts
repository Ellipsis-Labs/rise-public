type OverflowStrategy = "drop-oldest" | "drop-newest";

export interface AsyncQueue<T> extends AsyncIterable<T> {
  push(v: T): boolean;
  close(): void;
  fail(e: unknown): void;
  size(): number;
  isClosed(): boolean;
  finished: Promise<void>;
}

export const createAsyncQueue = <T>(
  capacity = 512,
  overflowStrategy: OverflowStrategy = "drop-oldest",
  signal?: AbortSignal
): AsyncQueue<T> => {
  interface Waiter {
    resolve: (r: IteratorResult<T, void>) => void;
    reject: (e: unknown) => void;
  }

  const queue: T[] = [];
  let waiter: Waiter | null = null;

  let closed = false;
  let error: unknown = null;

  let finishedResolve: (() => void) | null = null;
  const finished = new Promise<void>((resolve) => {
    finishedResolve = resolve;
  });

  const push = (v: T): boolean => {
    if (closed || error != null) return false;

    if (queue.length >= capacity) {
      if (overflowStrategy === "drop-oldest") {
        queue.shift();
      } else {
        return false;
      }
    }

    if (waiter) {
      const { resolve } = waiter;
      waiter = null;
      resolve({ value: v, done: false });
      return true;
    }

    queue.push(v);
    return true;
  };

  const close = () => {
    if (closed) return;
    closed = true;
    cleanup();
    finishedResolve?.();
  };

  const fail = (e: unknown) => {
    if (error != null) return;
    error = e ?? new Error("AsyncQueue error");
    cleanup();
    finishedResolve?.();
  };

  const cleanup = () => {
    if (waiter && queue.length) {
      const { resolve } = waiter;
      waiter = null;
      resolve({ value: queue.shift()!, done: false });
      return;
    }

    if ((closed || error !== null) && queue.length === 0 && waiter) {
      const { reject, resolve } = waiter;
      waiter = null;

      if (error !== null) reject(safeError(error));
      else resolve({ value: undefined, done: true });
    }
  };

  const next = (): Promise<IteratorResult<T, void>> =>
    new Promise<IteratorResult<T, void>>((resolve, reject) => {
      if (error !== null) return reject(safeError(error));
      if (queue.length) return resolve({ value: queue.shift()!, done: false });
      if (closed) return resolve({ value: undefined, done: true });

      if (waiter !== null) {
        return reject(
          new Error(
            "[AsyncQueue] Multiple concurrent consumers detected. Only one consumer can call next() at a time."
          )
        );
      }

      waiter = { resolve, reject };
    });

  const iterator: AsyncIterableIterator<T> = {
    next,
    return() {
      close();
      return Promise.resolve({ value: undefined, done: true });
    },
    [Symbol.asyncIterator]() {
      return this;
    },
  };

  signal?.addEventListener("abort", () => close(), { once: true });

  return {
    push,
    close,
    fail,
    size: () => queue.length,
    isClosed: () => closed || error != null,
    finished,
    [Symbol.asyncIterator]: () => iterator,
  };
};

const safeError = (e: unknown) => {
  if (e instanceof Error) return e;
  return new Error(String(e));
};
