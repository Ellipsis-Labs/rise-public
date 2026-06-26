import { allocateIsolatedSubaccount } from "@/core/helpers";
import { address, type Address } from "@solana/kit";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock the trader fetch/decode; PDA derivation and the allocation logic itself
// run for real. `fetchTrader` drives the sequential fallback path,
// `decodeTrader` drives the batched (fetchAccounts) path.
vi.mock("@/accounts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/accounts")>();
  return { ...actual, fetchTrader: vi.fn(), decodeTrader: vi.fn() };
});

import { decodeTrader, fetchTrader } from "@/accounts";

const PHOENIX_PROGRAM = address(
  "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY"
) as Address;
const AUTHORITY = address("So11111111111111111111111111111111111111112");
const ASSET_ID = 7;

// Address -> existence, resolved after we learn the derived PDAs.
const existingByAddress = new Map<Address, boolean>();

const isolatedTrader = (opts: { hasAsset?: boolean; empty?: boolean }) => ({
  maxPositions: 1n,
  positions: {
    len: opts.empty ? 0n : 1n,
    entries: opts.hasAsset
      ? [{ key: BigInt(ASSET_ID), value: {} }]
      : opts.empty
        ? []
        : [{ key: BigInt(ASSET_ID + 1), value: {} }],
  },
});

const crossTrader = () => ({
  maxPositions: 8n,
  positions: { len: 1n, entries: [{ key: BigInt(ASSET_ID), value: {} }] },
});

beforeEach(() => {
  existingByAddress.clear();
  vi.mocked(fetchTrader).mockReset();
  vi.mocked(decodeTrader).mockReset();
});

type Decide = (index: number) => {
  exists: boolean;
  trader?: ReturnType<typeof isolatedTrader> | ReturnType<typeof crossTrader>;
};

/**
 * Drive the allocator with a per-index decision function, running BOTH the
 * batched (`fetchMaybeAccounts`) and sequential (`accountExists`/`fetchTrader`)
 * client shapes and asserting they agree. We can't know the derived PDAs up
 * front, so we re-derive them to map address -> index. The batched path encodes
 * the index in a one-byte `data` buffer that the mocked `decodeTrader` reads.
 */
const runAllocator = async (decide: Decide) => {
  const { getPhoenixTraderSubaccountAddress } = await import("@/pdas");
  const indexByAddress = new Map<Address, number>();
  for (let index = 1; index <= 100; index++) {
    const addr = await getPhoenixTraderSubaccountAddress({
      authority: AUTHORITY as never,
      traderPdaIndex: 0,
      subaccountIndex: index,
      phoenixProgramAddress: PHOENIX_PROGRAM,
    });
    indexByAddress.set(addr as Address, index);
    if (decide(index).exists) {
      existingByAddress.set(addr as Address, true);
    }
  }

  vi.mocked(fetchTrader).mockImplementation((async ({ address: addr }) => {
    const index = indexByAddress.get(addr as Address);
    const result = index !== undefined ? decide(index) : { exists: false };
    if (!result.exists || result.trader === undefined) {
      throw new Error("account not found");
    }
    return result.trader as never;
  }) as never);

  vi.mocked(decodeTrader).mockImplementation(((bytes: Uint8Array) => {
    const result = decide(bytes[0]);
    if (!result.exists || result.trader === undefined) {
      throw new Error("not a trader account");
    }
    return result.trader as never;
  }) as never);

  const sequentialClient = {
    addresses: { phoenixProgramAddress: PHOENIX_PROGRAM },
    accountExists: async (addr: Address) => existingByAddress.has(addr),
  } as never;

  const batchedClient = {
    addresses: { phoenixProgramAddress: PHOENIX_PROGRAM },
    accountExists: async (addr: Address) => existingByAddress.has(addr),
    fetchMaybeAccounts: async (addrs: readonly Address[]) =>
      addrs.map((addr) => {
        const index = indexByAddress.get(addr);
        if (index === undefined || !decide(index).exists) {
          return null;
        }
        return { data: Uint8Array.from([index]) };
      }),
  } as never;

  const [sequential, batched] = await Promise.all([
    allocateIsolatedSubaccount(
      sequentialClient,
      AUTHORITY as never,
      0,
      ASSET_ID
    ),
    allocateIsolatedSubaccount(batchedClient, AUTHORITY as never, 0, ASSET_ID),
  ]);

  // Both client shapes must reach the same allocation decision.
  expect(batched).toEqual(sequential);
  return sequential;
};

describe("allocateIsolatedSubaccount", () => {
  it("reuses an existing isolated subaccount holding the asset", async () => {
    const result = await runAllocator((index) =>
      index === 1
        ? { exists: true, trader: isolatedTrader({ hasAsset: true }) }
        : { exists: false }
    );
    expect(result.index).toBe(1);
    expect(result.needsRegistration).toBe(false);
  });

  it("prefers an asset match over an earlier empty subaccount", async () => {
    const result = await runAllocator((index) => {
      if (index === 1)
        return { exists: true, trader: isolatedTrader({ empty: true }) };
      if (index === 2)
        return { exists: true, trader: isolatedTrader({ hasAsset: true }) };
      return { exists: false };
    });
    // Empty index 1 is recorded but the asset match at 2 wins.
    expect(result.index).toBe(2);
    expect(result.needsRegistration).toBe(false);
  });

  it("reuses the first empty isolated subaccount when no asset match exists", async () => {
    const result = await runAllocator((index) =>
      index === 3
        ? { exists: true, trader: isolatedTrader({ empty: true }) }
        : index < 3
          ? { exists: true, trader: isolatedTrader({ hasAsset: false }) }
          : { exists: false }
    );
    expect(result.index).toBe(3);
    expect(result.needsRegistration).toBe(false);
  });

  it("skips cross (non-isolated) subaccounts", async () => {
    const result = await runAllocator((index) => {
      if (index === 1) return { exists: true, trader: crossTrader() };
      if (index === 2)
        return { exists: true, trader: isolatedTrader({ empty: true }) };
      return { exists: false };
    });
    expect(result.index).toBe(2);
    expect(result.needsRegistration).toBe(false);
  });

  it("returns the first unregistered slot when nothing is reusable", async () => {
    const result = await runAllocator(() => ({ exists: false }));
    expect(result.index).toBe(1);
    expect(result.needsRegistration).toBe(true);
  });

  it("prefers reusing an empty subaccount over registering a new one", async () => {
    // Index 1 is an unregistered gap; index 2 is an existing empty isolated
    // subaccount. Reuse (no registration) wins over registering at the gap.
    const result = await runAllocator((index) => {
      if (index === 1) return { exists: false };
      if (index === 2)
        return { exists: true, trader: isolatedTrader({ empty: true }) };
      return { exists: false };
    });
    expect(result.index).toBe(2);
    expect(result.needsRegistration).toBe(false);
  });
});
