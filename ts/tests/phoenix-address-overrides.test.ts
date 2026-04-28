import {
  PHOENIX_GLOBAL_CONFIGURATION_ADDRESS,
  PHOENIX_LOG_AUTHORITY_ADDRESS,
  PHOENIX_PROGRAM_ADDRESS,
  buildPlaceMarketOrderIx,
  getPhoenixInstructionAddresses,
  type PlaceMarketOrderParams,
  baseLots,
  quoteLots,
  resolvePhoenixInstructionAddresses,
  ticks,
  SelfTradeBehavior,
  Side,
} from "@/index";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalConfigurationAddress,
  GlobalTraderIndexAddressArray,
  LogAuthorityAddress,
  MarketAddress,
  PerpAssetMapAddress,
  PhoenixProgramAddress,
  SplineCollectionAddress,
  TraderAddress,
} from "@/primitives";
import { address } from "@solana/kit";
import { afterEach, describe, expect, it } from "vitest";

const BETA_PROGRAM_ADDRESS = address(
  "phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf"
) as PhoenixProgramAddress;
const BETA_LOG_AUTHORITY_ADDRESS = address(
  "8Q1zeC7qAPUhJ2ncHAW8N1TGcpMVDsSxdSBPLaE8G2ab"
) as LogAuthorityAddress;
const BETA_GLOBAL_CONFIGURATION_ADDRESS = address(
  "3CkM38UaZW6nyTJku4ABE5jjS5AComQErrkd55LGTfxa"
) as GlobalConfigurationAddress;

afterEach(() => {
  delete process.env.PHOENIX_ENV;
  delete process.env.NEXT_PUBLIC_PHOENIX_ENV;
});

const createPlaceMarketOrderParams = (
  overrides: Partial<PlaceMarketOrderParams> = {}
): PlaceMarketOrderParams => {
  const activeTraderBuffer = [
    address("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"),
    address("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
  ] as ActiveTraderBufferAddressArray;
  const globalTraderIndex = [
    address("SysvarRent111111111111111111111111111111111"),
    address("SysvarC1ock11111111111111111111111111111111"),
  ] as GlobalTraderIndexAddressArray;

  return {
    trader: address("Vote111111111111111111111111111111111111111") as Authority,
    traderAccount: address(
      "Stake11111111111111111111111111111111111111"
    ) as TraderAddress,
    perpAssetMap: address(
      "SysvarFees111111111111111111111111111111111"
    ) as PerpAssetMapAddress,
    orderbook: address(
      "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"
    ) as MarketAddress,
    splineCollection: address(
      "SysvarRecentB1ockHashes11111111111111111111"
    ) as SplineCollectionAddress,
    activeTraderBuffer,
    globalTraderIndex,
    orderPacket: {
      side: Side.Bid,
      priceInTicks: ticks(10n),
      numBaseLots: baseLots(1n),
      numQuoteLots: quoteLots(10n),
      minBaseLotsToFill: baseLots(1n),
      minQuoteLotsToFill: quoteLots(1n),
      selfTradeBehavior: SelfTradeBehavior.Abort,
      matchLimit: null,
      clientOrderId: 0n,
      lastValidSlot: null,
      orderFlags: 0,
      cancelExisting: false,
    },
    ...overrides,
  };
};

describe("phoenix instruction address overrides", () => {
  it("uses explicit override addresses instead of default prod constants", () => {
    const programAddress = BETA_PROGRAM_ADDRESS;
    const logAuthorityAddress = BETA_LOG_AUTHORITY_ADDRESS;
    const globalConfigurationAddress = BETA_GLOBAL_CONFIGURATION_ADDRESS;

    const ix = buildPlaceMarketOrderIx(
      createPlaceMarketOrderParams({
        programAddress,
        logAuthorityAddress,
        globalConfigurationAddress,
      })
    );

    expect(ix.programAddress).toBe(programAddress);
    expect(ix.accounts[0]?.address).toBe(programAddress);
    expect(ix.accounts[1]?.address).toBe(logAuthorityAddress);
    expect(ix.accounts[2]?.address).toBe(globalConfigurationAddress);
  });

  it("resolves prod defaults before env changes and beta defaults after env changes", () => {
    const prodAddresses = getPhoenixInstructionAddresses();

    process.env.PHOENIX_ENV = "beta";

    const betaAddresses = getPhoenixInstructionAddresses();

    expect(prodAddresses.programAddress).toBe(PHOENIX_PROGRAM_ADDRESS);
    expect(prodAddresses.logAuthorityAddress).toBe(
      PHOENIX_LOG_AUTHORITY_ADDRESS
    );
    expect(prodAddresses.globalConfigurationAddress).toBe(
      PHOENIX_GLOBAL_CONFIGURATION_ADDRESS
    );

    expect(betaAddresses.programAddress).toBe(BETA_PROGRAM_ADDRESS);
    expect(betaAddresses.logAuthorityAddress).toBe(BETA_LOG_AUTHORITY_ADDRESS);
    expect(betaAddresses.globalConfigurationAddress).toBe(
      BETA_GLOBAL_CONFIGURATION_ADDRESS
    );
  });

  it("applies beta defaults to builder output at call time", () => {
    process.env.PHOENIX_ENV = "beta";

    const ix = buildPlaceMarketOrderIx(createPlaceMarketOrderParams());

    expect(ix.programAddress).toBe(BETA_PROGRAM_ADDRESS);
    expect(ix.accounts[0]?.address).toBe(BETA_PROGRAM_ADDRESS);
    expect(ix.accounts[1]?.address).toBe(BETA_LOG_AUTHORITY_ADDRESS);
    expect(ix.accounts[2]?.address).toBe(BETA_GLOBAL_CONFIGURATION_ADDRESS);
  });

  it("prefers NEXT_PUBLIC_PHOENIX_ENV over legacy PHOENIX_ENV", () => {
    process.env.PHOENIX_ENV = "prod";
    process.env.NEXT_PUBLIC_PHOENIX_ENV = "beta";

    const addresses = getPhoenixInstructionAddresses();

    expect(addresses.programAddress).toBe(BETA_PROGRAM_ADDRESS);
    expect(addresses.logAuthorityAddress).toBe(BETA_LOG_AUTHORITY_ADDRESS);
    expect(addresses.globalConfigurationAddress).toBe(
      BETA_GLOBAL_CONFIGURATION_ADDRESS
    );
  });

  it("keeps explicit overrides ahead of beta env defaults", () => {
    process.env.PHOENIX_ENV = "beta";

    const programAddress = address(
      "11111111111111111111111111111112"
    ) as PhoenixProgramAddress;
    const logAuthorityAddress = address(
      "11111111111111111111111111111113"
    ) as LogAuthorityAddress;
    const globalConfigurationAddress = address(
      "11111111111111111111111111111114"
    ) as GlobalConfigurationAddress;

    const ix = buildPlaceMarketOrderIx(
      createPlaceMarketOrderParams({
        programAddress,
        logAuthorityAddress,
        globalConfigurationAddress,
      })
    );

    expect(ix.programAddress).toBe(programAddress);
    expect(ix.accounts[0]?.address).toBe(programAddress);
    expect(ix.accounts[1]?.address).toBe(logAuthorityAddress);
    expect(ix.accounts[2]?.address).toBe(globalConfigurationAddress);
  });

  it("uses beta companion addresses when only the known beta program id is overridden", () => {
    const addresses = resolvePhoenixInstructionAddresses({
      programAddress: BETA_PROGRAM_ADDRESS,
    });

    expect(addresses.programAddress).toBe(BETA_PROGRAM_ADDRESS);
    expect(addresses.logAuthorityAddress).toBe(BETA_LOG_AUTHORITY_ADDRESS);
    expect(addresses.globalConfigurationAddress).toBe(
      BETA_GLOBAL_CONFIGURATION_ADDRESS
    );
  });
});
