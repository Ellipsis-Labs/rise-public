import { DISCRIMINANTS } from "@/core/discriminants";
import {
  concat,
  encodeSwap,
  packedInstructionChunks,
  u64,
  u8,
} from "@/core/ixBuilders/NativeSol/codec";
import type {
  PackedInstruction,
  SwapDirection,
  SwapSlippage,
} from "@/core/ixBuilders/NativeSol";
import type { WithdrawSpotAction } from "./types";

export const encodeSyncSpot = (): Uint8Array =>
  new Uint8Array(DISCRIMINANTS.SYNC_SPOT);

export const encodeWithdrawSpot = (action: WithdrawSpotAction): Uint8Array => {
  const prefix = new Uint8Array(DISCRIMINANTS.WITHDRAW_SPOT);
  switch (action.kind) {
    case "allExcess":
      return concat(prefix, u8(0));
    case "withExcess":
      return concat(prefix, u8(1), u64(action.amount));
    case "withoutExcess":
      return concat(prefix, u8(2), u64(action.amount));
  }
};

export const encodeTransferSpot = (amount: bigint): Uint8Array =>
  concat(new Uint8Array(DISCRIMINANTS.TRANSFER_SPOT), u64(amount));

export const encodeTransferSpotFromChildToParent = (): Uint8Array =>
  new Uint8Array(DISCRIMINANTS.TRANSFER_SPOT_FROM_CHILD_TO_PARENT);

export const encodeSwapSpotWithUsdc = (
  direction: SwapDirection,
  amountIn: bigint,
  minAmountOut: SwapSlippage,
  instructions: readonly PackedInstruction[]
): Uint8Array =>
  encodeSwap(
    DISCRIMINANTS.SWAP_SPOT_WITH_USDC,
    direction,
    amountIn,
    minAmountOut,
    instructions
  );

export const encodeSwapSpotWithSol = (
  direction: SwapDirection,
  amountIn: bigint,
  minAmountOut: SwapSlippage,
  instructions: readonly PackedInstruction[]
): Uint8Array =>
  encodeSwap(
    DISCRIMINANTS.SWAP_SPOT_WITH_SOL,
    direction,
    amountIn,
    minAmountOut,
    instructions
  );

export const encodeSwapSpotWithSpot = (
  amountIn: bigint,
  minAmountOut: SwapSlippage,
  instructions: readonly PackedInstruction[]
): Uint8Array =>
  concat(
    new Uint8Array(DISCRIMINANTS.SWAP_SPOT_WITH_SPOT),
    u64(amountIn),
    // `unprotected` encodes the on-chain sentinel that disables the check.
    u64(minAmountOut === "unprotected" ? 0n : minAmountOut),
    ...packedInstructionChunks(instructions)
  );

export const encodeLiquidateSpot = (
  maxSpotAmount: bigint,
  numTradersToCheck: bigint,
  instructions: readonly PackedInstruction[]
): Uint8Array =>
  concat(
    new Uint8Array(DISCRIMINANTS.LIQUIDATE_SPOT),
    u64(maxSpotAmount),
    u64(numTradersToCheck),
    ...packedInstructionChunks(instructions)
  );
