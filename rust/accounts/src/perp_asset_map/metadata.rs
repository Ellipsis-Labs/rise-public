use bytemuck::{Pod, Zeroable};
use phoenix_rise_math::{
    BaseLots, BasisPoints, Constant, FundingRateUnitInSeconds, QuoteLotsPerBaseLotPerTick,
    SignedQuoteLotsPerBaseLot, SignedQuoteLotsPerBaseLotUpcasted, Ticks, UPnlRiskFactor,
};
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;

use super::price::PriceComponent;
use super::symbol::AssetSymbol;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const_assert_eq!(core::mem::size_of::<StaticMarketParams>(), 48);
const_assert_eq!(core::mem::size_of::<LeverageTier>(), 24);
const_assert_eq!(core::mem::size_of::<TransferFeeTier>(), 16);
const_assert_eq!(core::mem::size_of::<RiskParams>(), 200);
const_assert_eq!(core::mem::size_of::<FundingAccumulator>(), 96);
const_assert_eq!(core::mem::size_of::<OpenInterestParams>(), 16);
const_assert_eq!(core::mem::size_of::<StableIndexedShortMapMetadata>(), 8);
const_assert_eq!(core::mem::size_of::<PerpAssetMetadataLayout>(), 1568);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct StaticMarketParams {
    pub market_account: [u8; 32],
    pub tick_size: QuoteLotsPerBaseLotPerTick,
    asset_id_lower_bytes: u16,
    pub base_lot_decimals: i8,
    _padding0: u8,
    asset_id_upper_bytes: u16,
    _padding: [u8; 2],
}

impl StaticMarketParams {
    #[inline(always)]
    pub const fn asset_id(&self) -> u32 {
        self.asset_id_lower_bytes as u32 | ((self.asset_id_upper_bytes as u32) << 16)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LeverageTier {
    pub upper_bound_size: BaseLots,
    pub max_leverage: Constant,
    pub limit_order_risk_factor: BasisPoints,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct TransferFeeTier {
    pub position_size_limit: BaseLots,
    pub fee_rate: u16,
    _padding: [u8; 6],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct RiskParams {
    pub leverage_tiers: [LeverageTier; 4],
    pub upnl_risk_factor: UPnlRiskFactor,
    pub max_liquidation_size: BaseLots,
    pub transfer_fee_tiers: [TransferFeeTier; 4],
    pub risk_factors: [u16; 3],
    pub cancel_order_risk_factor: u16,
    pub upnl_risk_factor_for_withdrawals: UPnlRiskFactor,
    pub isolated_only: u8,
    pub post_only_market_price_radius_percentage: u8,
    _padding: [u8; 6],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct FundingAccumulator {
    pub acc: SignedQuoteLotsPerBaseLotUpcasted,
    pub last_diff: SignedQuoteLotsPerBaseLotUpcasted,
    pub cumulative_funding_rate: SignedQuoteLotsPerBaseLot,
    pub start_interval_timestamp: FundingRateUnitInSeconds,
    pub last_funding_update_timestamp: FundingRateUnitInSeconds,
    pub funding_interval_seconds: FundingRateUnitInSeconds,
    pub funding_period_seconds: FundingRateUnitInSeconds,
    pub max_funding_rate: SignedQuoteLotsPerBaseLot,
    pub flags: u8,
    _padding: [u8; 15],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpenInterestParams {
    pub open_interest: BaseLots,
    pub open_interest_cap: BaseLots,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AssetFlags {
    pub bits: u8,
    pub is_commodity: bool,
    pub is_commodities_reopen: bool,
    pub is_commodities_after_hours: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
struct StableIndexedShortMapMetadata {
    index_num: u16,
    is_tombstoned: u8,
    _padding: [u8; 5],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub(super) struct PerpAssetMetadataLayout {
    oracle_price: PriceComponent,
    _padding0: [u64; 2],
    static_market_params: StaticMarketParams,
    _padding1: [u64; 8],
    risk_params: RiskParams,
    _padding2: [u64; 8],
    funding_accumulator: FundingAccumulator,
    _padding3: [u64; 6],
    open_interest_params: OpenInterestParams,
    finalized_mark_price: Ticks,
    short_map_metadata: StableIndexedShortMapMetadata,
    asset_flags: u8,
    _padding_flags: [u8; 7],
    commodities_after_hours_radius: Ticks,
    last_known_index_price: Ticks,
    last_index_expiry_timestamp: u64,
    commodities_after_hours_radius_bps: BasisPoints,
    _padding4: [u64; 9],
}

/// View over one PerpAssetMap metadata slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerpAssetMetadata {
    layout: PerpAssetMetadataLayout,
}

impl PerpAssetMetadata {
    #[inline(always)]
    pub(super) fn new(layout: PerpAssetMetadataLayout) -> Self {
        Self { layout }
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(&self.layout)
    }

    #[deprecated(since = "0.1.13", note = "use as_bytes")]
    #[inline(always)]
    pub fn raw_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    #[inline(always)]
    pub fn is_tombstoned(&self) -> bool {
        self.layout.short_map_metadata.is_tombstoned != 0
    }

    #[inline(always)]
    pub fn is_active(&self) -> bool {
        !self.is_tombstoned()
    }

    #[inline(always)]
    pub fn oracle_price(&self) -> &PriceComponent {
        &self.layout.oracle_price
    }

    #[inline(always)]
    pub fn static_market_params(&self) -> &StaticMarketParams {
        &self.layout.static_market_params
    }

    #[inline(always)]
    pub fn risk_params(&self) -> &RiskParams {
        &self.layout.risk_params
    }

    #[inline(always)]
    pub fn funding_accumulator(&self) -> &FundingAccumulator {
        &self.layout.funding_accumulator
    }

    #[inline(always)]
    pub fn open_interest_params(&self) -> &OpenInterestParams {
        &self.layout.open_interest_params
    }

    #[inline(always)]
    pub fn finalized_mark_price(&self) -> Ticks {
        self.layout.finalized_mark_price
    }

    #[inline(always)]
    pub fn map_index(&self) -> u16 {
        self.layout.short_map_metadata.index_num
    }

    #[inline(always)]
    pub fn asset_flags(&self) -> AssetFlags {
        asset_flags_from_bits(self.layout.asset_flags)
    }

    #[inline(always)]
    pub fn commodities_after_hours_radius(&self) -> Ticks {
        self.layout.commodities_after_hours_radius
    }

    #[inline(always)]
    pub fn last_known_index_price(&self) -> Option<Ticks> {
        if self.layout.last_known_index_price == Ticks::ZERO {
            None
        } else {
            Some(self.layout.last_known_index_price)
        }
    }

    #[inline(always)]
    pub fn last_index_expiry_timestamp(&self) -> u64 {
        self.layout.last_index_expiry_timestamp
    }

    #[inline(always)]
    pub fn commodities_after_hours_radius_bps(&self) -> BasisPoints {
        self.layout.commodities_after_hours_radius_bps
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerpAssetMetadataEntry {
    pub symbol: AssetSymbol,
    pub metadata: PerpAssetMetadata,
}

fn asset_flags_from_bits(bits: u8) -> AssetFlags {
    AssetFlags {
        bits,
        is_commodity: bits & (1 << 0) != 0,
        is_commodities_reopen: bits & (1 << 1) != 0,
        is_commodities_after_hours: bits & (1 << 2) != 0,
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for StaticMarketParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("StaticMarketParams", 4)?;
        state.serialize_field("market_account", &pubkey_string(&self.market_account))?;
        state.serialize_field("tick_size", &self.tick_size)?;
        state.serialize_field("asset_id", &self.asset_id())?;
        state.serialize_field("base_lot_decimals", &self.base_lot_decimals)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TransferFeeTier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TransferFeeTier", 2)?;
        state.serialize_field("position_size_limit", &self.position_size_limit)?;
        state.serialize_field("fee_rate", &self.fee_rate)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for RiskParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("RiskParams", 9)?;
        state.serialize_field("leverage_tiers", &self.leverage_tiers)?;
        state.serialize_field("upnl_risk_factor", &self.upnl_risk_factor)?;
        state.serialize_field("max_liquidation_size", &self.max_liquidation_size)?;
        state.serialize_field("transfer_fee_tiers", &self.transfer_fee_tiers)?;
        state.serialize_field("risk_factors", &self.risk_factors)?;
        state.serialize_field("cancel_order_risk_factor", &self.cancel_order_risk_factor)?;
        state.serialize_field(
            "upnl_risk_factor_for_withdrawals",
            &self.upnl_risk_factor_for_withdrawals,
        )?;
        state.serialize_field("isolated_only", &self.isolated_only)?;
        state.serialize_field(
            "post_only_market_price_radius_percentage",
            &self.post_only_market_price_radius_percentage,
        )?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for FundingAccumulator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("FundingAccumulator", 9)?;
        state.serialize_field("acc", &self.acc)?;
        state.serialize_field("last_diff", &self.last_diff)?;
        state.serialize_field("cumulative_funding_rate", &self.cumulative_funding_rate)?;
        state.serialize_field("start_interval_timestamp", &self.start_interval_timestamp)?;
        state.serialize_field(
            "last_funding_update_timestamp",
            &self.last_funding_update_timestamp,
        )?;
        state.serialize_field("funding_interval_seconds", &self.funding_interval_seconds)?;
        state.serialize_field("funding_period_seconds", &self.funding_period_seconds)?;
        state.serialize_field("max_funding_rate", &self.max_funding_rate)?;
        state.serialize_field("flags", &self.flags)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PerpAssetMetadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PerpAssetMetadata", 14)?;
        state.serialize_field("map_index", &self.map_index())?;
        state.serialize_field("is_tombstoned", &self.is_tombstoned())?;
        state.serialize_field("is_active", &self.is_active())?;
        state.serialize_field("oracle_price", self.oracle_price())?;
        state.serialize_field("static_market_params", self.static_market_params())?;
        state.serialize_field("risk_params", self.risk_params())?;
        state.serialize_field("funding_accumulator", self.funding_accumulator())?;
        state.serialize_field("open_interest_params", self.open_interest_params())?;
        state.serialize_field("finalized_mark_price", &self.finalized_mark_price())?;
        state.serialize_field("asset_flags", &self.asset_flags())?;
        state.serialize_field(
            "commodities_after_hours_radius",
            &self.commodities_after_hours_radius(),
        )?;
        state.serialize_field("last_known_index_price", &self.last_known_index_price())?;
        state.serialize_field(
            "last_index_expiry_timestamp",
            &self.last_index_expiry_timestamp(),
        )?;
        state.serialize_field(
            "commodities_after_hours_radius_bps",
            &self.commodities_after_hours_radius_bps(),
        )?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PerpAssetMetadataEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PerpAssetMetadataEntry", 2)?;
        state.serialize_field("symbol", &self.symbol)?;
        state.serialize_field("metadata", &self.metadata)?;
        state.end()
    }
}
