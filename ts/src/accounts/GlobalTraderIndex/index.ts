import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import type { TraderAddress } from "@/primitives/_addressTypes";
import {
  getTraderAddressDecoder,
  getTraderStateDecoder,
  type TraderState,
} from "../internal";
import {
  decodeMultiArenaTree,
  type MultiArenaAccountBuffers,
  type MultiArenaAccountHeader,
} from "../multiArena";

export interface GlobalTraderIndexEntry {
  /** Stable node pointer used by active trader buffer position IDs. */
  readonly nodeIndex: number;
  readonly trader: TraderAddress;
  readonly state: TraderState;
}

export interface GlobalTraderIndex {
  readonly header: MultiArenaAccountHeader;
  /** Entries in public-key byte order. Only active traders are in the GTI. */
  readonly entries: readonly GlobalTraderIndexEntry[];
}

/** Decode a header account followed by all registered overflow accounts. */
export function decodeGlobalTraderIndex(
  buffers: MultiArenaAccountBuffers
): GlobalTraderIndex {
  const { header, entries } = decodeMultiArenaTree(buffers, {
    name: "GlobalTraderIndex",
    discriminant: ACCOUNT_DISCRIMINANTS.GLOBAL_TRADER_INDEX_HEADER,
    arenaDiscriminant: ACCOUNT_DISCRIMINANTS.GLOBAL_TRADER_INDEX_ARENA_HEADER,
    nodeSize: 64,
  });
  const addressDecoder = getTraderAddressDecoder();
  const stateDecoder = getTraderStateDecoder();
  for (let index = 1; index < entries.length; index++) {
    const previous = entries[index - 1].data;
    const current = entries[index].data;
    let order = 0;
    for (let byte = 0; byte < 32 && order === 0; byte++) {
      order = current[byte] - previous[byte];
    }
    if (order <= 0)
      throw new Error("GlobalTraderIndex: unordered or duplicate trader keys");
  }
  return {
    header,
    entries: entries.map(({ nodeIndex, data }) => ({
      nodeIndex,
      trader: addressDecoder.read(data, 0)[0],
      state: stateDecoder.read(data, 32)[0],
    })),
  };
}
