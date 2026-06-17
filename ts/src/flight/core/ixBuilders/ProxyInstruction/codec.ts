import {
  combineCodec,
  getConstantEncoder,
  getStructDecoder,
  getStructEncoder,
  getU64Decoder,
  getU64Encoder,
  type Codec,
  type Decoder,
  type Encoder,
  type ReadonlyUint8Array,
} from "@solana/kit";
import { getOptionToNullDecoder, getOptionToNullEncoder } from "@/core/utils";
import { FLIGHT_DISCRIMINANTS } from "../../discriminants.js";

export interface ProxyInstructionWithFeeOverrideParamsData {
  feeBpsOverride: bigint | null;
}

export const getProxyInstructionPrefixEncoder = (): Encoder<void> =>
  getConstantEncoder(FLIGHT_DISCRIMINANTS.PROXY_INSTRUCTION);

export const getProxyInstructionWithFeeOverridePrefixEncoder =
  (): Encoder<void> =>
    getConstantEncoder(
      FLIGHT_DISCRIMINANTS.PROXY_INSTRUCTION_WITH_FEE_OVERRIDE
    );

export const getProxyInstructionWithFeeOverrideParamsEncoder =
  (): Encoder<ProxyInstructionWithFeeOverrideParamsData> =>
    getStructEncoder([
      ["feeBpsOverride", getOptionToNullEncoder(getU64Encoder())],
    ]);

export const getProxyInstructionWithFeeOverrideParamsDecoder =
  (): Decoder<ProxyInstructionWithFeeOverrideParamsData> =>
    getStructDecoder([
      ["feeBpsOverride", getOptionToNullDecoder(getU64Decoder())],
    ]);

export const getProxyInstructionWithFeeOverrideParamsCodec =
  (): Codec<ProxyInstructionWithFeeOverrideParamsData> =>
    combineCodec(
      getProxyInstructionWithFeeOverrideParamsEncoder(),
      getProxyInstructionWithFeeOverrideParamsDecoder()
    );

export const encodeProxyInstructionData = (
  innerInstructionData: Uint8Array | ReadonlyUint8Array = new Uint8Array(0)
): Uint8Array => {
  const prefix = getProxyInstructionPrefixEncoder().encode(undefined);
  const data = new Uint8Array(prefix.length + innerInstructionData.length);
  data.set(prefix, 0);
  data.set(innerInstructionData, prefix.length);
  return data;
};

export const encodeProxyInstructionWithFeeOverrideData = (
  feeBpsOverride: bigint,
  innerInstructionData: Uint8Array | ReadonlyUint8Array = new Uint8Array(0)
): Uint8Array => {
  const prefix =
    getProxyInstructionWithFeeOverridePrefixEncoder().encode(undefined);
  const params = getProxyInstructionWithFeeOverrideParamsEncoder().encode({
    feeBpsOverride,
  });
  const data = new Uint8Array(
    prefix.length + params.length + innerInstructionData.length
  );
  data.set(prefix, 0);
  data.set(params, prefix.length);
  data.set(innerInstructionData, prefix.length + params.length);
  return data;
};
