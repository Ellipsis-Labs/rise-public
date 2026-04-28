import { ACCOUNT_DISCRIMINANTS } from "@/core/discriminants";
import type { Decoder } from "@solana/kit";
import {
  getConstantDecoder,
  getHiddenPrefixDecoder,
  getStructDecoder,
  getU16Decoder,
  getU64Decoder,
  getU8Decoder,
  transformDecoder,
} from "@solana/kit";
import type { GlobalConfiguration } from "./types";
import {
  getFixedArrayDecoder,
  getActiveTraderBufferAddressHeaderDecoder,
  getAuthoritySetDecoder,
  getGlobalConfigurationAddressDecoder,
  getGlobalTraderIndexAddressHeaderDecoder,
  getGlobalVaultAddressDecoder,
  getMintAddressDecoder,
  getPerpAssetMapAddressDecoder,
  getWithdrawQueueAddressDecoder,
} from "../internal";
import { getQuoteLotsDecoder } from "@/primitives/_numberTypes";

export const getGlobalConfigurationDecoder = (): Decoder<GlobalConfiguration> =>
  transformDecoder(
    getHiddenPrefixDecoder(
      getStructDecoder([
        ["accountKey", getGlobalConfigurationAddressDecoder()],
        ["currentAuthorities", getAuthoritySetDecoder()],
        ["canonicalTokenMintKey", getMintAddressDecoder()],
        ["globalVaultKey", getGlobalVaultAddressDecoder()],
        ["perpAssetMapKey", getPerpAssetMapAddressDecoder()],
        [
          "globalTraderIndexHeaderKey",
          getGlobalTraderIndexAddressHeaderDecoder(),
        ],
        [
          "activeTraderBufferHeaderKey",
          getActiveTraderBufferAddressHeaderDecoder(),
        ],
        ["totalQuoteLotFees", getQuoteLotsDecoder()],
        ["unclaimedQuoteLotFees", getQuoteLotsDecoder()],
        ["withdrawQueueKey", getWithdrawQueueAddressDecoder()],
        ["exchangeStatus", getU8Decoder()],
        ["quoteDecimals", getU8Decoder()],
        ["withdrawalMarginFactorBps", getU16Decoder()],
        ["_padding0", getFixedArrayDecoder(getU8Decoder, 4)],
        ["depositCooldownPeriodInSlots", getU64Decoder()],
        ["pendingAuthorities", getAuthoritySetDecoder()],
        ["_padding1", getFixedArrayDecoder(getU64Decoder, 31)],
        ["_padding2", getFixedArrayDecoder(getU64Decoder, 32)],
        ["_padding3", getFixedArrayDecoder(getU64Decoder, 32)],
        ["_padding4", getFixedArrayDecoder(getU64Decoder, 32)],
        ["_padding5", getFixedArrayDecoder(getU64Decoder, 32)],
        ["_padding6", getFixedArrayDecoder(getU64Decoder, 32)],
        ["_padding7", getFixedArrayDecoder(getU64Decoder, 32)],
      ]),
      [getConstantDecoder(ACCOUNT_DISCRIMINANTS.GLOBAL_CONFIGURATION)]
    ),
    ({
      _padding0,
      _padding1,
      _padding2,
      _padding3,
      _padding4,
      _padding5,
      _padding6,
      _padding7,
      ...configuration
    }): GlobalConfiguration => configuration
  );

export const decodeGlobalConfiguration = (
  bytes: Uint8Array | Readonly<Uint8Array>
): GlobalConfiguration => getGlobalConfigurationDecoder().decode(bytes);
