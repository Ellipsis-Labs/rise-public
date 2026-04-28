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

export interface TransferCollateralParams extends PhoenixInstructionAddressOverrides {
  trader: Authority; // Trader wallet (signer)
  srcTraderAccount: TraderAddress; // Source trader account
  dstTraderAccount: TraderAddress; // Destination trader account
  perpAssetMap: PerpAssetMapAddress; // Perp asset map account
  globalTraderIndex: GlobalTraderIndexAddressArray; // Array of global trader index addresses (header, then N arenas)
  activeTraderBuffer: ActiveTraderBufferAddressArray; // Array of active trader buffer addresses (header, then N arenas)
  amount: bigint; // Amount to transfer
}

export type TransferCollateralIx = InstructionsWithAccountsAndData;

export type TransferCollateralAccounts = readonly AccountMeta[];
