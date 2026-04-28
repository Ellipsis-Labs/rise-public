import { getPhoenixInstructionAddresses } from "@/core/constants";
import {
  generateArenaAccounts,
  generateReadonlyAccount,
  generateReadonlySignerAccount,
  generateWritableAccount,
} from "@/core/utils/accountMeta";
import { getAcceptEscrowRequestEncoder } from "./codec";
import type {
  AcceptEscrowRequestAccounts,
  AcceptEscrowRequestIx,
  AcceptEscrowRequestParams,
} from "./types";

export const buildAcceptEscrowRequestIx = (
  params: AcceptEscrowRequestParams
): AcceptEscrowRequestIx => {
  validate(params);
  const { programAddress, logAuthorityAddress, globalConfigurationAddress } =
    getPhoenixInstructionAddresses(params);

  const data = getAcceptEscrowRequestEncoder().encode(undefined);

  const accounts: AcceptEscrowRequestAccounts = [
    generateReadonlyAccount(programAddress),
    generateReadonlyAccount(logAuthorityAddress),
    generateReadonlyAccount(globalConfigurationAddress),
    generateReadonlyAccount(params.senderWallet),
    generateWritableAccount(params.senderTraderAccount),
    generateReadonlySignerAccount(params.receiverWallet),
    generateWritableAccount(params.receiverTraderAccount),
    generateWritableAccount(params.receiverEscrow),
    generateReadonlyAccount(params.perpAssetMap),
    ...generateArenaAccounts(params.globalTraderIndex),
    ...generateArenaAccounts(params.activeTraderBuffer),
  ] as const;

  return {
    programAddress,
    accounts,
    data,
  };
};

const validate = (params: AcceptEscrowRequestParams) => {
  if (!params.receiverWallet) {
    throw new Error("Receiver wallet is required");
  }
  if (!params.receiverTraderAccount) {
    throw new Error("Receiver trader account is required");
  }
  if (!params.receiverEscrow) {
    throw new Error("Receiver escrow PDA is required");
  }
  if (!params.senderWallet) {
    throw new Error("Sender wallet is required");
  }
  if (!params.senderTraderAccount) {
    throw new Error("Sender trader account is required");
  }
  if (!params.perpAssetMap) {
    throw new Error("Perp asset map is required");
  }
  if (!params.globalTraderIndex || params.globalTraderIndex.length === 0) {
    throw new Error(
      "Global trader index array is required and must not be empty"
    );
  }
  if (!params.activeTraderBuffer || params.activeTraderBuffer.length === 0) {
    throw new Error(
      "Active trader buffer array is required and must not be empty"
    );
  }
};
