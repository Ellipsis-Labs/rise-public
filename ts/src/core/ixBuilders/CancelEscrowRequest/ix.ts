import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getCancelEscrowRequestEncoder } from "./codec";
import type {
  CancelEscrowRequestAccounts,
  CancelEscrowRequestIx,
  CancelEscrowRequestParams,
} from "./types";

export const buildCancelEscrowRequestIx = (
  params: CancelEscrowRequestParams
): CancelEscrowRequestIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getCancelEscrowRequestEncoder().encode(params.sequenceNumber);

  const accounts: CancelEscrowRequestAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlySignerAccount(params.signerWallet),
    generateReadonlyAccount(params.receiverWallet),
    generateWritableAccount(params.receiverEscrow),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: CancelEscrowRequestParams) => {
  if (!params.signerWallet) {
    throw new Error("Signer wallet is required");
  }
  if (!params.receiverWallet) {
    throw new Error("Receiver wallet is required");
  }
  if (!params.receiverEscrow) {
    throw new Error("Receiver escrow PDA is required");
  }
  if (params.sequenceNumber === undefined || params.sequenceNumber === null) {
    throw new Error("Sequence number is required");
  }
};
