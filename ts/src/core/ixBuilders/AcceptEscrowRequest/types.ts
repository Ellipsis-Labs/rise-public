import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  PerpAssetMapAddress,
  TraderAddress,
} from "@/primitives";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface AcceptEscrowRequestParams extends PhoenixInstructionAddressOverrides {
  receiverWallet: Authority;
  receiverTraderAccount: TraderAddress;
  receiverEscrow: Address;
  senderWallet: Authority;
  senderTraderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

export type AcceptEscrowRequestIx = InstructionsWithAccountsAndData;

export type AcceptEscrowRequestAccounts = readonly AccountMeta[];
