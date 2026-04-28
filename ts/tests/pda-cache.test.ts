import { beforeEach, describe, expect, it, vi } from "vitest";

const mocked = vi.hoisted(() => ({
  getPhoenixGlobalConfigurationAddress: vi.fn(),
  getPhoenixTraderSubaccountAddress: vi.fn(),
}));

vi.mock("@/pdas", async () => {
  const actual = await vi.importActual<typeof import("@/pdas")>("@/pdas");
  return {
    ...actual,
    getPhoenixGlobalConfigurationAddress:
      mocked.getPhoenixGlobalConfigurationAddress,
    getPhoenixTraderSubaccountAddress: mocked.getPhoenixTraderSubaccountAddress,
  };
});

import { PhoenixHttpClient } from "@/client";
import { createPhoenixPdaClient } from "@/pdaClient";
import type {
  Authority,
  GlobalConfigurationAddress,
  TraderAddress,
} from "@/primitives";
import { address } from "@solana/kit";

const TEST_AUTHORITY = address("11111111111111111111111111111112") as Authority;
const TEST_GLOBAL_CONFIGURATION = address(
  "11111111111111111111111111111111"
) as GlobalConfigurationAddress;
const TEST_TRADER_ADDRESS = address(
  "11111111111111111111111111111113"
) as TraderAddress;

describe("pda cache", () => {
  beforeEach(() => {
    mocked.getPhoenixGlobalConfigurationAddress.mockReset();
    mocked.getPhoenixGlobalConfigurationAddress.mockResolvedValue(
      TEST_GLOBAL_CONFIGURATION
    );

    mocked.getPhoenixTraderSubaccountAddress.mockReset();
    mocked.getPhoenixTraderSubaccountAddress.mockResolvedValue(
      TEST_TRADER_ADDRESS
    );
  });

  it("memoizes repeated PDA derivations by default", async () => {
    const pda = createPhoenixPdaClient();

    const [first, second] = await Promise.all([
      pda.getGlobalConfigurationAddress(),
      pda.getGlobalConfigurationAddress(),
    ]);

    expect(first).toBe(TEST_GLOBAL_CONFIGURATION);
    expect(second).toBe(TEST_GLOBAL_CONFIGURATION);
    expect(mocked.getPhoenixGlobalConfigurationAddress).toHaveBeenCalledTimes(
      1
    );
  });

  it("defaults to a 1024-entry LRU cache", async () => {
    const pda = createPhoenixPdaClient();

    for (let traderPdaIndex = 0; traderPdaIndex <= 1_024; traderPdaIndex += 1) {
      await pda.getTraderAddress({
        authority: TEST_AUTHORITY,
        traderPdaIndex,
        subaccountIndex: 0,
      });
    }

    await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });

    expect(mocked.getPhoenixTraderSubaccountAddress).toHaveBeenCalledTimes(
      1_026
    );
  });

  it("keeps keyed trader lookups isolated per input", async () => {
    const pda = createPhoenixPdaClient();

    await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });
    await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });
    await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 1,
      subaccountIndex: 0,
    });

    expect(mocked.getPhoenixTraderSubaccountAddress).toHaveBeenCalledTimes(2);
  });

  it("can disable memoization on standalone pda clients", async () => {
    const pda = createPhoenixPdaClient({ cache: false });

    await pda.getGlobalConfigurationAddress();
    await pda.getGlobalConfigurationAddress();

    expect(mocked.getPhoenixGlobalConfigurationAddress).toHaveBeenCalledTimes(
      2
    );
  });

  it("evicts least recently used entries when maxEntries is reached", async () => {
    const pda = createPhoenixPdaClient({
      cache: {
        maxEntries: 1,
      },
    });

    await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });
    await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 1,
      subaccountIndex: 0,
    });
    await pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });

    expect(mocked.getPhoenixTraderSubaccountAddress).toHaveBeenCalledTimes(3);
  });

  it("can clear cached PDA derivations explicitly", async () => {
    const pda = createPhoenixPdaClient();

    await pda.getGlobalConfigurationAddress();
    await pda.getGlobalConfigurationAddress();
    pda.clearCache();
    await pda.getGlobalConfigurationAddress();

    expect(mocked.getPhoenixGlobalConfigurationAddress).toHaveBeenCalledTimes(
      2
    );
  });

  it("threads pdaCache through PhoenixHttpClient", async () => {
    const client = new PhoenixHttpClient({
      baseUrl: "http://localhost:8080",
      pdaCache: false,
    });

    await client.pda.getGlobalConfigurationAddress();
    await client.pda.getGlobalConfigurationAddress();

    expect(mocked.getPhoenixGlobalConfigurationAddress).toHaveBeenCalledTimes(
      2
    );
  });

  it("threads bounded pdaCache config through PhoenixHttpClient", async () => {
    const client = new PhoenixHttpClient({
      baseUrl: "http://localhost:8080",
      pdaCache: {
        maxEntries: 1,
      },
    });

    await client.pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });
    await client.pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 1,
      subaccountIndex: 0,
    });
    await client.pda.getTraderAddress({
      authority: TEST_AUTHORITY,
      traderPdaIndex: 0,
      subaccountIndex: 0,
    });

    expect(mocked.getPhoenixTraderSubaccountAddress).toHaveBeenCalledTimes(3);
  });
});
