import {
  getOptionDecoder,
  getOptionEncoder,
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
