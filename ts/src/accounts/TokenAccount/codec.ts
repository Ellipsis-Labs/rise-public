import {
  getAuthorityDecoder,
  getAuthorityEncoder,
  getMintAddressDecoder,
  getMintAddressEncoder,
} from "@/primitives/_addressTypes";
import {
  combineCodec,
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
import type { TokenAccount, TokenAccountState } from "./types";

interface TokenAccountInternal {
  mint: TokenAccount["mint"];
  owner: TokenAccount["owner"];
  amount: bigint;
  delegateOption: number;
  delegate: TokenAccount["delegate"];
  state: number;
  isNativeOption: number;
  isNative: bigint;
  delegatedAmount: bigint;
  closeAuthorityOption: number;
  closeAuthority: TokenAccount["closeAuthority"];
}

const getTokenAccountInternalDecoder = (): Decoder<TokenAccountInternal> =>
  getStructDecoder([
    ["mint", getMintAddressDecoder()],
    ["owner", getAuthorityDecoder()],
    ["amount", getU64Decoder()],
    ["delegateOption", getU32Decoder()],
    ["delegate", getAuthorityDecoder()],
    ["state", getU8Decoder()],
    ["isNativeOption", getU32Decoder()],
    ["isNative", getU64Decoder()],
    ["delegatedAmount", getU64Decoder()],
    ["closeAuthorityOption", getU32Decoder()],
    ["closeAuthority", getAuthorityDecoder()],
  ]);

const getTokenAccountInternalEncoder = (): Encoder<TokenAccount> =>
  getStructEncoder([
    ["mint", getMintAddressEncoder()],
    ["owner", getAuthorityEncoder()],
    ["amount", getU64Encoder()],
    ["delegateOption", getU32Encoder()],
    ["delegate", getAuthorityEncoder()],
    ["state", getU8Encoder()],
    ["isNativeOption", getU32Encoder()],
    ["isNative", getU64Encoder()],
    ["delegatedAmount", getU64Encoder()],
    ["closeAuthorityOption", getU32Encoder()],
    ["closeAuthority", getAuthorityEncoder()],
  ]);

export const getTokenAccountEncoder = (): Encoder<TokenAccount> =>
  getTokenAccountInternalEncoder();

export const getTokenAccountDecoder = (): Decoder<TokenAccount> =>
  transformDecoder(
    getTokenAccountInternalDecoder(),
    (internal): TokenAccount => ({
      ...internal,
      delegateOption: internal.delegateOption as 0 | 1,
      state: internal.state as TokenAccountState,
      isNativeOption: internal.isNativeOption as 0 | 1,
      closeAuthorityOption: internal.closeAuthorityOption as 0 | 1,
    })
  );

export const getTokenAccountCodec = (): Codec<TokenAccount> =>
  combineCodec(getTokenAccountEncoder(), getTokenAccountDecoder());

export const decodeTokenAccount = (
  bytes: Uint8Array | Readonly<Uint8Array>
): TokenAccount => getTokenAccountDecoder().decode(bytes);
