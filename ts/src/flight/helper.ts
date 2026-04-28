import { DISCRIMINANTS } from "@/core/discriminants.js";
import type { Instruction } from "@solana/kit";

const hasDiscriminant = (
  data: Instruction["data"],
  discriminant: Uint8Array
): boolean => {
  if (!data || data.length < discriminant.length) {
    return false;
  }

  for (let index = 0; index < discriminant.length; index += 1) {
    if (data[index] !== discriminant[index]) {
      return false;
    }
  }

  return true;
};

export const isOrderPlacingInstruction = (instruction: Instruction): boolean =>
  hasDiscriminant(instruction.data, DISCRIMINANTS.PLACE_MARKET_ORDER) ||
  hasDiscriminant(instruction.data, DISCRIMINANTS.PLACE_LIMIT_ORDER) ||
  hasDiscriminant(
    instruction.data,
    DISCRIMINANTS.PLACE_POSITION_CONDITIONAL_ORDER
  );
