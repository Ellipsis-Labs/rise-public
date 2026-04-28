import {
  ACCOUNT_DISCRIMINANTS,
  AccountFetchers,
  fetchConditionalOrderCollection,
  fetchGlobalConfiguration,
  fetchMint,
  getMintEncoder,
  type AccountFetcherClientWithAddresses,
  type Mint,
  flight,
} from "@/index";
import type { Authority, PhoenixProgramAddress } from "@/primitives";
import { address, getAddressDecoder, type Address } from "@solana/kit";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it, vi } from "vitest";

const TESTS_DIR = dirname(fileURLToPath(import.meta.url));
const MOCKS_DIR = resolve(TESTS_DIR, "mocks");

type FixtureFile = {
  account: {
    data: [string, string];
  };
};

const addressFromFill = (fill: number): Address =>
  getAddressDecoder().decode(new Uint8Array(32).fill(fill));

const writeU64LE = (bytes: Uint8Array, offset: number, value: bigint): void => {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  view.setBigUint64(offset, value, true);
};

const createConditionalOrderCollectionBytes = (): Uint8Array => {
  const headerSize = 96;
  const orderSize = 112;
  const bytes = new Uint8Array(headerSize + 2 * orderSize);
  bytes.set(ACCOUNT_DISCRIMINANTS.CONDITIONAL_ORDER_COLLECTION, 0);
  bytes[88] = 1;
  bytes[89] = 2;
  const orderOffset = headerSize + orderSize;
  writeU64LE(bytes, orderOffset + 64, 101n);
  bytes[orderOffset + 64 + 17] = 0x01;
  return bytes;
};

const loadMockBytes = (fileName: string): Uint8Array => {
  const fixture = JSON.parse(
    readFileSync(resolve(MOCKS_DIR, fileName), "utf8")
  ) as FixtureFile;

  return Buffer.from(fixture.account.data[0], "base64");
};

const BETA_PHOENIX_PROGRAM_ADDRESS = address(
  "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
) as PhoenixProgramAddress;
const BUILDER_AUTHORITY = address(
  "F952dz4aHVUu75YdxUrGhLejhCfzXCYVDysDw6yL6uT4"
) as Authority;

describe("account fetchers", () => {
  it("caches mint fetches and respects skipCache", async () => {
    const mint: Mint = {
      mintAuthorityOption: 1,
      mintAuthority: addressFromFill(1),
      supply: 123n,
      decimals: 6,
      isInitialized: true,
      freezeAuthorityOption: 1,
      freezeAuthority: addressFromFill(2),
    };
    const mintBytes = getMintEncoder().encode(mint);
    const address = addressFromFill(3);
    const client = {
      fetchAccount: vi.fn(async () => ({ data: mintBytes })),
      _accountCache: new Map<string, unknown>(),
      _cacheEnabled: true,
    };

    const first = await fetchMint({ client, address });
    const second = await fetchMint({ client, address });
    const third = await fetchMint({ client, address, skipCache: true });

    expect(first).toEqual(mint);
    expect(second).toBe(first);
    expect(third).toEqual(mint);
    expect(client.fetchAccount).toHaveBeenCalledTimes(2);
    expect(AccountFetchers.Mint).toBe(fetchMint);
  });

  it("uses the client's default global configuration address", async () => {
    const globalConfigurationAddress = addressFromFill(9);
    const client: AccountFetcherClientWithAddresses = {
      fetchAccount: vi.fn(async () => ({
        data: loadMockBytes("global_config.json"),
      })),
      _accountCache: new Map<string, unknown>(),
      _cacheEnabled: true,
      addresses: {
        globalConfigurationAddress,
      },
    };

    const configuration = await fetchGlobalConfiguration({ client });

    expect(client.fetchAccount).toHaveBeenCalledWith(
      globalConfigurationAddress
    );
    expect(configuration.accountKey).toBeTypeOf("string");
    expect(AccountFetchers.GlobalConfiguration).toBe(fetchGlobalConfiguration);
  });

  it("fetches ConditionalOrderCollection accounts", async () => {
    const conditionalOrdersAddress = addressFromFill(4);
    const client = {
      fetchAccount: vi.fn(async () => ({
        data: createConditionalOrderCollectionBytes(),
      })),
      _accountCache: new Map<string, unknown>(),
      _cacheEnabled: true,
    };

    const collection = await fetchConditionalOrderCollection({
      client,
      address: conditionalOrdersAddress,
    });

    expect(client.fetchAccount).toHaveBeenCalledWith(conditionalOrdersAddress);
    expect(collection.activeOrderIndices).toEqual([1]);
    expect(AccountFetchers.ConditionalOrderCollection).toBe(
      fetchConditionalOrderCollection
    );
  });

  it("fetches Flight GlobalState using the derived PDA by default", async () => {
    const client = {
      fetchAccount: vi.fn(async () => ({
        data: loadMockBytes("flight_global_state.json"),
      })),
      _accountCache: new Map<string, unknown>(),
      _cacheEnabled: true,
    };
    const globalStateAddress = await flight.getFlightGlobalStateAddress(
      BETA_PHOENIX_PROGRAM_ADDRESS
    );

    const globalState = await flight.fetchGlobalState({
      client,
      phoenixProgramAddress: BETA_PHOENIX_PROGRAM_ADDRESS,
    });

    expect(client.fetchAccount).toHaveBeenCalledWith(globalStateAddress);
    expect(globalState.maxFeeCapBps).toBe(10n);
    expect(globalState.programId).toBe(BETA_PHOENIX_PROGRAM_ADDRESS);
  });

  it("fetches Flight BuilderState using the authority-derived PDA", async () => {
    const client = {
      fetchAccount: vi.fn(async () => ({
        data: loadMockBytes("flight_builder_state.json"),
      })),
      _accountCache: new Map<string, unknown>(),
      _cacheEnabled: true,
    };
    const builderStateAddress = await flight.getFlightBuilderStateAddress(
      BUILDER_AUTHORITY,
      BETA_PHOENIX_PROGRAM_ADDRESS
    );

    const builderState = await flight.fetchBuilderState({
      client,
      authority: BUILDER_AUTHORITY,
      phoenixProgramAddress: BETA_PHOENIX_PROGRAM_ADDRESS,
    });

    expect(client.fetchAccount).toHaveBeenCalledWith(builderStateAddress);
    expect(builderState.authorityKey).toBe(BUILDER_AUTHORITY);
    expect(builderState.feeBps).toBe(8n);
    expect(builderState.isActive).toBe(true);
  });
});
