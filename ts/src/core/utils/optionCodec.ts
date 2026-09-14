import {
  getOptionDecoder,
  getOptionEncoder,
  getU64Decoder,
  getU64Encoder,
  transformDecoder,
  transformEncoder,
  type Decoder,
  type Encoder,
} from "@solana/kit";

/**
 * Creates a decoder that converts Option<T> to T | null
 */
export const getOptionToNullDecoder = <T>(
  decoder: Decoder<T>
): Decoder<T | null> =>
  transformDecoder(getOptionDecoder(decoder), (option) =>
    option.__option === "Some" ? option.value : null
  );

/**
 * Creates an encoder that converts T | null to Option<T>
 */
export const getOptionToNullEncoder = <T>(
  encoder: Encoder<T>
): Encoder<T | null> =>
  transformEncoder(getOptionEncoder(encoder), (value: T | null) =>
    value === null
      ? { __option: "None" as const }
      : { __option: "Some" as const, value }
  );

/**
 * Decoder for a raw `u64` where `0` means "no value" and any other value is
 * the value itself, minus the borsh `Option` tag byte. Used by
 * `CondensedOrderV2.lastValidSlot`, which trades the tag byte the legacy
 * `CondensedOrder` pays for a fixed-width sentinel.
 */
export const getOptionalNonZeroU64Decoder = (): Decoder<bigint | null> =>
  transformDecoder(getU64Decoder(), (value) => (value === 0n ? null : value));

/**
 * Encoder for a raw `u64` where `null` is encoded as `0` and any other value
 * is encoded as itself. `0` is reserved as the "no value" sentinel and is
 * not representable as an explicit value; passing `0n` throws. See
 * {@link getOptionalNonZeroU64Decoder}.
 */
export const getOptionalNonZeroU64Encoder = (): Encoder<bigint | null> =>
  transformEncoder(getU64Encoder(), (value: bigint | null) => {
    // BigInt() coerces so a runtime `number` 0 from an untyped caller is
    // caught too, not just a literal 0n.
    if (value !== null && BigInt(value) === 0n) {
      throw new Error(
        "0 cannot be represented in this optional-nonzero-u64 wire format " +
          "(0 is the no-value sentinel); pass null instead."
      );
    }
    return value ?? 0n;
  });
