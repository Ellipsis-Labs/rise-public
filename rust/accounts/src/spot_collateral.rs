//! Spot collateral configuration embedded in the global configuration account.
//!
//! Spot collateral is collateral held in an asset other than the canonical
//! quote token. Native SOL is the only spot asset today; the layout is
//! per-asset so SPL tokens can follow.
//!
//! The account bytes are zero on exchanges that never configured the asset,
//! which decodes as an inactive, all-zero metadata. Use
//! [`SpotCollateralMetadata::is_zeroed`] to tell "never configured" apart from
//! "configured with zero caps".
//!
//! Layout, discounts, and cap accounting mirror the on-chain
//! `program-core/exchange/src/accounts/spot_collateral.rs`.

use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::{BasisPointsU32, QuoteLots, ScalarBounds};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

/// The asset is engaged in margin.
///
/// Reading this is rarely necessary: no instruction can credit a balance while
/// the asset is inactive, so an inactive asset always means a zero balance.
pub const SPOT_COLLATERAL_FLAG_IS_ACTIVE: u8 = 1 << 0;
/// [`SpotCollateralMetadata::perp_asset_index_raw`] points at a real perp
/// asset, whose index price values this collateral.
pub const SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET: u8 = 1 << 1;
/// Exchange-wide kill switch for `SwapNative` signed by a position authority.
/// Traders carry an independent opt-out on
/// [`crate::trader::preferences::TRADER_PREFERENCE_DISABLE_POSITION_AUTHORITY_SWAP`],
/// which is a *different bit position* — do not share a constant.
pub const SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP: u8 = 1 << 2;

/// Per-asset configuration and global usage tracking for a spot collateral
/// asset.
///
/// All balances are raw `u64` in the asset's native units — lamports only when
/// the asset *is* native SOL, so no unit-bearing type applies. Discounts and
/// slippage are basis points in `0..=10_000`; the two buffers are quote lots.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SpotCollateralMetadata {
    mint_address: [u8; 32],
    decimals: u32,
    perp_asset_index: u32,
    max_per_trader_balance: u64,
    max_global_balance: u64,
    curr_global_balance: u64,
    min_margin_discount: BasisPointsU32,
    max_margin_discount: BasisPointsU32,
    max_liquidation_discount: BasisPointsU32,
    min_liquidation_slippage: BasisPointsU32,
    max_liquidation_size: u64,
    post_liquidation_buffer: QuoteLots,
    quote_lot_collateral_shortfall_buffer: QuoteLots,
    flags: u8,
    _padding_flags: [u8; 7],
    _padding: [u64; 26],
}

const_assert_eq!(core::mem::size_of::<SpotCollateralMetadata>(), 320);
const_assert_eq!(core::mem::offset_of!(SpotCollateralMetadata, decimals), 32);
const_assert_eq!(
    core::mem::offset_of!(SpotCollateralMetadata, max_per_trader_balance),
    40
);
const_assert_eq!(
    core::mem::offset_of!(SpotCollateralMetadata, min_margin_discount),
    64
);
const_assert_eq!(
    core::mem::offset_of!(SpotCollateralMetadata, max_liquidation_size),
    80
);
const_assert_eq!(core::mem::offset_of!(SpotCollateralMetadata, flags), 104);

impl SpotCollateralMetadata {
    /// Mint of the collateral asset. All-zero for native SOL.
    #[inline(always)]
    pub const fn mint_address(&self) -> [u8; 32] {
        self.mint_address
    }

    /// Native-unit decimals of the asset (9 for native SOL).
    #[inline(always)]
    pub const fn decimals(&self) -> u32 {
        self.decimals
    }

    /// Maximum balance a single trader may hold as collateral.
    #[inline(always)]
    pub const fn max_per_trader_balance(&self) -> u64 {
        self.max_per_trader_balance
    }

    /// Maximum balance held as collateral across all traders. This is also the
    /// denominator of the margin discount curve.
    #[inline(always)]
    pub const fn max_global_balance(&self) -> u64 {
        self.max_global_balance
    }

    /// Current balance held as collateral across all traders.
    #[inline(always)]
    pub const fn curr_global_balance(&self) -> u64 {
        self.curr_global_balance
    }

    /// Margin discount applied at a zero balance.
    #[inline(always)]
    pub const fn min_margin_discount(&self) -> BasisPointsU32 {
        self.min_margin_discount
    }

    /// Margin discount applied at [`Self::max_global_balance`].
    #[inline(always)]
    pub const fn max_margin_discount(&self) -> BasisPointsU32 {
        self.max_margin_discount
    }

    /// Liquidation discount applied at [`Self::max_liquidation_size`].
    #[inline(always)]
    pub const fn max_liquidation_discount(&self) -> BasisPointsU32 {
        self.max_liquidation_discount
    }

    /// Liquidation discount applied at a zero seizure size.
    #[inline(always)]
    pub const fn min_liquidation_slippage(&self) -> BasisPointsU32 {
        self.min_liquidation_slippage
    }

    /// Maximum amount seizable per liquidation call.
    #[inline(always)]
    pub const fn max_liquidation_size(&self) -> u64 {
        self.max_liquidation_size
    }

    /// Maximum quote lot collateral a liquidatee may end a spot liquidation
    /// with.
    #[inline(always)]
    pub const fn post_liquidation_buffer(&self) -> QuoteLots {
        self.post_liquidation_buffer
    }

    /// Quote buffer applied to a trader's quote lot collateral shortfall.
    #[inline(always)]
    pub const fn quote_lot_collateral_shortfall_buffer(&self) -> QuoteLots {
        self.quote_lot_collateral_shortfall_buffer
    }

    /// Raw flag bits.
    #[inline(always)]
    pub const fn flags(&self) -> u8 {
        self.flags
    }

    /// Whether the asset is engaged in margin. See
    /// [`SPOT_COLLATERAL_FLAG_IS_ACTIVE`] for why margin code does not need to
    /// consult this.
    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        self.flags & SPOT_COLLATERAL_FLAG_IS_ACTIVE != 0
    }

    /// Whether [`Self::perp_asset_index_raw`] points at a real perp asset.
    #[inline(always)]
    pub const fn has_perp_asset(&self) -> bool {
        self.flags & SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET != 0
    }

    /// Whether the exchange-wide kill switch for position-authority
    /// `SwapNative` is set.
    #[inline(always)]
    pub const fn position_authority_swap_disabled(&self) -> bool {
        self.flags & SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP != 0
    }

    /// Raw perp asset index, meaningful only when [`Self::has_perp_asset`].
    #[inline(always)]
    pub const fn perp_asset_index_raw(&self) -> u32 {
        self.perp_asset_index
    }

    /// Index of the perp asset whose index price values this collateral, or
    /// `None` when no perp asset is bound.
    #[inline(always)]
    pub const fn get_asset_id(&self) -> Option<u32> {
        if self.has_perp_asset() {
            Some(self.perp_asset_index)
        } else {
            None
        }
    }

    /// Whether every byte is zero, i.e. the asset was never configured. A
    /// configured-but-disabled asset is not zeroed.
    pub fn is_zeroed(&self) -> bool {
        bytemuck::bytes_of(self).iter().all(|byte| *byte == 0)
    }

    /// Whether every discount and slippage field is within `0..=10_000`.
    ///
    /// The program validates this on write, and `bytemuck` casts do not
    /// re-check it, so this is how a reader tells a real configuration from
    /// corrupt bytes.
    pub fn discounts_in_bounds(&self) -> bool {
        self.min_margin_discount.is_in_bounds()
            && self.max_margin_discount.is_in_bounds()
            && self.max_liquidation_discount.is_in_bounds()
            && self.min_liquidation_slippage.is_in_bounds()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for SpotCollateralMetadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("SpotCollateralMetadata", 17)?;
        state.serialize_field("mint_address", &pubkey_string(&self.mint_address()))?;
        state.serialize_field("decimals", &self.decimals())?;
        state.serialize_field("perp_asset_index", &self.perp_asset_index_raw())?;
        state.serialize_field("max_per_trader_balance", &self.max_per_trader_balance())?;
        state.serialize_field("max_global_balance", &self.max_global_balance())?;
        state.serialize_field("curr_global_balance", &self.curr_global_balance())?;
        state.serialize_field("min_margin_discount_bps", &self.min_margin_discount())?;
        state.serialize_field("max_margin_discount_bps", &self.max_margin_discount())?;
        state.serialize_field(
            "max_liquidation_discount_bps",
            &self.max_liquidation_discount(),
        )?;
        state.serialize_field(
            "min_liquidation_slippage_bps",
            &self.min_liquidation_slippage(),
        )?;
        state.serialize_field("max_liquidation_size", &self.max_liquidation_size())?;
        state.serialize_field("post_liquidation_buffer", &self.post_liquidation_buffer())?;
        state.serialize_field(
            "quote_lot_collateral_shortfall_buffer",
            &self.quote_lot_collateral_shortfall_buffer(),
        )?;
        state.serialize_field("flags", &self.flags())?;
        state.serialize_field("is_active", &self.is_active())?;
        state.serialize_field("has_perp_asset", &self.has_perp_asset())?;
        state.serialize_field(
            "position_authority_swap_disabled",
            &self.position_authority_swap_disabled(),
        )?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata_with_flags(flags: u8) -> SpotCollateralMetadata {
        let mut metadata = SpotCollateralMetadata::zeroed();
        metadata.flags = flags;
        metadata
    }

    #[test]
    fn flag_accessors_decode_independently() {
        let metadata = metadata_with_flags(
            SPOT_COLLATERAL_FLAG_IS_ACTIVE | SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP,
        );

        assert!(metadata.is_active());
        assert!(!metadata.has_perp_asset());
        assert!(metadata.position_authority_swap_disabled());
    }

    #[test]
    fn get_asset_id_requires_the_has_perp_asset_flag() {
        let mut metadata = metadata_with_flags(SPOT_COLLATERAL_FLAG_IS_ACTIVE);
        metadata.perp_asset_index = 7;
        assert_eq!(metadata.get_asset_id(), None);

        metadata.flags |= SPOT_COLLATERAL_FLAG_HAS_PERP_ASSET;
        assert_eq!(metadata.get_asset_id(), Some(7));
    }

    /// `bytemuck` casts do not validate bounded types, so a corrupt account
    /// can carry an out-of-range discount. The reader has to be able to see it.
    #[test]
    fn discounts_in_bounds_flags_out_of_range_basis_points() {
        let mut metadata = SpotCollateralMetadata::zeroed();
        metadata.min_margin_discount = BasisPointsU32::new(500);
        metadata.max_margin_discount = BasisPointsU32::new(10_000);
        assert!(metadata.discounts_in_bounds());

        metadata.max_margin_discount = BasisPointsU32::new(10_001);
        assert!(!metadata.discounts_in_bounds());
    }

    #[test]
    fn basis_points_upcast_reaches_the_math_compute_type() {
        assert_eq!(
            BasisPointsU32::new(9_500).upcast(),
            phoenix_rise_math::BasisPoints::new(9_500)
        );
    }

    #[test]
    fn is_zeroed_distinguishes_unconfigured_from_disabled() {
        assert!(SpotCollateralMetadata::zeroed().is_zeroed());

        let mut configured = SpotCollateralMetadata::zeroed();
        configured.max_global_balance = 1;
        assert!(!configured.is_zeroed());
    }
}
