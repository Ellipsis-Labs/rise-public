import { DISCRIMINANTS } from "@/core/discriminants";
import { getU32Encoder, getU64Encoder, getU8Encoder } from "@solana/kit";
import type {
  PackedInstruction,
  SwapDirection,
  SwapSlippage,
  WithdrawNativeSolAction,
} from "./types";

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

const u8 = (value: number): Uint8Array =>
  new Uint8Array(getU8Encoder().encode(value));
const u32 = (value: number): Uint8Array =>
  new Uint8Array(getU32Encoder().encode(value));
const u64 = (value: bigint): Uint8Array =>
  new Uint8Array(getU64Encoder().encode(value));

export const encodeSyncNative = (): Uint8Array =>
  new Uint8Array(DISCRIMINANTS.SYNC_NATIVE);

export const encodeWithdrawNativeSol = (
  action: WithdrawNativeSolAction
): Uint8Array => {
  const discriminant = new Uint8Array(DISCRIMINANTS.WITHDRAW_NATIVE_SOL);
  switch (action.kind) {
    case "allExcess":
      return concat(discriminant, u8(0));
    case "withExcess":
      return concat(discriminant, u8(1), u64(action.lamports));
    case "withoutExcess":
      return concat(discriminant, u8(2), u64(action.lamports));
  }
};

export const encodeTransferNativeSol = (lamports: bigint): Uint8Array =>
  concat(new Uint8Array(DISCRIMINANTS.TRANSFER_NATIVE_SOL), u64(lamports));

export const encodeTransferNativeSolFromChildToParent = (): Uint8Array =>
  new Uint8Array(DISCRIMINANTS.TRANSFER_NATIVE_SOL_FROM_CHILD_TO_PARENT);

/** `state = (index << 2) | (isSigner ? 0b10 : 0) | (isWritable ? 0b01 : 0)`. */
export const packedAccountMetaState = (meta: {
  index: number;
  isSigner: boolean;
  isWritable: boolean;
}): number =>
  (meta.index << 2) | (meta.isSigner ? 0b10 : 0) | (meta.isWritable ? 0b01 : 0);

export const encodeSwapNative = (
  direction: SwapDirection,
  amountIn: bigint,
  minAmountOut: SwapSlippage,
  instructions: readonly PackedInstruction[]
): Uint8Array => {
  const chunks: Uint8Array[] = [
    new Uint8Array(DISCRIMINANTS.SWAP_NATIVE),
    u8(direction),
    u64(amountIn),
    // `unprotected` encodes the on-chain sentinel that disables the check.
    u64(minAmountOut === "unprotected" ? 0n : minAmountOut),
    u32(instructions.length),
  ];

  for (const instruction of instructions) {
    chunks.push(
      u8(instruction.programIdIndex),
      u32(instruction.data.length),
      instruction.data,
      u32(instruction.accountMetas.length),
      Uint8Array.from(instruction.accountMetas.map(packedAccountMetaState))
    );
  }

  return concat(...chunks);
};

export const encodeLiquidateNativeSol = (
  maxNativeSolAmount: bigint,
  numTradersToCheck: bigint,
  instructions: readonly PackedInstruction[]
): Uint8Array => {
  const chunks: Uint8Array[] = [
    new Uint8Array(DISCRIMINANTS.LIQUIDATE_NATIVE_SOL),
    u64(maxNativeSolAmount),
    u64(numTradersToCheck),
    u32(instructions.length),
  ];
  for (const instruction of instructions) {
    chunks.push(
      u8(instruction.programIdIndex),
      u32(instruction.data.length),
      instruction.data,
      u32(instruction.accountMetas.length),
      Uint8Array.from(instruction.accountMetas.map(packedAccountMetaState))
    );
  }
  return concat(...chunks);
};
