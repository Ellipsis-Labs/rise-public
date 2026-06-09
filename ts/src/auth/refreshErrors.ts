const TERMINAL_REFRESH_ERROR_CODES = new Set([
  "invalid_refresh_token",
  "refresh_expired",
  "session_missing",
  "no_session",
]);

const SESSION_CLEARING_REFRESH_ERROR_CODES = new Set([
  "invalid_refresh_token",
  "refresh_expired",
  "session_missing",
]);

export const getRefreshErrorCode = (error: unknown): string | undefined => {
  if (!error || typeof error !== "object") return undefined;
  const code = (error as { code?: unknown }).code;
  return typeof code === "string" ? code : undefined;
};

export const getRefreshRetryAfterMs = (error: unknown): number | null => {
  if (!error || typeof error !== "object") return null;
  const candidate = error as {
    retryAfterMs?: unknown;
    retryAfterSeconds?: unknown;
  };
  if (
    typeof candidate.retryAfterMs === "number" &&
    Number.isFinite(candidate.retryAfterMs)
  ) {
    return Math.max(0, candidate.retryAfterMs);
  }
  if (
    typeof candidate.retryAfterSeconds === "number" &&
    Number.isFinite(candidate.retryAfterSeconds)
  ) {
    return Math.max(0, candidate.retryAfterSeconds * 1000);
  }
  return null;
};

export const isTerminalRefreshError = (error: unknown): boolean => {
  const code = getRefreshErrorCode(error);
  return code !== undefined && TERMINAL_REFRESH_ERROR_CODES.has(code);
};

export const shouldClearSessionOnRefreshFailure = (
  errorOrCode: unknown
): boolean => {
  const code =
    typeof errorOrCode === "string"
      ? errorOrCode
      : getRefreshErrorCode(errorOrCode);
  return code !== undefined && SESSION_CLEARING_REFRESH_ERROR_CODES.has(code);
};
