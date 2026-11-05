import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";
import type { TransferCollateralParams } from "../TransferCollateral/types";

export interface WithdrawToTargetLeverageParams extends Omit<
  TransferCollateralParams,
  "amount"
> {
  targetLeverageBps: bigint; // basis points, 10_000n = 1x
}

export type WithdrawToTargetLeverageIx = InstructionsWithAccountsAndData;

export type WithdrawToTargetLeverageAccounts = readonly AccountMeta[];
