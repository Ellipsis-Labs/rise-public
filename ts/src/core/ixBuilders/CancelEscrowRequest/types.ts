import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { Authority } from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface CancelEscrowRequestParams extends PhoenixInstructionAddressOverrides {
  signerWallet: Authority;
  receiverWallet: Authority;
  receiverEscrow: Address;
  sequenceNumber: bigint;
}

export type CancelEscrowRequestIx = InstructionsWithAccountsAndData;

export type CancelEscrowRequestAccounts = readonly AccountMeta[];
