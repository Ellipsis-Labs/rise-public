export type BackoffConfig = {
  baseMs: number;
  maxMs: number;
};

export type BackoffJitter = "none" | "equal" | "full";

export type BackoffComputeOpts = {
  jitter?: BackoffJitter;
  random?: () => number;
};

export const computeBackoffMs = (
  attempt: number,
  backoff?: BackoffConfig,
  opts?: BackoffComputeOpts
): number => {
  if (attempt <= 0) return 0;
  if (!backoff) return 0;

  const baseMs = Math.max(0, backoff.baseMs);
  const maxMs = Math.max(0, backoff.maxMs);
  if (baseMs === 0 || maxMs === 0) return 0;

  const raw = baseMs * 2 ** (attempt - 1);
  const capped = Math.min(raw, maxMs);

  const jitter = opts?.jitter ?? "equal";
  if (jitter === "none") return capped;

  const random = opts?.random ?? Math.random;
  const r = Math.max(0, Math.min(1, random()));

  if (jitter === "full") {
    return Math.floor(r * capped);
  }

  return Math.floor(capped / 2 + r * (capped / 2));
};
