import type { MintAddress } from "@/primitives/_addressTypes";
import type { QuoteLots } from "@/primitives/_numberTypes";

/** The asset is engaged in margin. This is only half the gate — the exchange
 * feature bit must be set too. See `isNativeSolCollateralActive`. */
export const SPOT_COLLATERAL_FLAG_IS_ACTIVE: number = 1 << 0;
/** `perpAssetIndex` points at a real perp asset, whose index price values this
 * collateral. */
export const SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET: number = 1 << 1;
/** Exchange-wide kill switch for `SwapNative` signed by a position authority.
 * Traders carry an independent opt-out on trader preference bit 1, which is a
 * *different bit position* — do not share a constant. */
export const SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP: number =
  1 << 2;

/**
 * Per-asset configuration and global usage tracking for a spot collateral
 * asset (native SOL today), embedded in the global configuration account.
 *
 * Balances are in the asset's native units (lamports for native SOL),
 * discounts and slippage are basis points in `0..=10_000`, and the two buffers
 * are quote lots.
 */
export interface SpotCollateralMetadata {
  /** Mint of the collateral asset; all-zero for native SOL. */
  mintAddress: MintAddress;
  /** Native-unit decimals of the asset (9 for native SOL). */
  decimals: number;
  /** Perp asset whose index price values this collateral. Meaningful only when
   * `hasPerpAsset` is set — prefer `getSpotCollateralAssetId`. */
  perpAssetIndex: number;
  /** Maximum balance a single trader may hold, in native units. */
  maxPerTraderBalance: bigint;
  /** Maximum balance across all traders, in native units. Also the denominator
   * of the margin discount curve. */
  maxGlobalBalance: bigint;
  /** Current balance across all traders, in native units. */
  currGlobalBalance: bigint;
  /** Margin discount at a zero balance, in basis points. */
  minMarginDiscountBps: number;
  /** Margin discount at `maxGlobalBalance`, in basis points. */
  maxMarginDiscountBps: number;
  /** Liquidation discount at `maxLiquidationSize`, in basis points. */
  maxLiquidationDiscountBps: number;
  /** Liquidation discount at a zero seizure size, in basis points. */
  minLiquidationSlippageBps: number;
  /** Maximum amount seizable per liquidation call, in native units. */
  maxLiquidationSize: bigint;
  /** Maximum quote lot collateral a liquidatee may end a spot liquidation
   * with. */
  postLiquidationBuffer: QuoteLots;
  /** Quote buffer applied to a trader's quote lot collateral shortfall. */
  quoteLotCollateralShortfallBuffer: QuoteLots;
  /** Raw flag bits. */
  flags: number;
  isActive: boolean;
  hasPerpAsset: boolean;
  positionAuthoritySwapDisabled: boolean;
}

/**
 * Index of the perp asset whose index price values this collateral, or `null`
 * when no perp asset is bound.
 */
export const getSpotCollateralAssetId = (
  metadata: SpotCollateralMetadata
): number | null => (metadata.hasPerpAsset ? metadata.perpAssetIndex : null);
