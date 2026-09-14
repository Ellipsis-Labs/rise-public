import { FLICKER_DISCRIMINANTS } from "@/core/discriminants";
import { baseLots, quoteLots, ticks } from "@/primitives/_numberTypes";
import type { ImmediateOrCancelOrderPacket } from "@/primitives/OrderPacket";
import type { TwapIocOrderPacketData } from "./types";
import type {
  CreateTwapAccountData,
  ExecuteTwapOrderData,
  PlaceTwapOrderData,
} from "./types";

export const TWAP_IOC_ORDER_PACKET_DISCRIMINANT = 1n;
export const TWAP_IOC_ORDER_PACKET_BYTE_LENGTH = 152;

type TwapEncoder<T> = { encode: (value: T) => Uint8Array };
type TwapDecoder<T> = {
  decode: (bytes: Uint8Array | readonly number[], offset?: number) => T;
};
type TwapCodec<T> = TwapEncoder<T> & TwapDecoder<T>;

const U8_MAX = 0xff;
const U32_MAX = 0xffff_ffff;
const U64_MAX = (1n << 64n) - 1n;
const U128_MAX = (1n << 128n) - 1n;

const concat = (...chunks: Uint8Array[]): Uint8Array => {
  const total = chunks.reduce((length, chunk) => length + chunk.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.length;
  }
  return out;
};

const toBytes = (bytes: Uint8Array | readonly number[]): Uint8Array =>
  bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);

const assertIntegerInRange = (
  value: number,
  min: number,
  max: number,
  field: string
): number => {
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new Error(`${field} must be an integer in ${min}..=${max}`);
  }
  return value;
};

const toBigIntValue = (value: bigint | number, field: string): bigint => {
  if (typeof value === "bigint") return value;
  if (!Number.isSafeInteger(value)) {
    throw new Error(`${field} must be a safe integer number or bigint`);
  }
  return BigInt(value);
};

const assertU8 = (value: number, field: string): number =>
  assertIntegerInRange(value, 0, U8_MAX, field);

const assertU32 = (value: number, field: string): number =>
  assertIntegerInRange(value, 0, U32_MAX, field);

const assertU64 = (value: bigint | number, field: string): bigint => {
  const bigintValue = toBigIntValue(value, field);
  if (bigintValue < 0n || bigintValue > U64_MAX) {
    throw new Error(`${field} must fit in u64`);
  }
  return bigintValue;
};

const assertU128 = (value: bigint, field: string): bigint => {
  if (value < 0n || value > U128_MAX) {
    throw new Error(`${field} must fit in u128`);
  }
  return value;
};

const u8 = (value: number, field = "u8"): Uint8Array =>
  Uint8Array.of(assertU8(value, field));

const u32 = (value: number, field = "u32"): Uint8Array => {
  const out = new Uint8Array(4);
  new DataView(out.buffer).setUint32(0, assertU32(value, field), true);
  return out;
};

const u64 = (value: bigint | number, field = "u64"): Uint8Array => {
  const out = new Uint8Array(8);
  new DataView(out.buffer).setBigUint64(0, assertU64(value, field), true);
  return out;
};

const u128 = (value: bigint, field = "u128"): Uint8Array => {
  const out = new Uint8Array(16);
  const bigintValue = assertU128(value, field);
  new DataView(out.buffer).setBigUint64(0, bigintValue & U64_MAX, true);
  new DataView(out.buffer).setBigUint64(8, bigintValue >> 64n, true);
  return out;
};

const zeroes = (length: number): Uint8Array => new Uint8Array(length);

const optionalRawNonZeroU64 = (
  value: bigint | number | null | undefined,
  field: string
): Uint8Array => {
  if (value === null || value === undefined) return u64(0n, field);
  const bigintValue = assertU64(value, field);
  if (bigintValue === 0n) {
    throw new Error(`${field} must be greater than 0 when set`);
  }
  return u64(bigintValue, field);
};

const optionNonZeroU64 = (
  value: bigint | number | null | undefined,
  field: string
): Uint8Array => {
  if (value === null || value === undefined) return u8(0);
  const bigintValue = assertU64(value, field);
  if (bigintValue === 0n) {
    throw new Error(`${field} must be greater than 0 when set`);
  }
  return concat(u8(1), u64(bigintValue, field));
};

const optionU64 = (
  value: bigint | number | null | undefined,
  field: string
): Uint8Array =>
  value === null || value === undefined
    ? u8(0)
    : concat(u8(1), u64(value, field));

const expectLength = (bytes: Uint8Array, offset: number, length: number) => {
  if (offset + length > bytes.length) {
    throw new Error(
      `Instruction data is too short: need ${offset + length} bytes, got ${bytes.length}`
    );
  }
};

const expectExactLength = (bytes: Uint8Array, expected: number) => {
  if (bytes.length !== expected) {
    throw new Error(
      `Invalid instruction data length: expected ${expected}, got ${bytes.length}`
    );
  }
};

const expectDiscriminant = (
  bytes: Uint8Array,
  offset: number,
  expected: Uint8Array,
  name: string
) => {
  expectLength(bytes, offset, expected.length);
  for (let i = 0; i < expected.length; i++) {
    if (bytes[offset + i] !== expected[i]) {
      throw new Error(`Invalid ${name} discriminant`);
    }
  }
};

const readU8 = (bytes: Uint8Array, offset: number): [number, number] => {
  expectLength(bytes, offset, 1);
  return [bytes[offset], offset + 1];
};

const readU32 = (bytes: Uint8Array, offset: number): [number, number] => {
  expectLength(bytes, offset, 4);
  return [
    new DataView(bytes.buffer, bytes.byteOffset + offset, 4).getUint32(0, true),
    offset + 4,
  ];
};

const readU64 = (bytes: Uint8Array, offset: number): [bigint, number] => {
  expectLength(bytes, offset, 8);
  return [
    new DataView(bytes.buffer, bytes.byteOffset + offset, 8).getBigUint64(
      0,
      true
    ),
    offset + 8,
  ];
};

const readU128 = (bytes: Uint8Array, offset: number): [bigint, number] => {
  const [lo, highOffset] = readU64(bytes, offset);
  const [hi, nextOffset] = readU64(bytes, highOffset);
  return [lo | (hi << 64n), nextOffset];
};

const readOptionNonZeroU64 = (
  bytes: Uint8Array,
  offset: number,
  field: string
): [bigint | null, number] => {
  const [tag, valueOffset] = readU8(bytes, offset);
  if (tag === 0) return [null, valueOffset];
  if (tag !== 1) throw new Error(`Invalid option tag for ${field}`);
  const [value, nextOffset] = readU64(bytes, valueOffset);
  if (value === 0n) {
    throw new Error(`${field} must be greater than 0 when set`);
  }
  return [value, nextOffset];
};

const readOptionU64 = (
  bytes: Uint8Array,
  offset: number,
  field: string
): [bigint | null, number] => {
  const [tag, valueOffset] = readU8(bytes, offset);
  if (tag === 0) return [null, valueOffset];
  if (tag !== 1) throw new Error(`Invalid option tag for ${field}`);
  return readU64(bytes, valueOffset);
};

const rawOptionalNonZero = (value: bigint): bigint | null =>
  value === 0n ? null : value;

const expectZeroes = (
  bytes: Uint8Array,
  offset: number,
  length: number,
  field: string
) => {
  expectLength(bytes, offset, length);
  for (let i = 0; i < length; i++) {
    if (bytes[offset + i] !== 0) {
      throw new Error(`${field} must be zero-filled`);
    }
  }
};

export const encodeTwapIocOrderPacket = (
  packet: ImmediateOrCancelOrderPacket
): Uint8Array =>
  concat(
    u64(TWAP_IOC_ORDER_PACKET_DISCRIMINANT, "iocDiscriminant"),
    u8(Number(packet.side), "side"),
    zeroes(7),
    optionalRawNonZeroU64(packet.priceInTicks, "priceInTicks"),
    u64(packet.numBaseLots, "numBaseLots"),
    optionalRawNonZeroU64(packet.numQuoteLots, "numQuoteLots"),
    u64(packet.minBaseLotsToFill, "minBaseLotsToFill"),
    u64(packet.minQuoteLotsToFill, "minQuoteLotsToFill"),
    u8(Number(packet.selfTradeBehavior), "selfTradeBehavior"),
    zeroes(7),
    optionalRawNonZeroU64(packet.matchLimit, "matchLimit"),
    u128(packet.clientOrderId, "clientOrderId"),
    optionalRawNonZeroU64(packet.lastValidSlot, "lastValidSlot"),
    u8(Number(packet.orderFlags), "orderFlags"),
    u8(packet.cancelExisting ? 1 : 0, "cancelExisting"),
    zeroes(6),
    zeroes(48)
  );

export const decodeTwapIocOrderPacket = (
  bytes: Uint8Array | readonly number[],
  offset = 0
): TwapIocOrderPacketData => {
  const byteArray = toBytes(bytes);
  expectLength(byteArray, offset, TWAP_IOC_ORDER_PACKET_BYTE_LENGTH);
  const endOffset = offset + TWAP_IOC_ORDER_PACKET_BYTE_LENGTH;

  let cursor = offset;
  const [discriminant, discriminantOffset] = readU64(byteArray, cursor);
  cursor = discriminantOffset;
  if (discriminant !== TWAP_IOC_ORDER_PACKET_DISCRIMINANT) {
    throw new Error("Invalid TWAP IOC order packet discriminant");
  }
  const [side, sideOffset] = readU8(byteArray, cursor);
  cursor = sideOffset;
  if (side !== 0 && side !== 1) throw new Error("Invalid TWAP IOC side");
  expectZeroes(byteArray, cursor, 7, "paddingAfterSide");
  cursor += 7;

  const [priceInTicks, priceOffset] = readU64(byteArray, cursor);
  cursor = priceOffset;
  const [numBaseLots, baseOffset] = readU64(byteArray, cursor);
  cursor = baseOffset;
  const [numQuoteLots, quoteOffset] = readU64(byteArray, cursor);
  cursor = quoteOffset;
  const [minBaseLotsToFill, minBaseOffset] = readU64(byteArray, cursor);
  cursor = minBaseOffset;
  const [minQuoteLotsToFill, minQuoteOffset] = readU64(byteArray, cursor);
  cursor = minQuoteOffset;

  const [selfTradeBehavior, selfTradeOffset] = readU8(byteArray, cursor);
  cursor = selfTradeOffset;
  if (![0, 1, 2].includes(selfTradeBehavior)) {
    throw new Error("Invalid TWAP IOC self-trade behavior");
  }
  expectZeroes(byteArray, cursor, 7, "paddingAfterSelfTradeBehavior");
  cursor += 7;

  const [matchLimit, matchLimitOffset] = readU64(byteArray, cursor);
  cursor = matchLimitOffset;
  const [clientOrderId, clientOrderOffset] = readU128(byteArray, cursor);
  cursor = clientOrderOffset;
  const [lastValidSlot, lastValidSlotOffset] = readU64(byteArray, cursor);
  cursor = lastValidSlotOffset;
  const [orderFlags, orderFlagsOffset] = readU8(byteArray, cursor);
  cursor = orderFlagsOffset;
  const [cancelExisting, cancelExistingOffset] = readU8(byteArray, cursor);
  cursor = cancelExistingOffset;
  if (cancelExisting !== 0 && cancelExisting !== 1) {
    throw new Error("Invalid TWAP IOC cancelExisting flag");
  }
  expectZeroes(byteArray, cursor, 6, "paddingAfterFlags");
  cursor += 6;
  expectZeroes(byteArray, cursor, 48, "reserved");
  cursor += 48;
  if (cursor !== endOffset) throw new Error("Invalid TWAP IOC packet length");

  return {
    side: side as TwapIocOrderPacketData["side"],
    priceInTicks:
      rawOptionalNonZero(priceInTicks) === null ? null : ticks(priceInTicks),
    numBaseLots: baseLots(numBaseLots),
    numQuoteLots:
      rawOptionalNonZero(numQuoteLots) === null
        ? null
        : quoteLots(numQuoteLots),
    minBaseLotsToFill: baseLots(minBaseLotsToFill),
    minQuoteLotsToFill: quoteLots(minQuoteLotsToFill),
    selfTradeBehavior:
      selfTradeBehavior as TwapIocOrderPacketData["selfTradeBehavior"],
    matchLimit: rawOptionalNonZero(matchLimit),
    clientOrderId,
    lastValidSlot: rawOptionalNonZero(lastValidSlot),
    orderFlags: orderFlags as TwapIocOrderPacketData["orderFlags"],
    cancelExisting: cancelExisting === 1,
  };
};

export const getTwapIocOrderPacketEncoder =
  (): TwapEncoder<ImmediateOrCancelOrderPacket> => ({
    encode: encodeTwapIocOrderPacket,
  });

export const getTwapIocOrderPacketDecoder =
  (): TwapDecoder<TwapIocOrderPacketData> => ({
    decode: decodeTwapIocOrderPacket,
  });

export const getCreateTwapAccountEncoder =
  (): TwapEncoder<CreateTwapAccountData> => ({
    encode: (value) =>
      concat(
        new Uint8Array(FLICKER_DISCRIMINANTS.CREATE_TWAP_ACCOUNT),
        u32(value.marketId, "marketId")
      ),
  });

export const getCreateTwapAccountDecoder =
  (): TwapDecoder<CreateTwapAccountData> => ({
    decode: (bytes, offset = 0) => {
      const byteArray = toBytes(bytes);
      expectExactLength(byteArray.subarray(offset), 12);
      expectDiscriminant(
        byteArray,
        offset,
        FLICKER_DISCRIMINANTS.CREATE_TWAP_ACCOUNT,
        "CreateTwapAccount"
      );
      const [marketId] = readU32(byteArray, offset + 8);
      return { marketId };
    },
  });

export const getCreateTwapAccountCodec =
  (): TwapCodec<CreateTwapAccountData> => ({
    encode: getCreateTwapAccountEncoder().encode,
    decode: getCreateTwapAccountDecoder().decode,
  });

export const getPlaceTwapOrderEncoder =
  (): TwapEncoder<PlaceTwapOrderData> => ({
    encode: (value) =>
      concat(
        new Uint8Array(FLICKER_DISCRIMINANTS.PLACE_TWAP_ORDER),
        u64(value.cooldownSlots, "cooldownSlots"),
        u64(value.nChildOrders, "nChildOrders"),
        u64(value.childOrderMaxSlippageBps, "childOrderMaxSlippageBps"),
        optionNonZeroU64(
          value.childOrderMinPriceInTicks,
          "childOrderMinPriceInTicks"
        ),
        optionNonZeroU64(
          value.childOrderMaxPriceInTicks,
          "childOrderMaxPriceInTicks"
        ),
        encodeTwapIocOrderPacket(value.childOrderPacket),
        optionNonZeroU64(
          value.childOrderCollateralQuoteLotsToTransfer,
          "childOrderCollateralQuoteLotsToTransfer"
        ),
        optionU64(value.lastValidSlot, "lastValidSlot"),
        u8(
          value.transferCollateralAccountCount,
          "transferCollateralAccountCount"
        )
      ),
  });

export const getPlaceTwapOrderDecoder =
  (): TwapDecoder<PlaceTwapOrderData> => ({
    decode: (bytes, offset = 0) => {
      const byteArray = toBytes(bytes);
      expectDiscriminant(
        byteArray,
        offset,
        FLICKER_DISCRIMINANTS.PLACE_TWAP_ORDER,
        "PlaceTwapOrder"
      );
      let cursor = offset + 8;
      const [cooldownSlots, cooldownOffset] = readU64(byteArray, cursor);
      cursor = cooldownOffset;
      const [nChildOrders, childOrdersOffset] = readU64(byteArray, cursor);
      cursor = childOrdersOffset;
      const [childOrderMaxSlippageBps, slippageOffset] = readU64(
        byteArray,
        cursor
      );
      cursor = slippageOffset;
      const [childOrderMinPriceInTicks, minPriceOffset] = readOptionNonZeroU64(
        byteArray,
        cursor,
        "childOrderMinPriceInTicks"
      );
      cursor = minPriceOffset;
      const [childOrderMaxPriceInTicks, maxPriceOffset] = readOptionNonZeroU64(
        byteArray,
        cursor,
        "childOrderMaxPriceInTicks"
      );
      cursor = maxPriceOffset;
      const childOrderPacket = decodeTwapIocOrderPacket(byteArray, cursor);
      cursor += TWAP_IOC_ORDER_PACKET_BYTE_LENGTH;
      const [childOrderCollateralQuoteLotsToTransfer, collateralOffset] =
        readOptionNonZeroU64(
          byteArray,
          cursor,
          "childOrderCollateralQuoteLotsToTransfer"
        );
      cursor = collateralOffset;
      const [lastValidSlot, lastValidSlotOffset] = readOptionU64(
        byteArray,
        cursor,
        "lastValidSlot"
      );
      cursor = lastValidSlotOffset;
      const [transferCollateralAccountCount, transferCountOffset] = readU8(
        byteArray,
        cursor
      );
      cursor = transferCountOffset;
      if (cursor !== byteArray.length) {
        throw new Error("Invalid PlaceTwapOrder instruction length");
      }

      return {
        cooldownSlots,
        nChildOrders,
        childOrderMaxSlippageBps,
        childOrderMinPriceInTicks:
          childOrderMinPriceInTicks === null
            ? null
            : ticks(childOrderMinPriceInTicks),
        childOrderMaxPriceInTicks:
          childOrderMaxPriceInTicks === null
            ? null
            : ticks(childOrderMaxPriceInTicks),
        childOrderPacket,
        childOrderCollateralQuoteLotsToTransfer:
          childOrderCollateralQuoteLotsToTransfer === null
            ? null
            : quoteLots(childOrderCollateralQuoteLotsToTransfer),
        lastValidSlot,
        transferCollateralAccountCount,
      };
    },
  });

export const getPlaceTwapOrderCodec = (): TwapCodec<PlaceTwapOrderData> => ({
  encode: getPlaceTwapOrderEncoder().encode,
  decode: getPlaceTwapOrderDecoder().decode,
});

export const getExecuteTwapOrderEncoder =
  (): TwapEncoder<ExecuteTwapOrderData> => ({
    encode: (value) =>
      concat(
        new Uint8Array(FLICKER_DISCRIMINANTS.EXECUTE_TWAP_ORDER),
        u8(
          value.transferCollateralAccountCount,
          "transferCollateralAccountCount"
        )
      ),
  });

export const getExecuteTwapOrderDecoder =
  (): TwapDecoder<ExecuteTwapOrderData> => ({
    decode: (bytes, offset = 0) => {
      const byteArray = toBytes(bytes);
      expectExactLength(byteArray.subarray(offset), 9);
      expectDiscriminant(
        byteArray,
        offset,
        FLICKER_DISCRIMINANTS.EXECUTE_TWAP_ORDER,
        "ExecuteTwapOrder"
      );
      const [transferCollateralAccountCount] = readU8(byteArray, offset + 8);
      return { transferCollateralAccountCount };
    },
  });

export const getExecuteTwapOrderCodec =
  (): TwapCodec<ExecuteTwapOrderData> => ({
    encode: getExecuteTwapOrderEncoder().encode,
    decode: getExecuteTwapOrderDecoder().decode,
  });

export const getCancelTwapOrderEncoder = (): TwapEncoder<void> => ({
  encode: () => new Uint8Array(FLICKER_DISCRIMINANTS.CANCEL_TWAP_ORDER),
});

export const getCancelTwapOrderDecoder = (): TwapDecoder<void> => ({
  decode: (bytes, offset = 0) => {
    const byteArray = toBytes(bytes);
    expectExactLength(byteArray.subarray(offset), 8);
    expectDiscriminant(
      byteArray,
      offset,
      FLICKER_DISCRIMINANTS.CANCEL_TWAP_ORDER,
      "CancelTwapOrder"
    );
  },
});

export const getCancelTwapOrderCodec = (): TwapCodec<void> => ({
  encode: getCancelTwapOrderEncoder().encode,
  decode: getCancelTwapOrderDecoder().decode,
});

export const getCloseInactiveTwapAccountEncoder = (): TwapEncoder<void> => ({
  encode: () =>
    new Uint8Array(FLICKER_DISCRIMINANTS.CLOSE_INACTIVE_TWAP_ACCOUNT),
});

export const getCloseInactiveTwapAccountDecoder = (): TwapDecoder<void> => ({
  decode: (bytes, offset = 0) => {
    const byteArray = toBytes(bytes);
    expectExactLength(byteArray.subarray(offset), 8);
    expectDiscriminant(
      byteArray,
      offset,
      FLICKER_DISCRIMINANTS.CLOSE_INACTIVE_TWAP_ACCOUNT,
      "CloseInactiveTwapAccount"
    );
  },
});

export const getCloseInactiveTwapAccountCodec = (): TwapCodec<void> => ({
  encode: getCloseInactiveTwapAccountEncoder().encode,
  decode: getCloseInactiveTwapAccountDecoder().decode,
});
