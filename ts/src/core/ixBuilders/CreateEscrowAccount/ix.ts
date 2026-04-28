import {
  getPhoenixInstructionAddresses,
  SYSTEM_PROGRAM_ADDRESS,
} from "@/core/constants";
import {
  generateReadonlyAccount,
  generateWritableAccount,
  generateWritableSignerAccount,
} from "@/core/utils/accountMeta";
import { getCreateEscrowAccountEncoder } from "./codec";
import type {
  CreateEscrowAccountAccounts,
  CreateEscrowAccountIx,
  CreateEscrowAccountParams,
} from "./types";

export const buildCreateEscrowAccountIx = (
  params: CreateEscrowAccountParams
): CreateEscrowAccountIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getCreateEscrowAccountEncoder().encode(params.capacity);

  const accounts: CreateEscrowAccountAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateWritableSignerAccount(params.payer),
    generateReadonlyAccount(params.traderWallet),
    generateWritableAccount(params.escrowPda),
    generateReadonlyAccount(SYSTEM_PROGRAM_ADDRESS),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: CreateEscrowAccountParams) => {
  if (!params.payer) {
    throw new Error("Payer is required");
  }
  if (!params.traderWallet) {
    throw new Error("Trader wallet is required");
  }
  if (!params.escrowPda) {
    throw new Error("Escrow PDA is required");
  }
  if (params.capacity === undefined || params.capacity === null) {
    throw new Error("Capacity is required");
  }
  if (params.capacity <= 0n) {
    throw new Error("Capacity must be greater than 0");
  }
};
