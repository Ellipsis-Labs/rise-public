import {
  getConstantEncoder,
  type Encoder,
  type ReadonlyUint8Array,
} from "@solana/kit";
import { FLIGHT_DISCRIMINANTS } from "../../discriminants.js";

export const getProxyInstructionPrefixEncoder = (): Encoder<void> =>
  getConstantEncoder(FLIGHT_DISCRIMINANTS.PROXY_INSTRUCTION);

export const encodeProxyInstructionData = (
  innerInstructionData: Uint8Array | ReadonlyUint8Array = new Uint8Array(0)
): Uint8Array => {
  const prefix = getProxyInstructionPrefixEncoder().encode(undefined);
  const data = new Uint8Array(prefix.length + innerInstructionData.length);
  data.set(prefix, 0);
  data.set(innerInstructionData, prefix.length);
  return data;
};
