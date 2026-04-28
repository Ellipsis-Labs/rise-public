type RuntimeEnv = Record<string, string | undefined>;

export interface PhoenixRpcUrlConfig {
  /** RPC URL used for direct Solana account fetches and exchange metadata fallback. */
  rpcUrl?: string;
}

const RPC_URL_ENV_KEYS = [
  "NEXT_PUBLIC_SOLANA_RPC_URL",
  "SOLANA_RPC_URL",
] as const;

const normalizePhoenixRpcUrl = (value: string): string =>
  value.trim().replace(/\/+$/, "");

const getRuntimeEnv = (): RuntimeEnv | undefined => {
  const runtimeProcess = (
    globalThis as typeof globalThis & {
      process?: { env?: RuntimeEnv };
    }
  ).process;
  if (runtimeProcess?.env && typeof runtimeProcess.env === "object") {
    return runtimeProcess.env;
  }

  if (
    typeof process !== "undefined" &&
    process !== null &&
    typeof process.env === "object" &&
    process.env !== null
  ) {
    return process.env;
  }

  return undefined;
};

const readEnvRpcUrl = (): string | undefined => {
  const env = getRuntimeEnv();
  if (!env) return undefined;

  for (const key of RPC_URL_ENV_KEYS) {
    const value = env[key];
    if (typeof value !== "string") continue;
    const normalized = normalizePhoenixRpcUrl(value);
    if (normalized) {
      return normalized;
    }
  }

  return undefined;
};

export const resolvePhoenixRpcUrl = (
  config: PhoenixRpcUrlConfig = {}
): string | undefined => {
  const resolved = config.rpcUrl ?? readEnvRpcUrl();
  if (resolved === undefined) {
    return undefined;
  }

  const normalized = normalizePhoenixRpcUrl(resolved);
  if (!normalized) {
    throw new Error("rpcUrl must be a non-empty string");
  }
  return normalized;
};
