import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  GlobalVaultAddress,
  MintAddress,
  TokenAccountAddress,
  TraderAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { AccountMeta, Address } from "@solana/kit";

export interface DepositFundsParams extends PhoenixInstructionAddressOverrides {
  trader: Authority; // Trader wallet (signer)
  mint: MintAddress; // Token mint address
  traderAccount: TraderAddress; // Trader account
  traderTokenAccount: TokenAccountAddress; // Trader's token account (optional, will derive if not provided)
  globalVault: GlobalVaultAddress; // Global vault address for the mint
  globalTraderIndex: GlobalTraderIndexAddressArray; // Array of global trader index addresses (header, then N arenas)
  activeTraderBuffer: ActiveTraderBufferAddressArray; // Array of active trader buffer addresses (header, then N arenas)
  amount: bigint; // Amount to deposit
  permissionAccount?: Address; // Permission account address (optional, if the trader authority is delegated by the actual authority)
}

export type DepositFundsIx = InstructionsWithAccountsAndData;

export type DepositFundsAccounts = readonly AccountMeta[];
