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
import {
  HAWKEYE_PROGRAM_ADDRESS,
  HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT,
  buildHawkeyeViewBboIx,
  buildHawkeyeViewFundingIx,
  buildHawkeyeViewLiquidationPriceIx,
  buildHawkeyeViewMarginForAssetIx,
  buildHawkeyeViewMarginIx,
  decodeHawkeyeReturnData,
  encodeHawkeyeSimulationTransaction,
  type HawkeyeAssetReturn,
  type HawkeyeBboReturn,
  type HawkeyeFundingReturn,
  type HawkeyeLiquidationPriceReturn,
  type HawkeyeMarginReturn,
  type HawkeyeReturnData,
  type HawkeyeTraderViewAccounts,
} from "@/hawkeye";
import type { PhoenixPdaClient } from "@/pdaClient";
import type { Address } from "@solana/kit";
import type { Blockhash, Instruction } from "@solana/kit";
import type { Symbol } from "./primitives";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalConfigurationAddress,
  GlobalTraderIndexAddressArray,
  MarketAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  TraderAddress,
} from "./primitives";
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

  async request<TResult>(method: string, params: unknown[]): Promise<TResult> {
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

  async fetchMaybeAccounts(
    addresses: readonly Address[]
  ): Promise<ReadonlyArray<{ readonly data: Uint8Array } | null>> {
    if (addresses.length === 0) {
      return [];
    }

    // getMultipleAccounts caps at 100 accounts per request; chunk to be safe.
    const results: ({ readonly data: Uint8Array } | null)[] = [];
    for (let start = 0; start < addresses.length; start += 100) {
      const chunk = addresses.slice(start, start + 100);
      const result = await this.request<RpcMultipleAccountsResult>(
        "getMultipleAccounts",
        [chunk, { encoding: "base64", commitment: "confirmed" }]
      );
      const values = result.value ?? [];
      for (let i = 0; i < chunk.length; i++) {
        const encoded = values[i]?.data;
        // Tolerate gaps: a missing account is `null`, not an error.
        results.push(
          encoded && encoded[1] === "base64"
            ? { data: decodeBase64ToBytes(encoded[0]) }
            : null
        );
      }
    }
    return results;
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
  hawkeye: PhoenixHawkeyeRpcClient;
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

export type HawkeyeTraderLookup =
  | {
      authority: Authority;
      traderAccount?: never;
      traderPdaIndex?: number;
      traderSubaccountIndex?: number;
    }
  | {
      authority?: never;
      traderAccount: TraderAddress;
      traderPdaIndex?: never;
      traderSubaccountIndex?: never;
    };

export type HawkeyeAssetLookup =
  | { assetId: number; symbol?: Symbol }
  | { symbol: Symbol; assetId?: number };

export type HawkeyeTraderAssetLookup = HawkeyeTraderLookup & HawkeyeAssetLookup;

export interface HawkeyeSimulationReturnData<
  TDecoded extends HawkeyeReturnData,
> {
  programId: string | null;
  encoding: string;
  base64: string;
  hex: string;
  byteLength: number;
  decoded: TDecoded;
}

export interface HawkeyeSimulation<TDecoded extends HawkeyeReturnData> {
  err: unknown;
  logs: string[];
  unitsConsumed: number | null;
  returnData: HawkeyeSimulationReturnData<TDecoded> | null;
}

export interface PhoenixHawkeyeRpcClient {
  simulateInstruction<TDecoded extends HawkeyeReturnData = HawkeyeReturnData>(
    instruction: Instruction
  ): Promise<HawkeyeSimulation<TDecoded>>;
  viewMargin(
    params: HawkeyeTraderLookup
  ): Promise<HawkeyeSimulation<HawkeyeMarginReturn>>;
  viewMarginForAsset(
    params: HawkeyeTraderAssetLookup
  ): Promise<HawkeyeSimulation<HawkeyeAssetReturn>>;
  viewLiquidationPrice(
    params: HawkeyeTraderAssetLookup
  ): Promise<HawkeyeSimulation<HawkeyeLiquidationPriceReturn>>;
  viewBbo(params: {
    symbol: Symbol;
  }): Promise<HawkeyeSimulation<HawkeyeBboReturn>>;
  viewFunding(
    params: HawkeyeTraderAssetLookup
  ): Promise<HawkeyeSimulation<HawkeyeFundingReturn>>;
}

type RpcLatestBlockhashResult = {
  value: {
    blockhash: string;
    lastValidBlockHeight: string | number | bigint;
  };
};

type RpcSimulationResult = {
  value: {
    err?: unknown;
    logs?: string[] | null;
    unitsConsumed?: number | null;
    returnData?: {
      programId?: string;
      data?: [string, string] | string;
    } | null;
  };
};

const toHex = (bytes: Uint8Array): string =>
  Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");

const toBigInt = (value: string | number | bigint): bigint =>
  typeof value === "bigint" ? value : BigInt(value);

const decodeSimulationReturnData = <TDecoded extends HawkeyeReturnData>(
  returnData: RpcSimulationResult["value"]["returnData"]
): HawkeyeSimulationReturnData<TDecoded> | null => {
  if (!returnData) {
    return null;
  }
  const dataField = returnData.data;
  const base64 = Array.isArray(dataField) ? dataField[0] : dataField;
  const encoding = Array.isArray(dataField) ? dataField[1] : "base64";
  if (typeof base64 !== "string") {
    return null;
  }
  if (encoding !== "base64") {
    throw new Error(`Unsupported Hawkeye return-data encoding '${encoding}'`);
  }
  if (
    returnData.programId &&
    returnData.programId !== HAWKEYE_PROGRAM_ADDRESS
  ) {
    throw new Error(
      `Hawkeye simulation returned data for program ${returnData.programId}, expected ${HAWKEYE_PROGRAM_ADDRESS}`
    );
  }
  const bytes = decodeBase64ToBytes(base64);
  return {
    programId: returnData.programId ?? null,
    encoding,
    base64,
    hex: toHex(bytes),
    byteLength: bytes.byteLength,
    decoded: decodeHawkeyeReturnData(bytes) as TDecoded,
  };
};

const createPhoenixHawkeyeRpcClient = (config: {
  fetcher?: PhoenixRpcAccountFetcherClient;
  exchange: PhoenixExchangeMetadata;
  pda: PhoenixPdaClient;
}): PhoenixHawkeyeRpcClient => {
  const simulateInstruction = async <
    TDecoded extends HawkeyeReturnData = HawkeyeReturnData,
  >(
    instruction: Instruction
  ): Promise<HawkeyeSimulation<TDecoded>> => {
    const client = assertRpcClient(config.fetcher);
    const latestBlockhash = await client.request<RpcLatestBlockhashResult>(
      "getLatestBlockhash",
      [{ commitment: "confirmed" }]
    );
    const encodedTransaction = encodeHawkeyeSimulationTransaction({
      instruction,
      blockhash: latestBlockhash.value.blockhash as Blockhash,
      lastValidBlockHeight: toBigInt(
        latestBlockhash.value.lastValidBlockHeight
      ),
      computeUnitLimit: HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT,
    });
    const result = await client.request<RpcSimulationResult>(
      "simulateTransaction",
      [
        encodedTransaction,
        {
          commitment: "confirmed",
          encoding: "base64",
          replaceRecentBlockhash: true,
          sigVerify: false,
        },
      ]
    );

    return {
      err: result.value.err ?? null,
      logs: result.value.logs ?? [],
      unitsConsumed: result.value.unitsConsumed ?? null,
      returnData: decodeSimulationReturnData<TDecoded>(
        result.value.returnData ?? null
      ),
    };
  };

  const resolveExchangeAccounts = async () => {
    await config.exchange.ready();
    const exchange = config.exchange.snapshot().exchange;
    return {
      phoenixProgramAddress: (exchange.programId ??
        config.pda.getProgramAddress()) as PhoenixProgramAddress,
      globalConfigurationAddress:
        exchange.globalConfig as GlobalConfigurationAddress,
      globalTraderIndex:
        exchange.globalTraderIndex as GlobalTraderIndexAddressArray,
      activeTraderBuffer:
        exchange.activeTraderBuffer as ActiveTraderBufferAddressArray,
      perpAssetMap: exchange.perpAssetMap as PerpAssetMapAddress,
    };
  };

  const resolveTraderAccount = async (
    params: HawkeyeTraderLookup
  ): Promise<TraderAddress> => {
    if (params.traderAccount !== undefined) {
      return params.traderAccount;
    }
    return config.pda.getTraderAddress({
      authority: params.authority,
      traderPdaIndex: params.traderPdaIndex ?? 0,
      subaccountIndex: params.traderSubaccountIndex ?? 0,
      phoenixProgramAddress: config.pda.getProgramAddress(),
    });
  };

  const resolveTraderAccounts = async (
    params: HawkeyeTraderLookup
  ): Promise<HawkeyeTraderViewAccounts> => ({
    ...(await resolveExchangeAccounts()),
    traderAccount: await resolveTraderAccount(params),
  });

  const resolveAssetId = async (
    params: HawkeyeAssetLookup
  ): Promise<number> => {
    if (params.assetId !== undefined) {
      return params.assetId;
    }
    if (params.symbol === undefined) {
      throw new Error("symbol or assetId is required for this Hawkeye view");
    }
    const market = await getMarketOrThrow(config.exchange, params.symbol);
    return market.assetId;
  };

  return {
    simulateInstruction,
    viewMargin: async (params) =>
      simulateInstruction<HawkeyeMarginReturn>(
        buildHawkeyeViewMarginIx(await resolveTraderAccounts(params))
      ),
    viewMarginForAsset: async (params) =>
      simulateInstruction<HawkeyeAssetReturn>(
        buildHawkeyeViewMarginForAssetIx({
          ...(await resolveTraderAccounts(params)),
          assetId: await resolveAssetId(params),
        })
      ),
    viewLiquidationPrice: async (params) =>
      simulateInstruction<HawkeyeLiquidationPriceReturn>(
        buildHawkeyeViewLiquidationPriceIx({
          ...(await resolveTraderAccounts(params)),
          assetId: await resolveAssetId(params),
        })
      ),
    viewBbo: async (params) => {
      const [exchangeAccounts, market] = await Promise.all([
        resolveExchangeAccounts(),
        getMarketOrThrow(config.exchange, params.symbol),
      ]);
      return simulateInstruction<HawkeyeBboReturn>(
        buildHawkeyeViewBboIx({
          ...exchangeAccounts,
          orderbook: market.marketPubkey as MarketAddress,
          splineCollection: market.splinePubkey as SplineCollectionAddress,
        })
      );
    },
    viewFunding: async (params) =>
      simulateInstruction<HawkeyeFundingReturn>(
        buildHawkeyeViewFundingIx({
          ...(await resolveTraderAccounts(params)),
          assetId: await resolveAssetId(params),
        })
      ),
  };
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
    hawkeye: createPhoenixHawkeyeRpcClient({
      fetcher,
      exchange: config.exchange,
      pda: config.pda,
    }),
  };
};
