import { getPhoenixProgramAddress } from "@/core/constants";
import {
  getEmberStateAddress,
  getEmberVaultAddress,
  getPhoenixConditionalOrdersAddress,
  getPhoenixEscrowAddress,
  getPhoenixGlobalConfigurationAddress,
  getPhoenixGlobalVaultAddress,
  getPhoenixLogAuthorityAddress,
  getPhoenixPermissionAddress,
  getPhoenixSplineCollectionAddress,
  getPhoenixStopLossAddress,
  getPhoenixTraderSubaccountAddress,
  getPhoenixTraderTokenAccountAddress,
  type StopLossAddressParams,
  type TraderSubaccountAddressParams,
} from "@/pdas";
import type {
  Authority,
  EmberStateAddress,
  EmberVaultAddress,
  GlobalConfigurationAddress,
  GlobalVaultAddress,
  LogAuthorityAddress,
  MarketAddress,
  MintAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  TokenAccountAddress,
  TraderAddress,
} from "@/primitives";
import type { Address } from "@solana/kit";
import type {
  FlightBuilderStateAddress,
  FlightGlobalStateAddress,
} from "./flight/types.js";
import {
  getFlightBuilderStateAddress,
  getFlightGlobalStateAddress,
} from "./flight/pdas.js";

export interface PhoenixPdaAddressOverrides {
  programAddress?: PhoenixProgramAddress;
  phoenixProgramAddress?: PhoenixProgramAddress;
}

export interface PhoenixPdaClientConfig {
  phoenixEnv?: string | null;
  programAddress?: PhoenixProgramAddress;
  addresses?: PhoenixPdaAddressOverrides;
  cache?: boolean | PhoenixPdaCacheConfig;
}

export interface PhoenixPdaCacheConfig {
  /**
   * Maximum number of cached PDA derivations retained per PDA client.
   *
   * The cache uses LRU eviction when this limit is reached.
   */
  maxEntries?: number;
}

export interface PhoenixProgramAddressOverride {
  phoenixProgramAddress?: PhoenixProgramAddress;
}

export interface PhoenixPdaClient {
  getProgramAddress: (
    input?: PhoenixProgramAddressOverride
  ) => PhoenixProgramAddress;
  getLogAuthorityAddress: (
    input?: PhoenixProgramAddressOverride
  ) => Promise<LogAuthorityAddress>;
  getGlobalConfigurationAddress: (
    input?: PhoenixProgramAddressOverride
  ) => Promise<GlobalConfigurationAddress>;
  getSplineCollectionAddress: (input: {
    marketAccountAddress: MarketAddress;
    phoenixProgramAddress?: PhoenixProgramAddress;
  }) => Promise<SplineCollectionAddress>;
  getTraderAddress: (
    input: TraderSubaccountAddressParams
  ) => Promise<TraderAddress>;
  getTraderTokenAccountAddress: (input: {
    authority: Authority;
    mint: MintAddress;
  }) => Promise<TokenAccountAddress>;
  getGlobalVaultAddress: (input: {
    mint: MintAddress;
    phoenixProgramAddress?: PhoenixProgramAddress;
  }) => Promise<GlobalVaultAddress>;
  getEmberStateAddress: (
    input?: PhoenixProgramAddressOverride
  ) => Promise<EmberStateAddress>;
  getEmberVaultAddress: (
    input?: PhoenixProgramAddressOverride
  ) => Promise<EmberVaultAddress>;
  getPermissionAddress: (input: {
    permissionAuthority: Address;
    delegatedKey: Address;
    phoenixProgramAddress?: PhoenixProgramAddress;
  }) => Promise<Address>;
  getEscrowAddress: (input: {
    authority: Address;
    phoenixProgramAddress?: PhoenixProgramAddress;
  }) => Promise<Address>;
  getStopLossAddress: (input: StopLossAddressParams) => Promise<Address>;
  getConditionalOrdersAddress: (input: {
    traderAccount: TraderAddress;
    phoenixProgramAddress?: PhoenixProgramAddress;
  }) => Promise<Address>;
  getFlightGlobalStateAddress: (
    input?: PhoenixProgramAddressOverride
  ) => Promise<FlightGlobalStateAddress>;
  getFlightBuilderStateAddress: (
    authority: Authority,
    input?: PhoenixProgramAddressOverride
  ) => Promise<FlightBuilderStateAddress>;
  clearCache: () => void;
}

const getConfiguredProgramAddress = (
  config: PhoenixPdaClientConfig,
  override?: PhoenixProgramAddress
): PhoenixProgramAddress => {
  const configuredProgramAddress =
    override ??
    config.addresses?.programAddress ??
    config.addresses?.phoenixProgramAddress ??
    config.programAddress;

  return getPhoenixProgramAddress({
    phoenixEnv: config.phoenixEnv,
    programAddress: configuredProgramAddress,
  });
};

const cacheKey = (...segments: Array<string | number | bigint>): string =>
  segments.map((segment) => segment.toString()).join(":");

const DEFAULT_PDA_CACHE_MAX_ENTRIES = 1_024;

interface PhoenixPdaPromiseCache {
  get: <TResult>(key: string) => Promise<TResult> | undefined;
  set: <TResult>(key: string, value: Promise<TResult>) => void;
  delete: (key: string) => void;
  clear: () => void;
}

const normalizePdaCacheConfig = (
  cache: PhoenixPdaClientConfig["cache"]
): { enabled: boolean; maxEntries: number } => {
  if (cache === false) {
    return {
      enabled: false,
      maxEntries: 0,
    };
  }

  if (cache === true || cache === undefined) {
    return {
      enabled: true,
      maxEntries: DEFAULT_PDA_CACHE_MAX_ENTRIES,
    };
  }

  const maxEntries = cache.maxEntries ?? DEFAULT_PDA_CACHE_MAX_ENTRIES;
  if (!Number.isInteger(maxEntries) || maxEntries < 1) {
    throw new Error("pda cache maxEntries must be a positive integer");
  }

  return {
    enabled: true,
    maxEntries,
  };
};

const createPdaPromiseCache = (
  cache: PhoenixPdaClientConfig["cache"]
): PhoenixPdaPromiseCache => {
  const config = normalizePdaCacheConfig(cache);
  if (!config.enabled) {
    return {
      get: () => undefined,
      set: () => {},
      delete: () => {},
      clear: () => {},
    };
  }

  const cacheStore = new Map<string, Promise<unknown>>();

  const touch = (key: string, value: Promise<unknown>): void => {
    cacheStore.delete(key);
    cacheStore.set(key, value);
  };

  return {
    get: <TResult>(key: string): Promise<TResult> | undefined => {
      const cached = cacheStore.get(key);
      if (!cached) {
        return undefined;
      }

      touch(key, cached);
      return cached as Promise<TResult>;
    },
    set: <TResult>(key: string, value: Promise<TResult>): void => {
      touch(key, value as Promise<unknown>);
      while (cacheStore.size > config.maxEntries) {
        const oldestKey = cacheStore.keys().next().value;
        if (oldestKey === undefined) {
          break;
        }
        cacheStore.delete(oldestKey);
      }
    },
    delete: (key: string): void => {
      cacheStore.delete(key);
    },
    clear: (): void => {
      cacheStore.clear();
    },
  };
};

const memoizeAsync = <TArgs extends readonly unknown[], TResult>(
  cache: PhoenixPdaPromiseCache,
  keyFor: (...args: TArgs) => string,
  derive: (...args: TArgs) => Promise<TResult>
): ((...args: TArgs) => Promise<TResult>) => {
  return (...args: TArgs) => {
    const key = keyFor(...args);
    const cached = cache.get<TResult>(key);
    if (cached) return cached;

    const pending: Promise<TResult> = derive(...args).catch((error) => {
      cache.delete(key);
      throw error;
    });
    cache.set<TResult>(key, pending);
    return pending;
  };
};

export const createPhoenixPdaClient = (
  config: PhoenixPdaClientConfig = {}
): PhoenixPdaClient => {
  const cache = createPdaPromiseCache(config.cache);
  const resolveProgramAddress = (
    input?: PhoenixProgramAddressOverride
  ): PhoenixProgramAddress =>
    getConfiguredProgramAddress(config, input?.phoenixProgramAddress);

  return {
    getProgramAddress: (input = {}) => resolveProgramAddress(input),
    getLogAuthorityAddress: memoizeAsync(
      cache,
      (input?: PhoenixProgramAddressOverride) =>
        cacheKey("logAuthority", resolveProgramAddress(input)),
      (input: PhoenixProgramAddressOverride = {}) =>
        getPhoenixLogAuthorityAddress(resolveProgramAddress(input))
    ),
    getGlobalConfigurationAddress: memoizeAsync(
      cache,
      (input?: PhoenixProgramAddressOverride) =>
        cacheKey("globalConfiguration", resolveProgramAddress(input)),
      (input: PhoenixProgramAddressOverride = {}) =>
        getPhoenixGlobalConfigurationAddress(resolveProgramAddress(input))
    ),
    getSplineCollectionAddress: memoizeAsync(
      cache,
      (input) =>
        cacheKey(
          "splineCollection",
          resolveProgramAddress(input),
          input.marketAccountAddress
        ),
      (input) =>
        getPhoenixSplineCollectionAddress(
          input.marketAccountAddress,
          resolveProgramAddress(input)
        )
    ),
    getTraderAddress: memoizeAsync(
      cache,
      (input) =>
        cacheKey(
          "trader",
          resolveProgramAddress(input),
          input.authority,
          input.traderPdaIndex,
          input.subaccountIndex
        ),
      (input) =>
        getPhoenixTraderSubaccountAddress({
          ...input,
          phoenixProgramAddress: resolveProgramAddress(input),
        })
    ),
    getTraderTokenAccountAddress: memoizeAsync(
      cache,
      (input) => cacheKey("traderTokenAccount", input.authority, input.mint),
      (input) =>
        getPhoenixTraderTokenAccountAddress(input.authority, input.mint)
    ),
    getGlobalVaultAddress: memoizeAsync(
      cache,
      (input) =>
        cacheKey("globalVault", resolveProgramAddress(input), input.mint),
      (input) =>
        getPhoenixGlobalVaultAddress(input.mint, resolveProgramAddress(input))
    ),
    getEmberStateAddress: memoizeAsync(
      cache,
      (input?: PhoenixProgramAddressOverride) =>
        cacheKey("emberState", resolveProgramAddress(input)),
      (input: PhoenixProgramAddressOverride = {}) =>
        getEmberStateAddress(resolveProgramAddress(input))
    ),
    getEmberVaultAddress: memoizeAsync(
      cache,
      (input?: PhoenixProgramAddressOverride) =>
        cacheKey("emberVault", resolveProgramAddress(input)),
      (input: PhoenixProgramAddressOverride = {}) =>
        getEmberVaultAddress(resolveProgramAddress(input))
    ),
    getPermissionAddress: memoizeAsync(
      cache,
      (input) =>
        cacheKey(
          "permission",
          resolveProgramAddress(input),
          input.permissionAuthority,
          input.delegatedKey
        ),
      (input) =>
        getPhoenixPermissionAddress(
          input.permissionAuthority,
          input.delegatedKey,
          resolveProgramAddress(input)
        )
    ),
    getEscrowAddress: memoizeAsync(
      cache,
      (input) =>
        cacheKey("escrow", resolveProgramAddress(input), input.authority),
      (input) =>
        getPhoenixEscrowAddress(input.authority, resolveProgramAddress(input))
    ),
    getStopLossAddress: memoizeAsync(
      cache,
      (input) =>
        cacheKey(
          "stopLoss",
          resolveProgramAddress(input),
          input.traderAccount,
          input.assetId
        ),
      (input) =>
        getPhoenixStopLossAddress({
          ...input,
          phoenixProgramAddress: resolveProgramAddress(input),
        })
    ),
    getConditionalOrdersAddress: memoizeAsync(
      cache,
      (input) =>
        cacheKey(
          "conditionalOrders",
          resolveProgramAddress(input),
          input.traderAccount
        ),
      (input) =>
        getPhoenixConditionalOrdersAddress({
          ...input,
          phoenixProgramAddress: resolveProgramAddress(input),
        })
    ),
    getFlightGlobalStateAddress: memoizeAsync(
      cache,
      (input?: PhoenixProgramAddressOverride) =>
        cacheKey("flightGlobalState", resolveProgramAddress(input)),
      (input: PhoenixProgramAddressOverride = {}) =>
        getFlightGlobalStateAddress(resolveProgramAddress(input))
    ),
    getFlightBuilderStateAddress: memoizeAsync(
      cache,
      (authority, input?: PhoenixProgramAddressOverride) =>
        cacheKey("flightBuilderState", resolveProgramAddress(input), authority),
      (authority, input: PhoenixProgramAddressOverride = {}) =>
        getFlightBuilderStateAddress(authority, resolveProgramAddress(input))
    ),
    clearCache: () => {
      cache.clear();
    },
  };
};
