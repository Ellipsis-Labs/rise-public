import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  PerpAssetMapAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { AccountMeta } from "@solana/kit";

export interface TransferCollateralChildToParentParams extends PhoenixInstructionAddressOverrides {
  trader: Authority; // Trader wallet (signer)
  childTraderAccount: TraderAddress; // Child (isolated) trader subaccount
  parentTraderAccount: TraderAddress; // Parent (cross) trader account
  perpAssetMap: PerpAssetMapAddress; // Perp asset map account
  globalTraderIndex: GlobalTraderIndexAddressArray; // Array of global trader index addresses (header, then N arenas)
  activeTraderBuffer: ActiveTraderBufferAddressArray; // Array of active trader buffer addresses (header, then N arenas)
}

export type TransferCollateralChildToParentIx = InstructionsWithAccountsAndData;

export type TransferCollateralChildToParentAccounts = readonly AccountMeta[];
