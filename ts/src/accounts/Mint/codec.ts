import {
  getAuthorityDecoder,
  getAuthorityEncoder,
} from "@/primitives/_addressTypes";
import {
  combineCodec,
  getBooleanDecoder,
  getBooleanEncoder,
  getStructDecoder,
  getStructEncoder,
  getU32Decoder,
  getU32Encoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  transformDecoder,
  type Codec,
  type Decoder,
  type Encoder,
} from "@solana/kit";
import type { Mint } from "./types";

interface MintInternal {
  mintAuthorityOption: number;
  mintAuthority: Mint["mintAuthority"];
  supply: bigint;
  decimals: number;
  isInitialized: boolean;
  freezeAuthorityOption: number;
  freezeAuthority: Mint["freezeAuthority"];
}

const getMintInternalDecoder = (): Decoder<MintInternal> =>
  getStructDecoder([
    ["mintAuthorityOption", getU32Decoder()],
    ["mintAuthority", getAuthorityDecoder()],
    ["supply", getU64Decoder()],
    ["decimals", getU8Decoder()],
    ["isInitialized", getBooleanDecoder()],
    ["freezeAuthorityOption", getU32Decoder()],
    ["freezeAuthority", getAuthorityDecoder()],
  ]);

const getMintInternalEncoder = (): Encoder<Mint> =>
  getStructEncoder([
    ["mintAuthorityOption", getU32Encoder()],
    ["mintAuthority", getAuthorityEncoder()],
    ["supply", getU64Encoder()],
    ["decimals", getU8Encoder()],
    ["isInitialized", getBooleanEncoder()],
    ["freezeAuthorityOption", getU32Encoder()],
    ["freezeAuthority", getAuthorityEncoder()],
  ]);

export const getMintEncoder = (): Encoder<Mint> => getMintInternalEncoder();

export const getMintDecoder = (): Decoder<Mint> =>
  transformDecoder(
    getMintInternalDecoder(),
    (internal): Mint => ({
      ...internal,
      mintAuthorityOption: internal.mintAuthorityOption as 0 | 1,
      freezeAuthorityOption: internal.freezeAuthorityOption as 0 | 1,
    })
  );

export const getMintCodec = (): Codec<Mint> =>
  combineCodec(getMintEncoder(), getMintDecoder());

export const decodeMint = (bytes: Uint8Array | Readonly<Uint8Array>): Mint =>
  getMintDecoder().decode(bytes);
