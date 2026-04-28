import {
  decodeGlobalConfiguration,
  decodeOrderbook,
  decodeOrderbookHeader,
  decodePerpAssetMap,
  decodeSplineCollection,
  type GlobalConfiguration,
  type Orderbook,
  type OrderbookHeader,
  type PerpAssetMap,
  type SplineCollection,
} from "@/accounts";
import type { AccountFetcherClient } from "@/accounts/fetcherFactory";
import type { PhoenixExchangeMetadata } from "@/exchange-cache";
import type { PhoenixPdaClient } from "@/pdaClient";
import type { Address } from "@solana/kit";
import type { Symbol } from "./primitives";
import { decodeBase64ToBytes } from "./base64";
import { resolvePhoenixRpcUrl } from "./rpcUrl";

type RpcAccountPayload = {
  readonly data: Uint8Array;
};

type RpcAccountInfoResult = {
  context?: {
    slot?: number | string | bigint;
  };
  value?: { data?: [string, string] | null } | null;
};

type RpcMultipleAccountsResult = {
  context?: {
    slot?: number | string | bigint;
  };
  value?: Array<{ data?: [string, string] | null } | null>;
};

const toRpcSlot = (value: number | string | bigint | undefined): bigint => {
  if (typeof value === "bigint") {
    return value;
  }
  if (typeof value === "number") {
    return BigInt(value);
  }
  if (typeof value === "string") {
    return BigInt(value);
  }
  throw new Error("RPC response did not include a slot");
};

const decodeRpcAccountData = (
  encoded: [string, string] | null | undefined,
  address: Address
): RpcAccountPayload => {
  if (!encoded || encoded[1] !== "base64") {
    throw new Error(`Account ${address} was not returned from RPC`);
  }

  return {
    data: decodeBase64ToBytes(encoded[0]),
  };
};

export class PhoenixRpcAccountFetcherClient implements AccountFetcherClient {
  constructor(private readonly url: string) {}

  private async request<TResult>(
    method: string,
    params: unknown[]
  ): Promise<TResult> {
    const response = await globalThis.fetch(this.url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method,
        params,
      }),
    });

    if (!response.ok) {
      throw new Error(`RPC ${method} failed with ${response.status}`);
    }

    const payload = (await response.json()) as {
      error?: { message?: string };
      result?: TResult;
    };

    if (payload.error) {
      throw new Error(payload.error.message ?? `RPC ${method} failed`);
    }

    if (payload.result === undefined) {
      throw new Error(`RPC ${method} returned no result`);
    }

    return payload.result;
  }

  async fetchAccount(address: Address): Promise<{ readonly data: Uint8Array }> {
    const result = await this.request<RpcAccountInfoResult>("getAccountInfo", [
      address,
      { encoding: "base64", commitment: "confirmed" },
    ]);

    return decodeRpcAccountData(result.value?.data, address);
  }

  async fetchAccounts(addresses: readonly Address[]): Promise<{
    readonly slot: bigint;
    readonly accounts: readonly RpcAccountPayload[];
  }> {
    const result = await this.request<RpcMultipleAccountsResult>(
      "getMultipleAccounts",
      [addresses, { encoding: "base64", commitment: "confirmed" }]
    );

    const values = result.value;
    if (!values || values.length !== addresses.length) {
      throw new Error("RPC getMultipleAccounts returned an unexpected payload");
    }

    return {
      slot: toRpcSlot(result.context?.slot),
      accounts: values.map((account, index) =>
        decodeRpcAccountData(account?.data, addresses[index])
      ),
    };
  }
}

export interface PhoenixRpcClient {
  readonly available: boolean;
  accounts: {
    fetchAccount(address: Address): Promise<{ readonly data: Uint8Array }>;
  };
  exchange: {
    getGlobalConfiguration(): Promise<GlobalConfiguration>;
    getPerpAssetMap(): Promise<PerpAssetMap>;
  };
  markets: {
    getOrderbookHeader(symbol: Symbol): Promise<OrderbookHeader>;
    getOrderbook(symbol: Symbol): Promise<Orderbook>;
    getSplineCollection(symbol: Symbol): Promise<SplineCollection>;
  };
}

const assertRpcClient = (
  fetcher: PhoenixRpcAccountFetcherClient | undefined
): PhoenixRpcAccountFetcherClient => {
  if (!fetcher) {
    throw new Error(
      "RPC client is unavailable. Pass rpcUrl to createPhoenixClient(...) or set SOLANA_RPC_URL to enable client.rpc."
    );
  }
  return fetcher;
};

const getMarketOrThrow = async (
  exchange: PhoenixExchangeMetadata,
  symbol: Symbol
) => {
  await exchange.ready();
  const market = exchange.market(symbol);
  if (market) {
    return market;
  }
  const availableSymbols = exchange
    .snapshot()
    .markets.map((entry) => entry.symbol)
    .join(", ");
  throw new Error(
    `Unknown market symbol '${symbol}'. Available symbols: ${availableSymbols}`
  );
};

export const createPhoenixRpcClient = (config: {
  rpcUrl?: string;
  exchange: PhoenixExchangeMetadata;
  pda: PhoenixPdaClient;
}): PhoenixRpcClient => {
  const rpcUrl = resolvePhoenixRpcUrl({
    rpcUrl: config.rpcUrl,
  });
  const fetcher = rpcUrl
    ? new PhoenixRpcAccountFetcherClient(rpcUrl)
    : undefined;

  return {
    available: fetcher !== undefined,
    accounts: {
      fetchAccount: async (address) =>
        assertRpcClient(fetcher).fetchAccount(address),
    },
    exchange: {
      getGlobalConfiguration: async () => {
        const client = assertRpcClient(fetcher);
        const globalConfigurationAddress =
          await config.pda.getGlobalConfigurationAddress();
        const account = await client.fetchAccount(globalConfigurationAddress);
        return decodeGlobalConfiguration(account.data);
      },
      getPerpAssetMap: async () => {
        const client = assertRpcClient(fetcher);
        const globalConfigurationAddress =
          await config.pda.getGlobalConfigurationAddress();
        const globalConfigAccount = await client.fetchAccount(
          globalConfigurationAddress
        );
        const globalConfiguration = decodeGlobalConfiguration(
          globalConfigAccount.data
        );
        const perpAssetMapAccount = await client.fetchAccount(
          globalConfiguration.perpAssetMapKey
        );
        return decodePerpAssetMap(perpAssetMapAccount.data);
      },
    },
    markets: {
      getOrderbookHeader: async (symbol) => {
        const client = assertRpcClient(fetcher);
        const market = await getMarketOrThrow(config.exchange, symbol);
        const account = await client.fetchAccount(
          market.marketPubkey as Address
        );
        return decodeOrderbookHeader(account.data);
      },
      getOrderbook: async (symbol) => {
        const client = assertRpcClient(fetcher);
        const market = await getMarketOrThrow(config.exchange, symbol);
        const account = await client.fetchAccount(
          market.marketPubkey as Address
        );
        return decodeOrderbook(account.data);
      },
      getSplineCollection: async (symbol) => {
        const client = assertRpcClient(fetcher);
        const market = await getMarketOrThrow(config.exchange, symbol);
        const account = await client.fetchAccount(
          market.splinePubkey as Address
        );
        return decodeSplineCollection(account.data);
      },
    },
  };
};
