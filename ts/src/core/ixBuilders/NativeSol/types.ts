import type {
  ActiveTraderBufferAddressArray,
  Authority,
  GlobalTraderIndexAddressArray,
  MintAddress,
  PerpAssetMapAddress,
  TokenAccountAddress,
  TraderAddress,
  WithdrawQueueAddress,
} from "@/primitives";
import type { InstructionsWithAccountsAndData } from "@/primitives/_utilityTypes";
import type { PhoenixInstructionAddressOverrides } from "@/core/constants";
import type { AccountMeta, Address } from "@solana/kit";

/** Accounts every native SOL instruction needs to locate the trader. */
interface TraderIndexAccounts {
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

export interface SyncNativeParams
  extends PhoenixInstructionAddressOverrides, TraderIndexAccounts {
  traderAccount: TraderAddress;
}

/**
 * What a `WithdrawNativeSol` call takes out.
 *
 * Accounted collateral is margin checked and consumes its quote-notional value
 * from the exchange-wide withdrawal throttle. Uncounted excess does neither,
 * because it was never protocol collateral.
 */
export type WithdrawNativeSolAction =
  /** Sweep every uncounted excess lamport, leaving accounted collateral and the
   * rent floor untouched. Margin-free. */
  | { kind: "allExcess" }
  /** Withdraw `lamports`, spending uncounted excess first and only then
   * accounted collateral. */
  | { kind: "withExcess"; lamports: bigint }
  /** Withdraw `lamports` of accounted collateral, leaving any excess in
   * place. */
  | { kind: "withoutExcess"; lamports: bigint };

export interface WithdrawNativeSolParams
  extends PhoenixInstructionAddressOverrides, TraderIndexAccounts {
  /** The trader's wallet authority, which must sign. A position authority
   * cannot sign this instruction. */
  trader: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  /** Where the lamports go. Must be a system-owned account, and must be
   * neither the trader account nor the native SOL authority PDA. */
  destination: Address;
  withdrawQueue: WithdrawQueueAddress;
  action: WithdrawNativeSolAction;
}

export interface TransferNativeSolParams
  extends PhoenixInstructionAddressOverrides, TraderIndexAccounts {
  trader: Authority;
  srcTraderAccount: TraderAddress;
  dstTraderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  /** Optional permission account for secondary position authorities. */
  permissionAccount?: Address;
  /** Amount in **lamports**, not quote units. */
  lamports: bigint;
}

export interface TransferNativeSolFromChildToParentParams
  extends PhoenixInstructionAddressOverrides, TraderIndexAccounts {
  trader: Authority;
  childTraderAccount: TraderAddress;
  parentTraderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  /**
   * Whether the trader wallet signs. Defaults to `true`, matching the quote
   * collateral sweep builder.
   *
   * The sweep is permissionless unless the child opted out with the
   * `disableCollateralSweep` preference, so a crank that does not hold the
   * wallet key should set this to `false`.
   */
  traderSigns?: boolean;
}

/** Which way a `SwapNative` call trades. */
export enum SwapDirection {
  /** Native SOL in, quote collateral out. */
  Sell = 0,
  /** Quote collateral in, native SOL out. */
  Buy = 1,
}

/**
 * An account reference inside a packed venue instruction: an index into the
 * deduplicated account list plus its signer/writable bits.
 */
export interface PackedAccountMeta {
  index: number;
  isSigner: boolean;
  isWritable: boolean;
}

/** One venue instruction, with its accounts addressed by index. */
export interface PackedInstruction {
  programIdIndex: number;
  data: Uint8Array;
  accountMetas: PackedAccountMeta[];
}

/**
 * Venue instructions packed for `buildSwapNativeIx`, together with the
 * deduplicated account list they index into.
 *
 * The two halves must stay together — an index list is meaningless against a
 * different account list — so `packVenueInstructions` returns them as one
 * value.
 */
export interface PackedVenueInstructions {
  instructions: PackedInstruction[];
  accounts: AccountMeta[];
}

export interface LiquidateNativeSolParams
  extends PhoenixInstructionAddressOverrides, TraderIndexAccounts {
  /** Risk authority or delegated liquidation authority. */
  signer: Authority;
  /** Delegated permission PDA. Omit when `signer` is the risk authority. */
  permissionAccount?: Address;
  liquidateeAccount: TraderAddress;
  mint: MintAddress;
  perpAssetMap: PerpAssetMapAddress;
  signerQuoteTokenAccount: TokenAccountAddress;
  withdrawQueue: WithdrawQueueAddress;
  /** Maximum native SOL seized, in lamports. */
  maxNativeSolAmount: bigint;
  /** Additional traders checked for the exchange-wide shortfall condition. */
  extraTraderAccounts?: TraderAddress[];
  venue: PackedVenueInstructions;
}

/**
 * Slippage protection for a swap.
 *
 * `"unprotected"` disables the check entirely: the swap then executes at
 * whatever price the venue returns, and the program applies no oracle floor.
 * Only use it when some other mechanism bounds the price.
 */
export type SwapSlippage = bigint | "unprotected";

export interface SwapNativeParams
  extends PhoenixInstructionAddressOverrides, TraderIndexAccounts {
  /** The swap signer: either the trader's wallet or its position authority.
   * Both legs of the swap transit this key's accounts. */
  signer: Authority;
  traderAccount: TraderAddress;
  /** The exchange's canonical quote mint. */
  mint: MintAddress;
  perpAssetMap: PerpAssetMapAddress;
  signerQuoteTokenAccount: TokenAccountAddress;
  withdrawQueue: WithdrawQueueAddress;
  direction: SwapDirection;
  /** Amount of the *input* asset: lamports for a sell, quote lots for a buy. */
  amountIn: bigint;
  /**
   * Minimum acceptable amount of the **output** asset — quote lots for a
   * `Sell`, lamports for a `Buy`.
   *
   * Getting the unit wrong either disables protection or makes every swap
   * fail, so this is required rather than defaulted.
   */
  minAmountOut: SwapSlippage;
  venue: PackedVenueInstructions;
}

export type NativeSolIx = InstructionsWithAccountsAndData;
export type NativeSolAccounts = readonly AccountMeta[];
