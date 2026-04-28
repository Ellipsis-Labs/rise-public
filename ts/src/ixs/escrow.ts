import { phoenixInstructionAddresses } from "@/core/constants";
import {
  buildCreateEscrowRequestIx,
  type CreateEscrowRequestIx,
} from "@/core/ixBuilders/CreateEscrowRequest";
import type { ResolvedCreateEscrowRequestIxInput } from "./types";

export const buildCreateEscrowRequestIxResolved = (
  params: ResolvedCreateEscrowRequestIxInput
): CreateEscrowRequestIx =>
  buildCreateEscrowRequestIx({
    ...phoenixInstructionAddresses({
      phoenixProgramAddress: params.exchange.phoenixProgramAddress,
      logAuthorityAddress: params.exchange.logAuthorityAddress,
      globalConfigurationAddress: params.exchange.globalConfigurationAddress,
    }),
    senderWallet: params.sender.authority,
    senderTraderAccount: params.sender.traderAccount,
    permissionAccount: params.receiver.permissionAddress,
    receiverWallet: params.receiver.authority,
    receiverTraderAccount: params.receiver.traderAccount,
    receiverEscrow: params.receiver.escrowAddress,
    perpAssetMap: params.exchange.perpAssetMap,
    globalTraderIndex: params.exchange.globalTraderIndex,
    activeTraderBuffer: params.exchange.activeTraderBuffer,
    senderPdaIndex: params.sender.traderPdaIndex,
    senderSubaccountIndex: params.sender.traderSubaccountIndex,
    receiverPdaIndex: params.receiver.traderPdaIndex,
    receiverSubaccountIndex: params.receiver.traderSubaccountIndex,
    actions: params.actions,
    lastValidSlot: params.lastValidSlot,
  });
