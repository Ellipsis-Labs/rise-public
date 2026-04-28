import {
  combineCodec,
  getArrayDecoder,
  getArrayEncoder,
  type Codec,
  type ReadonlyUint8Array,
} from "@solana/kit";

export type FixedLengthArray<T, N extends number> = T[] & {
  readonly length: N;
};

export const generatePadding = <T, N extends number>(
  length: N,
  value: T
): FixedLengthArray<T, N> => {
  return Array(length).fill(value) as FixedLengthArray<T, N>;
};

export const getFixedLengthArrayCodec = <T, N extends number>(
  itemCodec: Codec<T>,
  length: N
): Codec<FixedLengthArray<T, N>> => {
  const arrayCodec = combineCodec(
    getArrayEncoder(itemCodec, { size: length }),
    getArrayDecoder(itemCodec, { size: length })
  );

  return {
    encode: (value: FixedLengthArray<T, N>) => {
      if (value.length !== length) {
        throw new Error(
          `Expected array of length ${length}, got ${value.length}`
        );
      }
      return arrayCodec.encode(value);
    },
    write: (value: FixedLengthArray<T, N>, bytes: Uint8Array, offset = 0) => {
      if (value.length !== length) {
        throw new Error(
          `Expected array of length ${length}, got ${value.length}`
        );
      }
      return arrayCodec.write(value, bytes, offset);
    },
    getSizeFromValue: (value: FixedLengthArray<T, N>) => {
      if (value.length !== length) {
        throw new Error(
          `Expected array of length ${length}, got ${value.length}`
        );
      }
      return arrayCodec.getSizeFromValue(value);
    },
    maxSize: arrayCodec.maxSize,
    decode: (bytes: ReadonlyUint8Array, offset = 0): FixedLengthArray<T, N> => {
      const [decoded, _newOffset] = arrayCodec.decode(bytes, offset);
      return decoded as FixedLengthArray<T, N>;
    },
    read: (
      bytes: ReadonlyUint8Array,
      offset = 0
    ): [FixedLengthArray<T, N>, number] => {
      const [decoded, newOffset] = arrayCodec.read(bytes, offset);
      return [decoded as FixedLengthArray<T, N>, newOffset];
    },
  };
};
