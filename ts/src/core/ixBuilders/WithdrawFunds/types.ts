import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  GlobalVaultAddress,
  MintAddress,
  PerpAssetMapAddress,
  TokenAccountAddress,
  TraderAddress,
  WithdrawQueueAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { AccountMeta } from "@solana/kit";

export interface WithdrawFundsParams extends PhoenixInstructionAddressOverrides {
  trader: Authority; // Trader wallet (signer)
  traderAccount: TraderAddress; // Trader account
  mint: MintAddress; // Token mint address
  perpAssetMap: PerpAssetMapAddress; // Perp asset map account
  destinationTokenAccount: TokenAccountAddress; // Destination token account for withdrawal
  globalVault: GlobalVaultAddress;
  withdrawQueue: WithdrawQueueAddress;
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
  amount: bigint; // Amount to withdraw
}

export type WithdrawFundsIx = InstructionsWithAccountsAndData;

export type WithdrawFundsAccounts = readonly AccountMeta[];
