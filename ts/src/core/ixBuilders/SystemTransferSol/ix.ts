import { SYSTEM_PROGRAM_ADDRESS } from "@/core/constants";
import {
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { encodeSystemTransferSol } from "./codec";
import type {
  TransferSolAccounts,
  TransferSolIx,
  TransferSolParams,
} from "./types";

/**
 * A plain System Program SOL transfer.
 *
 * Exists here because rise's submission model signs raw instructions and never
 * materializes a `TransactionSigner`, so `@solana-program/system`'s
 * `getTransferSolInstruction` (which requires one) cannot be used by callers
 * composing a transfer into a rise flow. Account order is load-bearing: the
 * program reads source then destination.
 *
 * This is how lamports enter a trader account for a native SOL deposit — the
 * transfer lands them, and a following `SyncNative` accounts them (see
 * `buildNativeSolDepositFlow`).
 */
export const buildTransferSolIx = (
  params: TransferSolParams
): TransferSolIx => {
  if (!params.source) {
    throw new Error("Source is required");
  }
  if (!params.destination) {
    throw new Error("Destination is required");
  }
  if (params.lamports === undefined || params.lamports <= 0n) {
    throw new Error("Lamports must be greater than 0");
  }

  const accounts: TransferSolAccounts = [
    generateWritableSignerAccount(params.source),
    generateWritableAccount(params.destination),
  ] as const;

  return {
    programAddress: SYSTEM_PROGRAM_ADDRESS,
    accounts,
    data: encodeSystemTransferSol(params.lamports),
  };
};
