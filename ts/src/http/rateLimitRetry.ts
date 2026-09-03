export interface RateLimitRetryConfig {
  /** Maximum number of retries after the initial request. */
  maxRetries?: number;
  /** Maximum total time spent waiting between retries. */
  maxTotalWaitMs?: number;
  /** Fallback delay used when Retry-After is missing or invalid. */
  fallbackDelayMs?: number;
}

export interface RateLimitCooldownConfig {
  /** Maximum Retry-After delay that can be applied as shared client cooldown. */
  maxDelayMs?: number;
  /** Fallback cooldown delay used when Retry-After is missing or invalid. */
  fallbackDelayMs?: number;
}

export interface ResolvedRateLimitRetryConfig {
  readonly maxRetries: number;
  readonly maxTotalWaitMs: number;
  readonly fallbackDelayMs: number;
}

export interface ResolvedRateLimitCooldownConfig {
  readonly maxDelayMs: number;
  readonly fallbackDelayMs: number;
}

export type RateLimitRetryDecision =
  | RateLimitRetryAttempt
  | RateLimitRetryExhausted;

export interface RateLimitRetryAttempt {
  readonly kind: "retry";
  readonly waitMs: number;
  readonly nextTotalWaitMs: number;
  readonly retryAfterSeconds?: number;
}

export interface RateLimitRetryExhausted {
  readonly kind: "exhausted";
  readonly waitMs: number;
  readonly nextTotalWaitMs: number;
  readonly retryAfterSeconds?: number;
}

export interface ParsedRetryAfter {
  readonly raw: string;
  readonly retryAfterMs: number;
  readonly retryAfterSeconds: number;
}

export const DEFAULT_RATE_LIMIT_RETRY_CONFIG: Required<RateLimitRetryConfig> = {
  maxRetries: 2,
  maxTotalWaitMs: 15_000,
  fallbackDelayMs: 1_000,
};

export const DEFAULT_RATE_LIMIT_COOLDOWN_MAX_DELAY_MS = 30_000;

const DEFAULT_FALLBACK_JITTER_RATIO = 0.15;
const DEFAULT_RATE_LIMIT_JITTER_RATIO = 0.15;

const finiteNonNegative = (value: number | undefined): value is number =>
  value !== undefined && Number.isFinite(value) && value >= 0;

export const resolveRateLimitRetryConfig = (
  config: RateLimitRetryConfig | false | undefined
): ResolvedRateLimitRetryConfig | null => {
  if (config === false) {
    return null;
  }

  return {
    maxRetries: finiteNonNegative(config?.maxRetries)
      ? Math.floor(config.maxRetries)
      : DEFAULT_RATE_LIMIT_RETRY_CONFIG.maxRetries,
    maxTotalWaitMs: finiteNonNegative(config?.maxTotalWaitMs)
      ? Math.ceil(config.maxTotalWaitMs)
      : DEFAULT_RATE_LIMIT_RETRY_CONFIG.maxTotalWaitMs,
    fallbackDelayMs: finiteNonNegative(config?.fallbackDelayMs)
      ? Math.ceil(config.fallbackDelayMs)
      : DEFAULT_RATE_LIMIT_RETRY_CONFIG.fallbackDelayMs,
  };
};

export const resolveRateLimitCooldownConfig = (
  config: RateLimitCooldownConfig | false | undefined,
  retryConfig: ResolvedRateLimitRetryConfig | null
): ResolvedRateLimitCooldownConfig | null => {
  if (config === false) {
    return null;
  }

  return {
    maxDelayMs: finiteNonNegative(config?.maxDelayMs)
      ? Math.ceil(config.maxDelayMs)
      : DEFAULT_RATE_LIMIT_COOLDOWN_MAX_DELAY_MS,
    fallbackDelayMs: finiteNonNegative(config?.fallbackDelayMs)
      ? Math.ceil(config.fallbackDelayMs)
      : (retryConfig?.fallbackDelayMs ??
        DEFAULT_RATE_LIMIT_RETRY_CONFIG.fallbackDelayMs),
  };
};

export const parseRetryAfter = (
  value: string | null,
  nowMs: number = Date.now()
): ParsedRetryAfter | undefined => {
  if (!value) return undefined;
  const raw = value.trim();
  if (!raw) return undefined;

  const parsedSeconds = Number(raw);
  if (Number.isFinite(parsedSeconds) && parsedSeconds >= 0) {
    const retryAfterMs = Math.ceil(parsedSeconds * 1000);
    return {
      raw,
      retryAfterMs,
      retryAfterSeconds: Math.ceil(retryAfterMs / 1000),
    };
  }

  const retryAtMs = Date.parse(raw);
  if (Number.isNaN(retryAtMs)) return undefined;
  const retryAfterMs = Math.max(0, retryAtMs - nowMs);
  return {
    raw,
    retryAfterMs,
    retryAfterSeconds: Math.ceil(retryAfterMs / 1000),
  };
};

export const parseRetryAfterSeconds = (
  value: string | null,
  nowMs?: number
): number | undefined => parseRetryAfter(value, nowMs)?.retryAfterSeconds;

export const isRateLimitRetryableMethod = (method: string): boolean => {
  const normalized = method.toUpperCase();
  return normalized === "GET" || normalized === "HEAD";
};

const rateLimitRetryAttempts = new WeakMap<Response, number>();

export const setRateLimitRetryAttempts = (
  response: Response,
  attempts: number
): void => {
  if (Number.isFinite(attempts) && attempts > 0) {
    rateLimitRetryAttempts.set(response, Math.floor(attempts));
  }
};

export const getRateLimitRetryAttempts = (
  response: Response
): number | undefined => rateLimitRetryAttempts.get(response);

export const fallbackRateLimitDelayMs = (
  config: ResolvedRateLimitRetryConfig
): number => {
  const jitter =
    1 -
    DEFAULT_FALLBACK_JITTER_RATIO +
    Math.random() * DEFAULT_FALLBACK_JITTER_RATIO * 2;
  return Math.max(0, Math.round(config.fallbackDelayMs * jitter));
};

export const rateLimitDelayWithPositiveJitterMs = (delayMs: number): number =>
  Math.max(
    0,
    Math.round(delayMs * (1 + Math.random() * DEFAULT_RATE_LIMIT_JITTER_RATIO))
  );

export const rateLimitCooldownDelayMs = (
  response: Response,
  config: ResolvedRateLimitCooldownConfig,
  nowMs: number = Date.now()
): number => {
  const retryAfter = parseRetryAfter(
    response.headers.get("retry-after"),
    nowMs
  );
  return Math.min(
    retryAfter?.retryAfterMs ?? config.fallbackDelayMs,
    config.maxDelayMs
  );
};

export const rateLimitRetryDecision = (
  response: Response,
  config: ResolvedRateLimitRetryConfig,
  totalWaitMs: number
): RateLimitRetryDecision => {
  const retryAfter = parseRetryAfter(response.headers.get("retry-after"));
  const waitMs = retryAfter
    ? rateLimitDelayWithPositiveJitterMs(retryAfter.retryAfterMs)
    : fallbackRateLimitDelayMs(config);
  const nextTotalWaitMs = totalWaitMs + waitMs;
  const retryAfterSeconds = retryAfter?.retryAfterSeconds;
  const baseDecision = {
    waitMs,
    nextTotalWaitMs,
    ...(retryAfterSeconds !== undefined ? { retryAfterSeconds } : {}),
  };

  if (nextTotalWaitMs > config.maxTotalWaitMs) {
    return {
      kind: "exhausted",
      ...baseDecision,
    };
  }

  return {
    kind: "retry",
    ...baseDecision,
  };
};
