import { address } from "@solana/kit";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PhoenixRpcAccountFetcherClient } from "@/rpc";

const ADDRESS = address("11111111111111111111111111111112");

type RpcCall = { method: string; params: unknown[] };

const jsonResponse = (result: unknown) =>
  new Response(JSON.stringify({ jsonrpc: "2.0", id: 1, result }), {
    status: 200,
    headers: { "content-type": "application/json" },
  });

const stubRpc = (respond: (call: RpcCall) => unknown): RpcCall[] => {
  const calls: RpcCall[] = [];
  const fetchMock = vi.fn(async (_url: unknown, init?: RequestInit) => {
    const body = JSON.parse(String(init?.body)) as RpcCall;
    calls.push(body);
    return jsonResponse(respond(body));
  });
  vi.stubGlobal("fetch", fetchMock);
  return calls;
};

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("fetchLamportAccountState", () => {
  it("returns balance and the rent floor for the account's own size", async () => {
    const calls = stubRpc(({ method }) => {
      if (method === "getAccountInfo") {
        return {
          context: { slot: 1 },
          value: {
            data: ["", "base64"],
            lamports: 1_500_000_000,
            space: 4_992,
          },
        };
      }
      if (method === "getMinimumBalanceForRentExemption") {
        return 35_640_240;
      }
      throw new Error(`unexpected method ${method}`);
    });

    const client = new PhoenixRpcAccountFetcherClient("http://localhost:8899");
    const state = await client.fetchLamportAccountState(ADDRESS);

    expect(state).toEqual({
      exists: true,
      balanceLamports: 1_500_000_000n,
      rentExemptMinimumLamports: 35_640_240n,
    });
    // The rent query must use the account's actual data length.
    expect(calls[1]?.method).toBe("getMinimumBalanceForRentExemption");
    expect(calls[1]?.params[0]).toBe(4_992);
    // Only lamports and space are read, so the data payload is sliced away.
    expect(calls[0]?.params[1]).toMatchObject({
      dataSlice: { offset: 0, length: 0 },
    });
  });

  it("re-queries unsliced when the RPC omits space, using the returned space", async () => {
    const calls = stubRpc((call) => {
      if (call.method === "getAccountInfo") {
        const options = call.params[1] as { dataSlice?: unknown };
        return options.dataSlice
          ? // First, sliced call: this RPC does not report space.
            {
              context: { slot: 1 },
              value: { data: ["", "base64"], lamports: 9 },
            }
          : {
              context: { slot: 1 },
              value: { data: ["", "base64"], space: 64 },
            };
      }
      return 1_141_440;
    });

    const client = new PhoenixRpcAccountFetcherClient("http://localhost:8899");
    const state = await client.fetchLamportAccountState(ADDRESS);

    expect(state).toEqual({
      exists: true,
      balanceLamports: 9n,
      rentExemptMinimumLamports: 1_141_440n,
    });
    // The retry drops dataSlice so the size is recoverable.
    expect(calls[1]?.method).toBe("getAccountInfo");
    expect(calls[1]?.params[1]).not.toHaveProperty("dataSlice");
    expect(calls[2]?.params[0]).toBe(64);
  });

  it("falls back to the decoded payload length when space is absent entirely", async () => {
    // 8 bytes of data, base64 "AAAAAAAAAAA=" -> space must come from the data.
    const eightZeroBytes = Buffer.alloc(8).toString("base64");
    const calls = stubRpc((call) => {
      if (call.method === "getAccountInfo") {
        const options = call.params[1] as { dataSlice?: unknown };
        return options.dataSlice
          ? {
              context: { slot: 1 },
              value: { data: ["", "base64"], lamports: 3 },
            }
          : {
              context: { slot: 1 },
              value: { data: [eightZeroBytes, "base64"] },
            };
      }
      return 946_560;
    });

    const client = new PhoenixRpcAccountFetcherClient("http://localhost:8899");
    const state = await client.fetchLamportAccountState(ADDRESS);

    expect(state.rentExemptMinimumLamports).toBe(946_560n);
    expect(calls[2]?.params[0]).toBe(8);
  });

  it("memoizes the rent minimum per account size", async () => {
    const calls = stubRpc(({ method }) =>
      method === "getAccountInfo"
        ? {
            context: { slot: 1 },
            value: { data: ["", "base64"], lamports: 5, space: 128 },
          }
        : 1_781_760
    );

    const client = new PhoenixRpcAccountFetcherClient("http://localhost:8899");
    await client.fetchLamportAccountState(ADDRESS);
    await client.fetchLamportAccountState(ADDRESS);

    const rentCalls = calls.filter(
      (call) => call.method === "getMinimumBalanceForRentExemption"
    );
    expect(rentCalls).toHaveLength(1);
  });

  it("reports a missing account as zeros without a rent query", async () => {
    const calls = stubRpc(({ method }) => {
      if (method === "getAccountInfo") {
        return { context: { slot: 1 }, value: null };
      }
      throw new Error(`unexpected method ${method}`);
    });

    const client = new PhoenixRpcAccountFetcherClient("http://localhost:8899");
    const state = await client.fetchLamportAccountState(ADDRESS);

    expect(state).toEqual({
      exists: false,
      balanceLamports: 0n,
      rentExemptMinimumLamports: 0n,
    });
    expect(calls).toHaveLength(1);
  });

  it("tolerates string-encoded numeric fields", async () => {
    stubRpc(({ method }) =>
      method === "getAccountInfo"
        ? {
            context: { slot: 1 },
            value: { data: ["", "base64"], lamports: "7", space: "0" },
          }
        : "890880"
    );

    const client = new PhoenixRpcAccountFetcherClient("http://localhost:8899");
    const state = await client.fetchLamportAccountState(ADDRESS);

    expect(state).toEqual({
      exists: true,
      balanceLamports: 7n,
      rentExemptMinimumLamports: 890_880n,
    });
  });
});
