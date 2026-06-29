//! Borrowed views for Phoenix perp asset map accounts.

use bytemuck::{Pod, Zeroable};
#[cfg(feature = "serde")]
use serde::ser::{SerializeSeq, SerializeStruct};

use super::common::{
    PhoenixAccountDecodeError, SequenceNumber, none_if_zero_u64, read_pod, read_prefix,
    require_len, verify_discriminant,
};
use super::discriminants::PhoenixAccount;
#[cfg(feature = "serde")]
use crate::serde_helpers::pubkey_string;

const PERP_ASSET_MAP_ACCOUNT: &str = "PerpAssetMap";
const MAX_NUMBER_OF_PERP_ASSETS: usize = 1024;
const MAX_ORACLES: usize = 5;
const PERP_ASSET_MAP_HEADER_LEN: usize = core::mem::size_of::<PerpAssetMapHeader>();
const PERP_ASSET_MAP_ENTRY_LEN: usize = core::mem::size_of::<PerpAssetMapEntry>();
const PERP_ASSET_MAP_LEN: usize =
    PERP_ASSET_MAP_HEADER_LEN + MAX_NUMBER_OF_PERP_ASSETS * PERP_ASSET_MAP_ENTRY_LEN;

const_assert_eq!(core::mem::size_of::<TicksAtSlot>(), 16);
const_assert_eq!(core::mem::size_of::<OracleData>(), 40);
const_assert_eq!(core::mem::size_of::<BookPriceComponent>(), 88);
const_assert_eq!(core::mem::size_of::<PerpPriceComponent>(), 96);
const_assert_eq!(core::mem::size_of::<SpotPriceComponent>(), 136);
const_assert_eq!(core::mem::size_of::<MarkPrice>(), 872);
const_assert_eq!(core::mem::size_of::<PriceComponent>(), 888);
const_assert_eq!(core::mem::size_of::<StaticMarketParams>(), 48);
const_assert_eq!(core::mem::size_of::<LeverageTier>(), 24);
const_assert_eq!(core::mem::size_of::<TransferFeeTier>(), 16);
const_assert_eq!(core::mem::size_of::<RiskParams>(), 200);
const_assert_eq!(core::mem::size_of::<FundingAccumulator>(), 96);
const_assert_eq!(core::mem::size_of::<OpenInterestParams>(), 16);
const_assert_eq!(core::mem::size_of::<StableIndexedShortMapHeader>(), 16);
const_assert_eq!(core::mem::size_of::<PerpAssetMapHeader>(), 48);
const_assert_eq!(core::mem::size_of::<StableIndexedShortMapMetadata>(), 8);
const_assert_eq!(core::mem::size_of::<PerpAssetMetadataLayout>(), 1568);
const_assert_eq!(core::mem::size_of::<PerpAssetMapEntry>(), 1584);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetSymbol {
    bytes: [u8; 16],
    len: u8,
}

impl AssetSymbol {
    pub(crate) fn try_from_bytes(bytes: [u8; 16]) -> Result<Self, PhoenixAccountDecodeError> {
        Self::try_from_bytes_for_account(bytes, PERP_ASSET_MAP_ACCOUNT)
    }

    pub(crate) fn try_from_bytes_for_account(
        bytes: [u8; 16],
        account: &'static str,
    ) -> Result<Self, PhoenixAccountDecodeError> {
        let len = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        if !bytes[..len].iter().all(u8::is_ascii) {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account,
                reason: "asset symbol contains non-ASCII bytes",
            });
        }
        Ok(Self {
            bytes,
            len: len as u8,
        })
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes())
            .expect("AssetSymbol is validated as ASCII during decode")
    }

    #[inline(always)]
    pub fn matches(&self, symbol: &str) -> bool {
        self.as_bytes() == symbol.as_bytes()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TicksAtSlot {
    pub slot: u64,
    pub ticks: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct OracleData {
    pub oracle_pubkey: [u8; 32],
    pub divergence_score: u8,
    _padding: [u8; 7],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OracleParameters {
    pub oracle_divergence_radius: u16,
    pub min_oracle_responses: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BookPriceComponent {
    pub clamped_book_mid_price: TicksAtSlot,
    pub weight: u64,
    pub stale_threshold: u64,
    pub book_price_radius: u64,
    pub last_best_bid: TicksAtSlot,
    pub last_best_ask: TicksAtSlot,
    pub last_trade_price: TicksAtSlot,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PerpPriceComponent {
    pub last_exchange_perp_price: [TicksAtSlot; MAX_ORACLES],
    pub weight: u64,
    pub stale_threshold: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpotPriceComponent {
    pub last_exchange_spot_price: [TicksAtSlot; MAX_ORACLES],
    pub weight: u64,
    pub stale_threshold: u64,
    pub slot: u64,
    pub mid_spot_diff_ema_ticks: i64,
    pub mid_spot_diff_ema_ticks_dust: i64,
    pub ema_period_slots: u64,
    pub ema_diff_radius: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct MarkPrice {
    pub price: TicksAtSlot,
    pub spot_price_component: SpotPriceComponent,
    pub perp_price_component: PerpPriceComponent,
    pub book_price_component: BookPriceComponent,
    pub risk_action_price_validity_rules: [u8; 256],
    pub oracle_data: [OracleData; MAX_ORACLES],
    pub oracle_divergence_radius: u16,
    pub min_oracle_responses: u8,
    pub book_hard_stale_multiplier: u8,
    pub oracle_hard_stale_multiplier: u8,
    _padding: [u8; 3],
    pub mark_price_last_validated_slot: [u64; 4],
    pub oracle_last_updated_timestamps: [u64; MAX_ORACLES],
}

impl MarkPrice {
    #[inline(always)]
    pub const fn oracle_parameters(&self) -> OracleParameters {
        OracleParameters {
            oracle_divergence_radius: self.oracle_divergence_radius,
            min_oracle_responses: self.min_oracle_responses,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PriceComponent {
    pub price_sequence_number: SequenceNumber,
    pub mark_price: MarkPrice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct StaticMarketParams {
    pub market_account: [u8; 32],
    pub tick_size: u64,
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
    pub upper_bound_size: u64,
    pub max_leverage: u64,
    pub limit_order_risk_factor: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct TransferFeeTier {
    pub position_size_limit: u64,
    pub fee_rate: u16,
    _padding: [u8; 6],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct RiskParams {
    pub leverage_tiers: [LeverageTier; 4],
    pub upnl_risk_factor: u64,
    pub max_liquidation_size: u64,
    pub transfer_fee_tiers: [TransferFeeTier; 4],
    pub risk_factors: [u16; 3],
    pub cancel_order_risk_factor: u16,
    pub upnl_risk_factor_for_withdrawals: u64,
    pub isolated_only: u8,
    pub post_only_market_price_radius_percentage: u8,
    _padding: [u8; 6],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct FundingAccumulator {
    pub acc: i128,
    pub last_diff: i128,
    pub cumulative_funding_rate: i64,
    pub start_interval_timestamp: u64,
    pub last_funding_update_timestamp: u64,
    pub funding_interval_seconds: u64,
    pub funding_period_seconds: u64,
    pub max_funding_rate: i64,
    pub flags: u8,
    _padding: [u8; 15],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OpenInterestParams {
    pub open_interest: u64,
    pub open_interest_cap: u64,
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
struct StableIndexedShortMapHeader {
    slots_used: u32,
    tombstones: u32,
    capacity: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
struct PerpAssetMapHeader {
    discriminant: u64,
    sequence_number: SequenceNumber,
    num_assets: u16,
    _padding0: [u8; 6],
    metadata_header: StableIndexedShortMapHeader,
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
struct PerpAssetMetadataLayout {
    oracle_price: PriceComponent,
    _padding0: [u64; 2],
    static_market_params: StaticMarketParams,
    _padding1: [u64; 8],
    risk_params: RiskParams,
    _padding2: [u64; 8],
    funding_accumulator: FundingAccumulator,
    _padding3: [u64; 6],
    open_interest_params: OpenInterestParams,
    finalized_mark_price: u64,
    short_map_metadata: StableIndexedShortMapMetadata,
    asset_flags: u8,
    _padding_flags: [u8; 7],
    commodities_after_hours_radius: u64,
    last_known_index_price: u64,
    last_index_expiry_timestamp: u64,
    commodities_after_hours_radius_bps: u64,
    _padding4: [u64; 9],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
struct PerpAssetMapEntry {
    symbol: [u8; 16],
    metadata: PerpAssetMetadataLayout,
}

/// View over one PerpAssetMap metadata slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerpAssetMetadata {
    layout: PerpAssetMetadataLayout,
}

impl PerpAssetMetadata {
    #[inline(always)]
    fn new(layout: PerpAssetMetadataLayout) -> Self {
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
    pub fn finalized_mark_price(&self) -> u64 {
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
    pub fn commodities_after_hours_radius(&self) -> u64 {
        self.layout.commodities_after_hours_radius
    }

    #[inline(always)]
    pub fn last_known_index_price(&self) -> Option<u64> {
        none_if_zero_u64(self.layout.last_known_index_price)
    }

    #[inline(always)]
    pub fn last_index_expiry_timestamp(&self) -> u64 {
        self.layout.last_index_expiry_timestamp
    }

    #[inline(always)]
    pub fn commodities_after_hours_radius_bps(&self) -> u64 {
        self.layout.commodities_after_hours_radius_bps
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerpAssetMetadataEntry {
    pub symbol: AssetSymbol,
    pub metadata: PerpAssetMetadata,
}

/// Borrowed view over a Phoenix PerpAssetMap account.
///
/// This is the main account view for market metadata. It exposes active and
/// tombstoned assets, mark price inputs, funding accumulators, risk parameters,
/// open-interest caps, and static market params without allocating owned
/// metadata for every slot.
#[derive(Clone, Copy, Debug)]
pub struct PerpAssetMap<'a> {
    data: &'a [u8],
    sequence_number: SequenceNumber,
    num_assets: u16,
    slots_used: u32,
    tombstones: u32,
    capacity: u64,
}

impl<'a> PerpAssetMap<'a> {
    /// Decode a PerpAssetMap from raw account data.
    ///
    /// The returned view borrows `data`, verifies the account discriminant and
    /// fixed account size, and reads layout fields with unaligned-safe copies.
    pub fn try_from_account_bytes(data: &'a [u8]) -> Result<Self, PhoenixAccountDecodeError> {
        require_len(PERP_ASSET_MAP_ACCOUNT, data, PERP_ASSET_MAP_LEN)?;
        verify_discriminant(
            PERP_ASSET_MAP_ACCOUNT,
            data,
            PhoenixAccount::PerpAssetMap.discriminant(),
        )?;
        let value = read_prefix::<PerpAssetMapHeader>(PERP_ASSET_MAP_ACCOUNT, data)?;
        let slots_used = value.metadata_header.slots_used;
        let tombstones = value.metadata_header.tombstones;
        let capacity = value.metadata_header.capacity;
        if slots_used as usize > MAX_NUMBER_OF_PERP_ASSETS {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: PERP_ASSET_MAP_ACCOUNT,
                reason: "metadata slots_used exceeds capacity",
            });
        }
        if tombstones > slots_used {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account: PERP_ASSET_MAP_ACCOUNT,
                reason: "metadata tombstones exceeds slots_used",
            });
        }
        Ok(Self {
            data,
            sequence_number: value.sequence_number,
            num_assets: value.num_assets,
            slots_used,
            tombstones,
            capacity,
        })
    }

    #[inline(always)]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    #[inline(always)]
    pub const fn num_assets(&self) -> u16 {
        self.num_assets
    }

    #[inline(always)]
    pub const fn slots_used(&self) -> u32 {
        self.slots_used
    }

    #[inline(always)]
    pub const fn tombstones(&self) -> u32 {
        self.tombstones
    }

    #[inline(always)]
    pub const fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Iterate metadata slots in account order.
    ///
    /// Iteration includes tombstoned entries so callers can preserve map
    /// indexes. Check [`PerpAssetMetadata::is_active`] when you only want
    /// tradable markets.
    #[inline(always)]
    pub fn iter(&self) -> PerpAssetMapIter<'a> {
        let entries_start = PERP_ASSET_MAP_HEADER_LEN;
        let entries_len = self.slots_used as usize * PERP_ASSET_MAP_ENTRY_LEN;
        PerpAssetMapIter {
            remaining: &self.data[entries_start..entries_start + entries_len],
        }
    }

    /// Find the first metadata entry whose symbol bytes match `symbol`.
    ///
    /// This returns tombstoned entries too; inspect
    /// [`PerpAssetMetadata::is_active`] if the caller requires an active
    /// market.
    pub fn find_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Option<PerpAssetMetadataEntry>, PhoenixAccountDecodeError> {
        for entry in self.iter() {
            let entry = entry?;
            if entry.symbol.matches(symbol) {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }
}

pub struct PerpAssetMapIter<'a> {
    remaining: &'a [u8],
}

impl<'a> Iterator for PerpAssetMapIter<'a> {
    type Item = Result<PerpAssetMetadataEntry, PhoenixAccountDecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.remaining.is_empty() {
            if self.remaining.len() < PERP_ASSET_MAP_ENTRY_LEN {
                let actual = self.remaining.len();
                self.remaining = &[];
                return Some(Err(PhoenixAccountDecodeError::AccountTooSmall {
                    account: PERP_ASSET_MAP_ACCOUNT,
                    expected: PERP_ASSET_MAP_ENTRY_LEN,
                    actual,
                }));
            }

            let (entry_bytes, remaining) = self.remaining.split_at(PERP_ASSET_MAP_ENTRY_LEN);
            self.remaining = remaining;

            let value = match read_pod::<PerpAssetMapEntry>(PERP_ASSET_MAP_ACCOUNT, entry_bytes) {
                Ok(value) => value,
                Err(_) => {
                    return Some(Err(PhoenixAccountDecodeError::InvalidLayout {
                        account: PERP_ASSET_MAP_ACCOUNT,
                    }));
                }
            };
            let symbol = match AssetSymbol::try_from_bytes(value.symbol) {
                Ok(symbol) => symbol,
                Err(error) => return Some(Err(error)),
            };
            let metadata = PerpAssetMetadata::new(value.metadata);
            if metadata.is_active() {
                return Some(Ok(PerpAssetMetadataEntry { symbol, metadata }));
            }
        }
        None
    }
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
impl serde::Serialize for AssetSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for OracleData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("OracleData", 2)?;
        state.serialize_field("oracle_pubkey", &pubkey_string(&self.oracle_pubkey))?;
        state.serialize_field("divergence_score", &self.divergence_score)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for MarkPrice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("MarkPrice", 13)?;
        state.serialize_field("price", &self.price)?;
        state.serialize_field("spot_price_component", &self.spot_price_component)?;
        state.serialize_field("perp_price_component", &self.perp_price_component)?;
        state.serialize_field("book_price_component", &self.book_price_component)?;
        state.serialize_field(
            "risk_action_price_validity_rules",
            &self.risk_action_price_validity_rules.as_slice(),
        )?;
        state.serialize_field("oracle_data", &self.oracle_data)?;
        state.serialize_field("oracle_parameters", &self.oracle_parameters())?;
        state.serialize_field("oracle_divergence_radius", &self.oracle_divergence_radius)?;
        state.serialize_field("min_oracle_responses", &self.min_oracle_responses)?;
        state.serialize_field(
            "book_hard_stale_multiplier",
            &self.book_hard_stale_multiplier,
        )?;
        state.serialize_field(
            "oracle_hard_stale_multiplier",
            &self.oracle_hard_stale_multiplier,
        )?;
        state.serialize_field(
            "mark_price_last_validated_slot",
            &self.mark_price_last_validated_slot,
        )?;
        state.serialize_field(
            "oracle_last_updated_timestamps",
            &self.oracle_last_updated_timestamps,
        )?;
        state.end()
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

#[cfg(feature = "serde")]
struct PerpAssetMapEntries<'a>(&'a PerpAssetMap<'a>);

#[cfg(feature = "serde")]
impl serde::Serialize for PerpAssetMapEntries<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.num_assets() as usize))?;
        for entry in self.0.iter() {
            let entry = entry.map_err(serde::ser::Error::custom)?;
            seq.serialize_element(&entry)?;
        }
        seq.end()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PerpAssetMap<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PerpAssetMap", 7)?;
        state.serialize_field("sequence_number", &self.sequence_number())?;
        state.serialize_field("num_assets", &self.num_assets())?;
        state.serialize_field("slots_used", &self.slots_used())?;
        state.serialize_field("tombstones", &self.tombstones())?;
        state.serialize_field("capacity", &self.capacity())?;
        state.serialize_field("entries", &PerpAssetMapEntries(self))?;
        state.serialize_field("active_entries", &self.num_assets())?;
        state.end()
    }
}
