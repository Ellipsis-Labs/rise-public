import { EMBER_DISCRIMINANTS } from "@/core/discriminants";

/**
 * Encoder/Decoder for Ember Withdraw instruction
 * Uses Borsh Option<u64> encoding: Some(amount) = [1, ...u64_bytes]
 *
 * Format: [discriminant (8 bytes), variant (1 byte), amount (8 bytes)] = 17 bytes
 */

export const getEmberWithdrawEncoder = (): {
  encode: (amount: bigint | null) => Uint8Array;
} => ({
  encode: (amount: bigint | null): Uint8Array => {
    if (amount) {
      const result = new Uint8Array(17);

      // 1. Set discriminant (8 bytes)
      result.set(EMBER_DISCRIMINANTS.WITHDRAW, 0);

      // 2. Set Option::Some variant = 1 (1 byte)
      result[8] = 1;

      // 3. Set amount as u64 little-endian (8 bytes)
      const view = new DataView(result.buffer, 9, 8);
      view.setBigUint64(0, amount, true);

      return result;
    } else {
      const result = new Uint8Array(9);

      // 1. Set discriminant (8 bytes)
      result.set(EMBER_DISCRIMINANTS.WITHDRAW, 0);

      // 2. Set Option::None variant = 0 (1 byte)
      result[8] = 0;

      return result;
    }
  },
});

export const getEmberWithdrawDecoder = (): {
  decode: (
    bytes: Uint8Array | readonly number[],
    offset?: number
  ) => bigint | null;
} => ({
  decode: (
    bytes: Uint8Array | readonly number[],
    offset = 0
  ): bigint | null => {
    const byteArray =
      bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);

    // Skip discriminant (8 bytes)
    const dataOffset = offset + 8;

    // Read Option variant (1 byte)
    const variant = byteArray[dataOffset];

    if (variant === 0) {
      return null;
    }

    if (variant !== 1) {
      throw new Error(`Invalid Option variant: ${variant}, expected 0 or 1`);
    }

    // Read u64 amount (8 bytes after variant)
    const view = new DataView(
      byteArray.buffer,
      byteArray.byteOffset + dataOffset + 1,
      8
    );
    return view.getBigUint64(0, true); // little-endian
  },
});

export const getEmberWithdrawCodec = (): {
  encode: (amount: bigint | null) => Uint8Array;
  decode: (
    bytes: Uint8Array | readonly number[],
    offset?: number
  ) => bigint | null;
} => ({
  encode: getEmberWithdrawEncoder().encode,
  decode: getEmberWithdrawDecoder().decode,
});
