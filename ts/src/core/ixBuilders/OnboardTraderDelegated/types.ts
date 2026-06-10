import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta, Address } from "@solana/kit";

export interface OnboardTraderDelegatedParams extends PhoenixInstructionAddressOverrides {
  authority: Authority;
  permissionAccount: Address;
  traderAccount: TraderAddress;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  globalTraderIndex: GlobalTraderIndexAddressArray;
}

export type OnboardTraderDelegatedIx = InstructionsWithAccountsAndData;

export type OnboardTraderDelegatedAccounts = readonly AccountMeta[];
