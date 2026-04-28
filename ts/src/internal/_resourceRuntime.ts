const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => setTimeout(resolve, ms));

export interface ResourceLifecycle {
  retain(): () => void;
  pin(): void;
  close(): boolean;
  isClosed(): boolean;
  hasDemand(): boolean;
  retainCount(): number;
}

export const createLifecycle = (
  options: {
    onDemandStart?: () => void;
    onDemandStop?: () => void;
  } = {}
): ResourceLifecycle => {
  let closed = false;
  let retainCount = 0;
  let pinned = false;

  const hasDemand = (): boolean => !closed && (pinned || retainCount > 0);

  const updateDemand = (wasDemand: boolean): void => {
    const nextDemand = hasDemand();
    if (wasDemand === nextDemand) {
      return;
    }

    if (nextDemand) {
      options.onDemandStart?.();
      return;
    }

    options.onDemandStop?.();
  };

  return {
    retain: () => {
      if (closed) {
        throw new Error("Resource is closed");
      }

      const wasDemand = hasDemand();
      retainCount += 1;
      updateDemand(wasDemand);

      let released = false;
      return () => {
        if (released || closed) {
          return;
        }
        released = true;
        const previousDemand = hasDemand();
        retainCount = Math.max(0, retainCount - 1);
        updateDemand(previousDemand);
      };
    },
    pin: () => {
      if (closed) {
        throw new Error("Resource is closed");
      }

      const wasDemand = hasDemand();
      pinned = true;
      updateDemand(wasDemand);
    },
    close: () => {
      if (closed) {
        return false;
      }

      const hadDemand = hasDemand();
      closed = true;
      retainCount = 0;
      pinned = false;
      if (hadDemand) {
        options.onDemandStop?.();
      }
      return true;
    },
    isClosed: () => closed,
    hasDemand,
    retainCount: () => retainCount,
  };
};

export const normalizeBackgroundError = (
  error: unknown,
  fallbackMessage: string
): Error => (error instanceof Error ? error : new Error(fallbackMessage));

export const runRetryLoop = async <TMessage>(params: {
  shouldContinue: () => boolean;
  backoffMs: number;
  onController?: (controller: AbortController | null) => void;
  stream: (signal: AbortSignal) => AsyncIterable<TMessage>;
  onMessage: (message: TMessage) => void | Promise<void>;
  onDisconnect?: () => void | Promise<void>;
  onError?: (error: unknown) => void | Promise<void>;
  afterBackoff?: () => void | Promise<void>;
}): Promise<void> => {
  while (params.shouldContinue()) {
    const controller = new AbortController();
    params.onController?.(controller);

    try {
      for await (const message of params.stream(controller.signal)) {
        if (controller.signal.aborted || !params.shouldContinue()) {
          break;
        }

        await params.onMessage(message);
      }

      if (controller.signal.aborted || !params.shouldContinue()) {
        break;
      }

      await params.onDisconnect?.();
    } catch (error) {
      if (controller.signal.aborted || !params.shouldContinue()) {
        break;
      }

      await params.onError?.(error);
    } finally {
      params.onController?.(null);
      controller.abort();
    }

    if (!params.shouldContinue()) {
      break;
    }

    await delay(Math.max(1, params.backoffMs));
    if (!params.shouldContinue()) {
      break;
    }

    await params.afterBackoff?.();
  }
};
