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

export type EscrowAction = { kind: "noop" } | { kind: "cash"; amount: bigint };

export interface CreateEscrowRequestParams extends PhoenixInstructionAddressOverrides {
  senderWallet: Authority;
  senderTraderAccount: TraderAddress;
  permissionAccount: Address;
  receiverWallet: Authority;
  receiverTraderAccount: TraderAddress;
  receiverEscrow: Address;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  senderPdaIndex: number;
  senderSubaccountIndex: number;
  receiverPdaIndex: number;
  receiverSubaccountIndex: number;
  actions: EscrowAction[];
  lastValidSlot?: bigint | null;
}

export type CreateEscrowRequestIx = InstructionsWithAccountsAndData;

export type CreateEscrowRequestAccounts = readonly AccountMeta[];
