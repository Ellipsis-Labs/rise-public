import type { Address, Decoder, ReadonlyUint8Array } from "@solana/kit";

export interface AccountFetcherClient {
  fetchAccount: (address: Address) => Promise<{
    readonly data: ReadonlyUint8Array;
  }>;
  /**
   * Optional batch fetch backed by a single `getMultipleAccounts` round-trip
   * (the implementation may chunk internally to respect the 100-account RPC
   * limit). Returns one entry per input address, in order, with `null` for
   * accounts that do not exist — unlike a strict batch fetch, missing accounts
   * are tolerated rather than throwing, which is what makes it usable for
   * sparse scans (e.g. isolated-subaccount allocation). When present, callers
   * that would otherwise probe many addresses sequentially collapse those
   * round-trips into one; clients that do not implement it fall back to
   * per-address `fetchAccount`/`accountExists`.
   */
  fetchMaybeAccounts?: (
    addresses: readonly Address[]
  ) => Promise<ReadonlyArray<{ readonly data: ReadonlyUint8Array } | null>>;
  readonly _accountCache?: Map<string, unknown>;
  readonly _cacheEnabled?: boolean;
}

export interface AccountFetcherClientWithAddresses extends AccountFetcherClient {
  readonly addresses: {
    readonly globalConfigurationAddress: Address;
  };
}

export const getOrFetchCached = async <T>(
  client: AccountFetcherClient,
  cacheKey: string,
  fetcher: () => Promise<T>
): Promise<T> => {
  if (client._cacheEnabled === false || !client._accountCache) {
    return fetcher();
  }

  if (client._accountCache.has(cacheKey)) {
    return client._accountCache.get(cacheKey) as T;
  }

  const result = await fetcher();
  client._accountCache.set(cacheKey, result);
  return result;
};

export const invalidateCacheKey = (
  client: AccountFetcherClient,
  cacheKey: string
): boolean => {
  if (!client._accountCache) {
    return false;
  }

  return client._accountCache.delete(cacheKey);
};

interface FetchAccountParams<T> {
  client: AccountFetcherClient;
  address: Address;
  decoder: Decoder<T>;
  cacheKey?: string;
  skipCache?: boolean;
}

const fetchAccount = async <T>({
  client,
  address,
  decoder,
  cacheKey,
  skipCache,
}: FetchAccountParams<T>): Promise<T> => {
  const key = cacheKey ?? `account:${address}`;

  const getData = async () => {
    const account = await client.fetchAccount(address);
    return decoder.decode(account.data);
  };

  if (skipCache) {
    return getData();
  }

  return getOrFetchCached(client, key, getData);
};

export type AccountFetcher = <
  T,
  TAddress extends Address,
  TClient extends AccountFetcherClient = AccountFetcherClient,
>(params: {
  client: TClient;
  address: TAddress;
  skipCache?: boolean;
}) => Promise<T>;

export type AccountFetcherFor<
  T,
  TAddress extends Address,
  TClient extends AccountFetcherClient = AccountFetcherClient,
> = (params: {
  client: TClient;
  address: TAddress;
  skipCache?: boolean;
}) => Promise<T>;

export type AccountFetcherWithDefaultFor<
  T,
  TClient extends AccountFetcherClient = AccountFetcherClient,
> = (params: {
  client: TClient;
  address?: Address;
  skipCache?: boolean;
}) => Promise<T>;

export const createAccountFetcher = <
  T,
  TAddress extends Address,
  TClient extends AccountFetcherClient = AccountFetcherClient,
>(
  getDecoder: () => Decoder<T>
): ((params: {
  client: TClient;
  address: TAddress;
  skipCache?: boolean;
}) => Promise<T>) => {
  return async (params: {
    client: TClient;
    address: TAddress;
    skipCache?: boolean;
  }): Promise<T> => {
    return fetchAccount({
      client: params.client,
      address: params.address,
      decoder: getDecoder(),
      skipCache: params.skipCache,
    });
  };
};

export const createAccountFetcherWithDefault = <
  T,
  TClient extends AccountFetcherClient = AccountFetcherClient,
>(
  getDecoder: () => Decoder<T>,
  defaultAddress: Address
): ((params: {
  client: TClient;
  address?: Address;
  skipCache?: boolean;
}) => Promise<T>) => {
  return async (params: {
    client: TClient;
    address?: Address;
    skipCache?: boolean;
  }): Promise<T> => {
    return fetchAccount({
      client: params.client,
      address: params.address ?? defaultAddress,
      decoder: getDecoder(),
      skipCache: params.skipCache,
    });
  };
};
