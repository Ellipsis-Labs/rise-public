import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import {
  getU32Decoder,
  getU64Decoder,
  type ReadonlyUint8Array,
} from "@solana/kit";
import {
  getTraderPositionDecoder,
  getTraderPositionIdDecoder,
  NATIVE_SOL_ASSET_INDEX,
  type TraderLockedDeposit,
  type TraderPosition,
  type TraderPositionId,
  type TraderSpotCollateral,
} from "../internal";
import {
  decodeMultiArenaTree,
  type MultiArenaAccountBuffers,
  type MultiArenaAccountHeader,
} from "../multiArena";

export interface ActiveTraderPosition {
  readonly position: TraderPosition;
  readonly lowestAsk: number;
  readonly askOrdersHead: number | null;
  readonly highestBid: number;
  readonly bidOrdersHead: number | null;
  readonly totalNonReduceOnlyAskBaseLots: number;
  readonly totalReduceOnlyAskBaseLots: number;
  readonly totalNonReduceOnlyBidBaseLots: number;
  readonly totalReduceOnlyBidBaseLots: number;
}

export type ActiveTraderBufferValue =
  | { readonly kind: "position"; readonly value: ActiveTraderPosition }
  | { readonly kind: "spotCollateral"; readonly value: TraderSpotCollateral }
  | { readonly kind: "lockedDeposit"; readonly value: TraderLockedDeposit }
  | { readonly kind: "unknown"; readonly data: ReadonlyUint8Array };

export interface ActiveTraderBufferEntry {
  readonly nodeIndex: number;
  readonly id: TraderPositionId;
  readonly value: ActiveTraderBufferValue;
}

export interface ActiveTraderBuffer {
  readonly header: MultiArenaAccountHeader;
  /** Includes collateral and other extensions, which are not perp positions. */
  readonly entries: readonly ActiveTraderBufferEntry[];
}

function decodeValue(
  assetId: number,
  data: ReadonlyUint8Array
): ActiveTraderBufferValue {
  const u64 = (offset: number) => getU64Decoder().read(data, offset)[0];
  const u32 = (offset: number) => getU32Decoder().read(data, offset)[0];
  if (assetId < 0xff000000) {
    return {
      kind: "position",
      value: {
        position: getTraderPositionDecoder().read(data, 0)[0],
        lowestAsk: u32(32),
        askOrdersHead: u32(36) || null,
        highestBid: u32(40),
        bidOrdersHead: u32(44) || null,
        totalNonReduceOnlyAskBaseLots: u32(48),
        totalReduceOnlyAskBaseLots: u32(52),
        totalNonReduceOnlyBidBaseLots: u32(56),
        totalReduceOnlyBidBaseLots: u32(60),
      },
    };
  }
  if (
    BigInt(assetId) === NATIVE_SOL_ASSET_INDEX ||
    Math.floor(assetId / 0x10000) === 0xfffe
  ) {
    return {
      kind: "spotCollateral",
      value: {
        assetIndex: assetId,
        balance: u64(0),
        maxPerTraderBalanceOverride: u64(8) || null,
      },
    };
  }
  if (assetId === 0xffff0001) {
    return {
      kind: "lockedDeposit",
      value: {
        lockedQuoteLots: u64(0),
        consumedQuoteLots: u64(8),
        isIsolated: (u64(16) & 1n) !== 0n,
      },
    };
  }
  return { kind: "unknown", data: data.slice() };
}

/** Decode a header account followed by all registered overflow accounts. */
export function decodeActiveTraderBuffer(
  buffers: MultiArenaAccountBuffers
): ActiveTraderBuffer {
  const { header, entries } = decodeMultiArenaTree(buffers, {
    name: "ActiveTraderBuffer",
    discriminant: ACCOUNT_DISCRIMINANTS.ACTIVE_TRADER_BUFFER_HEADER,
    arenaDiscriminant: ACCOUNT_DISCRIMINANTS.ACTIVE_TRADER_BUFFER_ARENA_HEADER,
    nodeSize: 88,
  });
  let previousId: TraderPositionId | undefined;
  return {
    header,
    entries: entries.map(({ nodeIndex, data }) => {
      const id = getTraderPositionIdDecoder().read(data, 0)[0];
      if (id.traderId === null)
        throw new Error("ActiveTraderBuffer: null trader node pointer");
      if (
        previousId &&
        (id.traderId < (previousId.traderId ?? 0) ||
          (id.traderId === previousId.traderId &&
            id.assetId <= previousId.assetId))
      ) {
        throw new Error(
          "ActiveTraderBuffer: unordered or duplicate position IDs"
        );
      }
      previousId = id;
      return {
        nodeIndex,
        id,
        value: decodeValue(id.assetId, data.subarray(8)),
      };
    }),
  };
}
