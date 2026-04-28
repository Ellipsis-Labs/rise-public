export type RiseDebugNamespace = "auth" | "cache" | "http" | "ix" | "ws";

type DebugFlagKeys = {
  env: readonly string[];
  globals: readonly string[];
  storage: readonly string[];
};

const COMMON_DEBUG_KEYS: DebugFlagKeys = {
  env: ["PHOENIX_DEBUG", "PHOENIX_DEBUG_RISE"],
  globals: ["__PHOENIX_DEBUG__", "__PHOENIX_DEBUG_RISE__"],
  storage: ["phoenix.debug", "phoenix-rise.debug", "phoenix.debug.rise"],
};

const DEBUG_KEYS_BY_NAMESPACE: Record<RiseDebugNamespace, DebugFlagKeys> = {
  auth: {
    env: ["PHOENIX_DEBUG_AUTH"],
    globals: ["__PHOENIX_DEBUG_AUTH__"],
    storage: ["phoenix-debug-auth", "phoenix.debug.auth"],
  },
  cache: {
    env: ["PHOENIX_DEBUG_CACHE"],
    globals: ["__PHOENIX_DEBUG_CACHE__"],
    storage: [
      "phoenix-debug-cache",
      "phoenix.cache.debug",
      "phoenix.debug.cache",
    ],
  },
  http: {
    env: ["PHOENIX_DEBUG_HTTP"],
    globals: ["__PHOENIX_DEBUG_HTTP__"],
    storage: ["phoenix-debug-http", "phoenix.http.debug", "phoenix.debug.http"],
  },
  ix: {
    env: ["PHOENIX_DEBUG_IX"],
    globals: ["__PHOENIX_DEBUG_IX__"],
    storage: ["phoenix-debug-ix", "phoenix.ix.debug", "phoenix.debug.ix"],
  },
  ws: {
    env: ["PHOENIX_DEBUG_WS"],
    globals: ["__PHOENIX_DEBUG_WS__"],
    storage: ["phoenix.ws.debug", "phoenix.debug.ws", "phoenix-debug-ws"],
  },
};

const readEnvFlag = (keys: readonly string[]): boolean => {
  if (typeof process === "undefined") {
    return false;
  }

  return keys.some((key) => process.env[key] === "1");
};

const readWindowFlag = (keys: readonly string[]): boolean => {
  if (typeof window === "undefined") {
    return false;
  }

  const globalWindow = window as typeof window & Record<string, unknown>;
  return keys.some((key) => {
    const value = globalWindow[key];
    return value === true || value === "1";
  });
};

const readStorageFlag = (keys: readonly string[]): boolean => {
  if (typeof window === "undefined") {
    return false;
  }

  try {
    const localStorage = window.localStorage;
    if (!localStorage) {
      return false;
    }

    return keys.some((key) => localStorage.getItem(key) === "1");
  } catch {
    return false;
  }
};

const readDebugFlag = (namespace: RiseDebugNamespace): boolean => {
  const keys = DEBUG_KEYS_BY_NAMESPACE[namespace];
  return (
    readEnvFlag(COMMON_DEBUG_KEYS.env) ||
    readEnvFlag(keys.env) ||
    readWindowFlag(COMMON_DEBUG_KEYS.globals) ||
    readWindowFlag(keys.globals) ||
    readStorageFlag(COMMON_DEBUG_KEYS.storage) ||
    readStorageFlag(keys.storage)
  );
};

export const isRiseDebugEnabled = (namespace: RiseDebugNamespace): boolean =>
  readDebugFlag(namespace);

export const summarizeDebugError = (
  error: unknown
): Record<string, unknown> => {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
    };
  }

  return {
    message: String(error),
  };
};

export const debugRise = (
  namespace: RiseDebugNamespace,
  message: string,
  details?: Record<string, unknown>
): void => {
  if (!readDebugFlag(namespace)) {
    return;
  }

  const prefix = `[${new Date().toISOString()}] [phoenix-rise:${namespace}] ${message}`;
  if (details) {
    console.debug(prefix, details);
    return;
  }

  console.debug(prefix);
};
