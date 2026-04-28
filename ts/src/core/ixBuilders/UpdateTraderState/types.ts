import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  PerpAssetMapAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { AccountMeta } from "@solana/kit";

export interface UpdateTraderStateParams extends PhoenixInstructionAddressOverrides {
  trader: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

export type UpdateTraderStateIx = InstructionsWithAccountsAndData;

export type UpdateTraderStateAccounts = readonly AccountMeta[];
