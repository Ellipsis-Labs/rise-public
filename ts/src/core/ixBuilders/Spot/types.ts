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
import type {
  PackedVenueInstructions,
  SwapDirection,
  SwapSlippage,
} from "@/core/ixBuilders/NativeSol";

/** Accounts every spot instruction needs to locate the trader. */
interface TraderIndexAccounts {
  globalTraderIndex: GlobalTraderIndexAddressArray;
  activeTraderBuffer: ActiveTraderBufferAddressArray;
}

/** Fields every spot instruction needs to identify the asset. */
interface SpotAssetSelector {
  /** The spot asset's mint. The program resolves the asset from it. */
  mint: MintAddress;
}

export interface SyncSpotParams
  extends
    PhoenixInstructionAddressOverrides,
    TraderIndexAccounts,
    SpotAssetSelector {
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
}

/**
 * What a `WithdrawSpot` call takes out.
 *
 * Accounted collateral is margin checked and consumes its quote-notional value
 * from the exchange-wide withdrawal throttle. Uncounted excess does neither,
 * because it was never protocol collateral.
 */
export type WithdrawSpotAction =
  /** Sweep every uncounted excess token, leaving accounted collateral
   * untouched. Margin-free. */
  | { kind: "allExcess" }
  /** Withdraw `amount` base units, spending uncounted excess first and only
   * then accounted collateral. */
  | { kind: "withExcess"; amount: bigint }
  /** Withdraw `amount` base units of accounted collateral, leaving any excess
   * in place. */
  | { kind: "withoutExcess"; amount: bigint };

export interface WithdrawSpotParams
  extends
    PhoenixInstructionAddressOverrides,
    TraderIndexAccounts,
    SpotAssetSelector {
  /** The trader's wallet authority, which must sign. A position authority
   * cannot sign this instruction. */
  trader: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  /** Where the tokens go. Must be a token account of the asset's mint, and
   * must not be the custody ATA itself. */
  destination: TokenAccountAddress;
  withdrawQueue: WithdrawQueueAddress;
  action: WithdrawSpotAction;
}

export interface TransferSpotParams
  extends
    PhoenixInstructionAddressOverrides,
    TraderIndexAccounts,
    SpotAssetSelector {
  trader: Authority;
  srcTraderAccount: TraderAddress;
  dstTraderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  /** Optional permission account for secondary position authorities. */
  permissionAccount?: Address;
  /** Amount in the token's **base units**, not quote units. */
  amount: bigint;
}

export interface TransferSpotFromChildToParentParams
  extends
    PhoenixInstructionAddressOverrides,
    TraderIndexAccounts,
    SpotAssetSelector {
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

interface BaseSwapSpotParams
  extends PhoenixInstructionAddressOverrides, TraderIndexAccounts {
  /** The swap signer: either the trader's wallet or its position authority.
   * Both legs of the swap transit this key's accounts. */
  signer: Authority;
  traderAccount: TraderAddress;
  perpAssetMap: PerpAssetMapAddress;
  withdrawQueue: WithdrawQueueAddress;
  /** Amount of the *input* asset. See each builder's docs for the unit. */
  amountIn: bigint;
  /**
   * Minimum acceptable amount of the **output** asset.
   *
   * Getting the unit wrong either disables protection or makes every swap
   * fail, so this is required rather than defaulted. Pass `"unprotected"` to
   * disable the check explicitly.
   */
  minAmountOut: SwapSlippage;
  venue: PackedVenueInstructions;
}

export interface SwapSpotWithUsdcParams extends BaseSwapSpotParams {
  /** The exchange's canonical quote mint. */
  quoteMint: MintAddress;
  /** The spot asset's mint. */
  spotMint: MintAddress;
  signerQuoteTokenAccount: TokenAccountAddress;
  signerSpotTokenAccount: TokenAccountAddress;
  /** `Sell`: spot in, quote collateral out. `Buy`: quote collateral in, spot
   * out. */
  direction: SwapDirection;
}

export interface SwapSpotWithSolParams extends BaseSwapSpotParams {
  /** The spot asset's mint. */
  spotMint: MintAddress;
  signerSpotTokenAccount: TokenAccountAddress;
  /** `Sell`: spot in, native SOL out. `Buy`: native SOL in, spot out. */
  direction: SwapDirection;
}

export interface SwapSpotWithSpotParams extends BaseSwapSpotParams {
  /** The input asset's mint, debited from the trader. */
  srcMint: MintAddress;
  /** The output asset's mint, credited to the trader. */
  dstMint: MintAddress;
  /** The signer's token account for the source mint; the external venue
   * spends the withdrawn tokens from here. */
  signerSrcTokenAccount: TokenAccountAddress;
  /** The signer's token account for the destination mint. The venue must pay
   * the output here; its balance increase is moved into the destination
   * custody ATA and credited. */
  signerDstTokenAccount: TokenAccountAddress;
}

export interface LiquidateSpotParams
  extends PhoenixInstructionAddressOverrides, TraderIndexAccounts {
  /** Risk authority or delegated liquidation authority. */
  signer: Authority;
  /** Delegated permission PDA. Omit when `signer` is the risk authority. */
  permissionAccount?: Address;
  liquidateeAccount: TraderAddress;
  /** The exchange's canonical quote mint, for the global vault. */
  quoteMint: MintAddress;
  /** The seized asset's mint. */
  spotMint: MintAddress;
  perpAssetMap: PerpAssetMapAddress;
  signerQuoteTokenAccount: TokenAccountAddress;
  /** The signer's token account for the seized asset's mint; receives the
   * seized tokens so the venue instructions can spend them. */
  signerSpotTokenAccount: TokenAccountAddress;
  /** Maximum spot collateral seized, in the token's base units. */
  maxSpotAmount: bigint;
  /** Additional traders checked for the exchange-wide shortfall condition. */
  extraTraderAccounts?: TraderAddress[];
  venue: PackedVenueInstructions;
}

export type SpotIx = InstructionsWithAccountsAndData;
export type SpotAccounts = readonly AccountMeta[];
