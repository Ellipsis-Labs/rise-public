import type { ReadonlyUint8Array } from "@solana/kit";
import { getSequenceNumberDecoder, type SequenceNumber } from "./internal";

export interface MultiArenaAccountHeader {
  readonly sequenceNumber: SequenceNumber;
  readonly numAdditionalNodes: number;
  readonly size: number;
  /** Includes the header account, which contains arena zero. */
  readonly numArenas: number;
  readonly numActiveArenas: number;
  readonly numNodesPerArena: number;
}

interface TreeEntry {
  readonly nodeIndex: number;
  readonly data: ReadonlyUint8Array;
}

/** Account buffers include their account headers and are ordered by arena index. */
export type MultiArenaAccountBuffers = readonly ReadonlyUint8Array[];

export function decodeMultiArenaTree(
  buffers: MultiArenaAccountBuffers,
  options: {
    name: string;
    discriminant: ReadonlyUint8Array;
    arenaDiscriminant: ReadonlyUint8Array;
    nodeSize: number;
  }
): { header: MultiArenaAccountHeader; entries: readonly TreeEntry[] } {
  const invalid = (reason: string): never => {
    throw new Error(`${options.name}: ${reason}`);
  };
  const first = buffers[0];
  if (!first || first.length < 96) {
    return invalid("header account must contain at least 96 bytes");
  }
  const view = (bytes: ReadonlyUint8Array): DataView =>
    new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const matches = (bytes: ReadonlyUint8Array, expected: ReadonlyUint8Array) =>
    expected.every((byte, i) => bytes[i] === byte);
  if (!matches(first, options.discriminant)) {
    return invalid("incorrect header account discriminant");
  }
  const firstView = view(first);
  const header: MultiArenaAccountHeader = {
    sequenceNumber: getSequenceNumberDecoder().read(first, 8)[0],
    numAdditionalNodes: firstView.getUint32(24, true),
    size: firstView.getUint32(48, true),
    numArenas: firstView.getUint16(52, true),
    numActiveArenas: firstView.getUint16(54, true),
    numNodesPerArena: firstView.getUint32(56, true),
  };
  const bumpIndex = firstView.getUint32(60, true);
  const root = firstView.getUint32(80, true);
  if (
    header.numArenas !== buffers.length ||
    header.numActiveArenas > header.numArenas ||
    header.numNodesPerArena === 0 ||
    bumpIndex === 0
  ) {
    return invalid("invalid arena count or allocator bounds");
  }
  const arenas = buffers.map((bytes, index) => {
    const prefix = index === 0 ? 96 : 32;
    if (bytes.length < prefix) return invalid(`arena ${index} is truncated`);
    if (index > 0) {
      if (!matches(bytes, options.arenaDiscriminant)) {
        return invalid(`arena ${index} has an incorrect discriminant`);
      }
      if (view(bytes).getUint16(8, true) !== index) {
        return invalid(`arena ${index} is out of order`);
      }
    }
    const arena = bytes.subarray(prefix);
    if (arena.length % options.nodeSize !== 0) {
      return invalid(`arena ${index} contains a partial node`);
    }
    const capacity = arena.length / options.nodeSize;
    if (capacity > header.numNodesPerArena) {
      return invalid(`arena ${index} exceeds the arena capacity`);
    }
    return arena;
  });
  let lastNonempty = -1;
  for (let index = 0; index < arenas.length; index++) {
    if (arenas[index].length > 0) lastNonempty = index;
  }
  for (let index = 0; index < lastNonempty; index++) {
    if (arenas[index].length / options.nodeSize !== header.numNodesPerArena) {
      return invalid(
        `arena ${index} is not full before the last nonempty arena`
      );
    }
  }
  const capacity = arenas.reduce(
    (total, arena) => total + arena.length / options.nodeSize,
    0
  );
  if (
    header.size > capacity ||
    bumpIndex > capacity + 1 ||
    header.size >= bumpIndex
  ) {
    return invalid("allocator size exceeds its bounds");
  }
  const node = (pointer: number): ReadonlyUint8Array => {
    if (pointer === 0 || pointer >= bumpIndex) {
      return invalid(`node pointer ${pointer} is outside the allocated range`);
    }
    const arenaIndex = Math.floor((pointer - 1) / header.numNodesPerArena);
    const offset = ((pointer - 1) % header.numNodesPerArena) * options.nodeSize;
    const arena = arenas[arenaIndex];
    if (!arena || offset + options.nodeSize > arena.length) {
      return invalid(`node pointer ${pointer} exceeds its arena`);
    }
    const bytes = arena.subarray(offset, offset + options.nodeSize);
    if (view(bytes).getUint32(4, true) === 0xffffffff) {
      return invalid(`node pointer ${pointer} refers to a free node`);
    }
    return bytes;
  };
  const entries: TreeEntry[] = [];
  const seen = new Set<number>();
  const stack: number[] = [];
  let current = root;
  let parent = 0;
  while (current !== 0 || stack.length > 0) {
    while (current !== 0) {
      if (seen.has(current) || seen.size >= header.size) {
        return invalid("tree has a cycle, repeated node, or inconsistent size");
      }
      const currentView = view(node(current));
      if (
        currentView.getUint32(8, true) !== parent ||
        currentView.getUint32(12, true) > 1
      ) {
        return invalid("tree has an invalid parent pointer or color");
      }
      seen.add(current);
      stack.push(current);
      parent = current;
      current = currentView.getUint32(0, true);
    }
    const pointer = stack.pop();
    if (pointer === undefined) break;
    const bytes = node(pointer);
    entries.push({ nodeIndex: pointer, data: bytes.subarray(16) });
    parent = pointer;
    current = view(bytes).getUint32(4, true);
  }
  if (entries.length !== header.size) {
    return invalid("tree does not contain the declared number of entries");
  }
  return { header, entries };
}
