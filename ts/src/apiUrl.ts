export const DEFAULT_PHOENIX_API_URL = "https://perp-api.phoenix.trade";

export interface PhoenixApiUrlConfig {
  /** Base URL of the Phoenix API server (default: https://perp-api.phoenix.trade). */
  apiUrl?: string;
  /** @deprecated Use `apiUrl` instead. */
  baseUrl?: string;
}

const normalizePhoenixApiUrl = (value: string): string =>
  value.trim().replace(/\/+$/, "");

export const resolvePhoenixApiUrl = (
  config: PhoenixApiUrlConfig = {}
): string => {
  const resolvedApiUrl = config.apiUrl;
  const deprecatedBaseUrl = config.baseUrl;

  if (resolvedApiUrl && deprecatedBaseUrl) {
    const normalizedApiUrl = normalizePhoenixApiUrl(resolvedApiUrl);
    const normalizedBaseUrl = normalizePhoenixApiUrl(deprecatedBaseUrl);
    if (normalizedApiUrl !== normalizedBaseUrl) {
      throw new Error(
        "apiUrl and deprecated baseUrl must match when both are provided"
      );
    }
    return normalizedApiUrl;
  }

  const normalized = normalizePhoenixApiUrl(
    resolvedApiUrl ?? deprecatedBaseUrl ?? DEFAULT_PHOENIX_API_URL
  );
  if (!normalized) {
    throw new Error("apiUrl must be a non-empty string");
  }
  return normalized;
};
