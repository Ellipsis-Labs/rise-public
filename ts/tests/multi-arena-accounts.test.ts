import { getAddressDecoder } from "@solana/kit";
import { describe, expect, it } from "vitest";
import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import { decodeGlobalTraderIndex } from "@/accounts/GlobalTraderIndex";
import { decodeActiveTraderBuffer } from "@/accounts/ActiveTraderBuffer";

const view = (bytes: Uint8Array) =>
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

function accountBuffers(
  kind: "GLOBAL_TRADER_INDEX" | "ACTIVE_TRADER_BUFFER",
  size: number
) {
  const nodeSize = kind === "GLOBAL_TRADER_INDEX" ? 64 : 88;
  const first = new Uint8Array(96 + nodeSize);
  const overflow = new Uint8Array(32 + nodeSize * (size - 1));
  first.set(ACCOUNT_DISCRIMINANTS[`${kind}_HEADER`]);
  overflow.set(ACCOUNT_DISCRIMINANTS[`${kind}_ARENA_HEADER`]);
  view(first).setUint32(48, size, true);
  view(first).setUint16(52, 2, true);
  view(first).setUint16(54, 2, true);
  // One node per arena for the two-node GTI fixture; ATB uses a single arena below.
  view(first).setUint32(56, 1, true);
  view(first).setUint32(60, size + 1, true);
  view(first).setUint32(80, 1, true);
  view(overflow).setUint16(8, 1, true);
  view(first).setUint32(100, 2, true);
  view(overflow).setUint32(40, 1, true);
  return [first, overflow];
}

function gtiBuffers() {
  const buffers = accountBuffers("GLOBAL_TRADER_INDEX", 2);
  buffers[0].fill(1, 112, 144);
  buffers[1].fill(2, 48, 80);
  view(buffers[0]).setBigInt64(144, -9_007_199_254_740_993n, true);
  view(buffers[1]).setBigInt64(80, 100_000n, true);
  return buffers;
}

function atbBytes() {
  const bytes = new Uint8Array(96 + 88 * 5);
  bytes.set(ACCOUNT_DISCRIMINANTS.ACTIVE_TRADER_BUFFER_HEADER);
  const data = view(bytes);
  data.setUint32(48, 5, true);
  data.setUint16(52, 1, true);
  data.setUint16(54, 1, true);
  data.setUint32(56, 5, true);
  data.setUint32(60, 6, true);
  data.setUint32(80, 1, true);
  // Perp, SPL spot, SOL, lock, future extension: all belong to GTI node 2.
  const assets = [7, 0xfffe0002, 0xffff0000, 0xffff0001, 0xffff0002];
  for (let i = 0; i < assets.length; i++) {
    const offset = 96 + i * 88;
    data.setUint32(offset + 4, i === 4 ? 0 : i + 2, true);
    data.setUint32(offset + 8, i, true);
    data.setUint32(offset + 16, 2, true);
    data.setUint32(offset + 20, assets[i], true);
  }
  data.setBigInt64(120, -12n, true);
  data.setBigInt64(128, 50_000n, true);
  data.setUint32(152, 88, true);
  data.setUint32(156, 123, true);
  data.setUint32(168, 17, true);
  data.setBigUint64(208, 42n, true);
  data.setBigUint64(216, 100n, true);
  data.setBigUint64(296, 2_000_000_000n, true);
  data.setBigUint64(384, 1_000_000n, true);
  data.setBigUint64(392, 100_000n, true);
  data.setBigUint64(400, 1n, true);
  return bytes;
}

describe("multi-arena trader account readers", () => {
  it("joins GTI node IDs across overflow arenas without losing signed collateral precision", () => {
    const buffers = gtiBuffers();
    // Byte offsets need not have Rust struct alignment.
    const unaligned = buffers.map((bytes) => {
      const padded = new Uint8Array(bytes.length + 1);
      padded.set(bytes, 1);
      return padded.subarray(1);
    });
    const result = decodeGlobalTraderIndex(unaligned);
    expect(result.entries.map((entry) => entry.nodeIndex)).toEqual([1, 2]);
    expect(result.entries[0].state.quoteLotCollateral).toBe(
      -9_007_199_254_740_993n
    );
    expect(result.entries[1].trader).toBe(
      getAddressDecoder().decode(new Uint8Array(32).fill(2))
    );
    expect(result.entries[1].state.quoteLotCollateral).toBe(100_000n);
  });

  it("reads resized arenas using actual buffers when the stored active count lags", () => {
    const buffers = gtiBuffers();
    view(buffers[0]).setUint16(54, 1, true);
    expect(decodeGlobalTraderIndex(buffers).entries).toHaveLength(2);
    const emptyTail = new Uint8Array(32);
    emptyTail.set(ACCOUNT_DISCRIMINANTS.GLOBAL_TRADER_INDEX_ARENA_HEADER);
    view(emptyTail).setUint16(8, 2, true);
    view(buffers[0]).setUint16(52, 3, true);
    expect(
      decodeGlobalTraderIndex([...buffers, emptyTail]).entries
    ).toHaveLength(2);
  });

  it("keeps positions, collateral, locked deposits, and future extensions distinct", () => {
    const entries = decodeActiveTraderBuffer([atbBytes()]).entries;
    expect(entries.map((entry) => entry.id.traderId)).toEqual([2, 2, 2, 2, 2]);
    const position = entries[0].value;
    expect(position.kind).toBe("position");
    if (position.kind !== "position") throw new Error("expected position");
    expect(position.value.position.baseLotPosition).toBe(-12n);
    expect(position.value.lowestAsk).toBe(88);
    expect(position.value.askOrdersHead).toBe(123);
    expect(position.value.totalNonReduceOnlyAskBaseLots).toBe(17);
    expect(entries[1].value).toEqual({
      kind: "spotCollateral",
      value: {
        assetIndex: 0xfffe0002,
        balance: 42n,
        maxPerTraderBalanceOverride: 100n,
      },
    });
    expect(entries[2].value).toEqual({
      kind: "spotCollateral",
      value: {
        assetIndex: 0xffff0000,
        balance: 2_000_000_000n,
        maxPerTraderBalanceOverride: null,
      },
    });
    expect(entries[3].value).toEqual({
      kind: "lockedDeposit",
      value: {
        lockedQuoteLots: 1_000_000n,
        consumedQuoteLots: 100_000n,
        isIsolated: true,
      },
    });
    expect(entries[4].value.kind).toBe("unknown");
  });

  it.each([
    [
      "cycle",
      (buffers: Uint8Array[]) => view(buffers[1]).setUint32(36, 1, true),
    ],
    [
      "out of range pointer",
      (buffers: Uint8Array[]) => view(buffers[0]).setUint32(100, 3, true),
    ],
    [
      "free node",
      (buffers: Uint8Array[]) =>
        view(buffers[1]).setUint32(36, 0xffffffff, true),
    ],
    [
      "wrong arena index",
      (buffers: Uint8Array[]) => view(buffers[1]).setUint16(8, 2, true),
    ],
    [
      "wrong discriminator",
      (buffers: Uint8Array[]) => {
        buffers[1][0] ^= 1;
      },
    ],
    [
      "missing arena",
      (buffers: Uint8Array[]) => {
        buffers.pop();
      },
    ],
    [
      "truncated arena",
      (buffers: Uint8Array[]) => {
        buffers[1] = buffers[1].subarray(0, 95);
      },
    ],
    [
      "duplicate key",
      (buffers: Uint8Array[]) => {
        buffers[1].fill(1, 48, 80);
      },
    ],
    [
      "wrong parent",
      (buffers: Uint8Array[]) => view(buffers[1]).setUint32(40, 0, true),
    ],
  ])(
    "rejects %s instead of returning partial trader state",
    (_name, corrupt) => {
      const buffers = gtiBuffers();
      corrupt(buffers);
      expect(() => decodeGlobalTraderIndex(buffers)).toThrow();
    }
  );

  it("rejects unordered ATB IDs and null GTI references", () => {
    const bytes = atbBytes();
    view(bytes).setUint32(200, 0, true);
    expect(() => decodeActiveTraderBuffer([bytes])).toThrow();
    view(bytes).setUint32(200, 1, true);
    expect(() => decodeActiveTraderBuffer([bytes])).toThrow();
  });
});
