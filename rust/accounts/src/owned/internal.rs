use std::collections::BTreeMap;

use phoenix_rise_math::{BaseLots, Ticks};
use serde::{Serialize, Serializer};
use solana_pubkey::Pubkey;

use super::AccountDeserializeError;
use crate::{
    common as borrowed_common, perp_asset_map as borrowed_perp, trader as borrowed_trader,
};

pub(crate) const ORDERBOOK_CAPACITY: usize = 8192;
pub(crate) const SPLINE_REGION_CAPACITY: usize = 10;
#[cfg(test)]
pub const GTC_LIFESPAN: u64 = u64::MAX;

const MAX_POSITION_SIZE_ACTIVE_FLAG: u32 = 1 << 0;

fn serialize_i128_as_string<S>(value: &i128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

#[derive(Debug, Clone)]
pub(crate) struct Reader<'a> {
    account: &'static str,
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(account: &'static str, bytes: &'a [u8]) -> Self {
        Self {
            account,
            bytes,
            offset: 0,
        }
    }

    pub(crate) fn with_offset(account: &'static str, bytes: &'a [u8], offset: usize) -> Self {
        Self {
            account,
            bytes,
            offset,
        }
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, AccountDeserializeError> {
        Ok(self.read_array::<1>()?[0])
    }

    pub(crate) fn read_i8(&mut self) -> Result<i8, AccountDeserializeError> {
        Ok(i8::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32, AccountDeserializeError> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_i32(&mut self) -> Result<i32, AccountDeserializeError> {
        Ok(i32::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_u64(&mut self) -> Result<u64, AccountDeserializeError> {
        Ok(u64::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_i64(&mut self) -> Result<i64, AccountDeserializeError> {
        Ok(i64::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_pubkey(&mut self) -> Result<Pubkey, AccountDeserializeError> {
        Ok(Pubkey::new_from_array(self.read_array()?))
    }

    pub(crate) fn read_symbol(&mut self) -> Result<String, AccountDeserializeError> {
        let bytes = self.read_array::<16>()?;
        let len = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        if !bytes[..len].iter().all(u8::is_ascii) {
            return Err(AccountDeserializeError::invalid_data(
                self.account,
                "symbol contains non-ASCII bytes",
            ));
        }
        Ok(String::from_utf8_lossy(&bytes[..len]).into_owned())
    }

    pub(crate) fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], AccountDeserializeError> {
        let end = self.offset.checked_add(len).ok_or_else(|| {
            AccountDeserializeError::invalid_data(self.account, "offset overflow")
        })?;
        if end > self.bytes.len() {
            return Err(AccountDeserializeError::too_short(
                self.account,
                end,
                self.bytes.len(),
            ));
        }
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    pub(crate) fn skip(&mut self, len: usize) -> Result<(), AccountDeserializeError> {
        self.read_bytes(len).map(|_| ())
    }

    pub(crate) fn read_array<const N: usize>(
        &mut self,
    ) -> Result<[u8; N], AccountDeserializeError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.read_bytes(N)?);
        Ok(out)
    }
}

pub(crate) fn verify_discriminant(
    account: &'static str,
    data: &[u8],
    expected: [u8; 8],
) -> Result<(), AccountDeserializeError> {
    if data.len() < 8 {
        return Err(AccountDeserializeError::too_short(account, 8, data.len()));
    }
    let mut actual = [0u8; 8];
    actual.copy_from_slice(&data[..8]);
    if actual != expected {
        return Err(AccountDeserializeError::InvalidDiscriminant {
            account,
            expected,
            actual,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SequenceNumber {
    pub sequence_number: u64,
    pub last_update_slot: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthoritySet {
    pub root_authority: Pubkey,
    pub risk_authority: Pubkey,
    pub market_authority: Pubkey,
    pub oracle_authority: Pubkey,
    pub reserved_authority: Pubkey,
    pub adl_authority: Pubkey,
    pub cancel_authority: Pubkey,
    pub backstop_authority: Pubkey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShortEntries<K, V> {
    pub len: u64,
    pub capacity: u64,
    pub entries: Vec<(K, V)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TraderState {
    pub quote_lot_collateral: i64,
    pub flags: u32,
    pub global_position_sequence_number: u8,
    pub maker_fee_override_multiplier: i8,
    pub taker_fee_override_multiplier: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TraderPosition {
    pub base_lot_position: i64,
    pub virtual_quote_lot_position: i64,
    pub cumulative_funding_snapshot: i64,
    pub position_sequence_number: u8,
    pub accumulated_funding_for_active_position: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TicksAtSlot {
    pub slot: u64,
    pub ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OracleData {
    pub oracle_pubkey: Pubkey,
    pub divergence_score: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OracleParameters {
    pub oracle_divergence_radius: u16,
    pub min_oracle_responses: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BookPriceComponent {
    pub clamped_book_mid_price: TicksAtSlot,
    pub weight: u64,
    pub stale_threshold: u64,
    pub book_price_radius: u64,
    pub last_best_bid: TicksAtSlot,
    pub last_best_ask: TicksAtSlot,
    pub last_trade_price: TicksAtSlot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerpPriceComponent {
    pub last_exchange_perp_price: Vec<TicksAtSlot>,
    pub weight: u64,
    pub stale_threshold: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpotPriceComponent {
    pub last_exchange_spot_price: Vec<TicksAtSlot>,
    pub weight: u64,
    pub stale_threshold: u64,
    pub slot: u64,
    pub mid_spot_diff_ema_ticks: i64,
    pub mid_spot_diff_ema_ticks_dust: i64,
    pub ema_period_slots: u64,
    pub ema_diff_radius: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MarkPrice {
    pub price: TicksAtSlot,
    pub spot_price_component: SpotPriceComponent,
    pub perp_price_component: PerpPriceComponent,
    pub book_price_component: BookPriceComponent,
    pub risk_action_price_validity_rules: Vec<u8>,
    pub oracle_data: Vec<OracleData>,
    pub oracle_parameters: OracleParameters,
    pub mark_price_last_validated_slot: Vec<u64>,
    pub oracle_last_updated_timestamps: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PriceComponent {
    pub price_sequence_number: SequenceNumber,
    pub mark_price: MarkPrice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StaticMarketParams {
    pub market_account: Pubkey,
    pub tick_size: u64,
    pub asset_id: u32,
    pub base_lot_decimals: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LeverageTier {
    pub upper_bound_size: u64,
    pub max_leverage: u64,
    pub limit_order_risk_factor: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TransferFeeTier {
    pub position_size_limit: u64,
    pub fee_rate: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RiskParams {
    pub leverage_tiers: Vec<LeverageTier>,
    pub upnl_risk_factor: u64,
    pub max_liquidation_size: u64,
    pub transfer_fee_tiers: Vec<TransferFeeTier>,
    pub risk_factors: Vec<u16>,
    pub cancel_order_risk_factor: u16,
    pub upnl_risk_factor_for_withdrawals: u64,
    pub isolated_only: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FundingAccumulator {
    #[serde(serialize_with = "serialize_i128_as_string")]
    pub acc: i128,
    #[serde(serialize_with = "serialize_i128_as_string")]
    pub last_diff: i128,
    pub cumulative_funding_rate: i64,
    pub start_interval_timestamp: u64,
    pub last_funding_update_timestamp: u64,
    pub funding_interval_seconds: u64,
    pub funding_period_seconds: u64,
    pub max_funding_rate: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OpenInterestParams {
    pub open_interest: u64,
    pub open_interest_cap: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AssetFlags {
    pub bits: u8,
    pub is_commodity: bool,
    pub is_commodities_reopen: bool,
    pub is_commodities_after_hours: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerpAssetMetadata {
    pub oracle_price: PriceComponent,
    pub static_market_params: StaticMarketParams,
    pub risk_params: RiskParams,
    pub funding_accumulator: FundingAccumulator,
    pub open_interest_params: OpenInterestParams,
    pub asset_flags: AssetFlags,
    pub commodities_after_hours_radius: u64,
    pub last_known_index_price: Option<u64>,
    pub last_index_expiry_timestamp: u64,
    pub commodities_after_hours_radius_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerpAssetEntry {
    pub symbol: String,
    pub metadata: PerpAssetMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TraderPositionEntry {
    pub asset_id: u64,
    pub position: TraderPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TickRegion {
    pub start_offset: u64,
    pub end_offset: u64,
    pub density: u64,
    pub top_level_hidden_take_size: u64,
    pub total_size: u64,
    pub filled_size: u64,
    pub hidden_filled_size: u64,
    pub lifespan: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Spline {
    pub trader: Pubkey,
    pub is_active: bool,
    pub mid_price: u64,
    pub sequence_number: SequenceNumber,
    pub user_update_slot: u64,
    pub bid_offset: u64,
    pub ask_offset: u64,
    pub bid_filled_amount: u64,
    pub ask_filled_amount: u64,
    pub bid_num_regions: u64,
    pub ask_num_regions: u64,
    pub bid_regions: Vec<TickRegion>,
    pub ask_regions: Vec<TickRegion>,
    pub user_price_sequence_number: u64,
    pub user_parameter_sequence_number: u64,
    pub flags: u32,
    pub leverage_decrease_in_bps: u64,
    pub max_position_size_long: u64,
    pub max_position_size_short: u64,
    pub has_max_position_size: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SplineSide {
    Bid,
    Ask,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SplineRiskCapacity {
    pub max_bid_lots: BaseLots,
    pub max_ask_lots: BaseLots,
}

impl SplineRiskCapacity {
    pub const fn new(max_bid_lots: BaseLots, max_ask_lots: BaseLots) -> Self {
        Self {
            max_bid_lots,
            max_ask_lots,
        }
    }

    pub const fn unlimited() -> Self {
        Self {
            max_bid_lots: BaseLots::new_const(u32::MAX as u64),
            max_ask_lots: BaseLots::new_const(u32::MAX as u64),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplineLiquidity {
    pub trader: Pubkey,
    pub mid_price_ticks: Ticks,
    pub bid_filled_amount: BaseLots,
    pub ask_filled_amount: BaseLots,
    pub bid_regions: Vec<SplineLiquidityRegion>,
    pub ask_regions: Vec<SplineLiquidityRegion>,
}

impl SplineLiquidity {
    pub fn has_liquidity(&self) -> bool {
        self.available_bid_lots() > BaseLots::ZERO || self.available_ask_lots() > BaseLots::ZERO
    }

    pub fn available_bid_lots(&self) -> BaseLots {
        available_region_lots(&self.bid_regions)
    }

    pub fn available_ask_lots(&self) -> BaseLots {
        available_region_lots(&self.ask_regions)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplineLiquidityRegion {
    pub start_price_ticks: Ticks,
    pub end_price_ticks: Ticks,
    pub density_lots_per_tick: BaseLots,
    pub total_size_lots: BaseLots,
    pub filled_size_lots: BaseLots,
}

impl SplineLiquidityRegion {
    pub fn available_size_lots(&self) -> BaseLots {
        self.total_size_lots.saturating_sub(self.filled_size_lots)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct SplineLiquidityLevel {
    available_lots: BaseLots,
    filled_lots: BaseLots,
}

impl TickRegion {
    pub fn is_active(&self, current_slot: u64, last_updated_slot: u64) -> bool {
        self.lifespan.saturating_add(last_updated_slot) >= current_slot
            && self.filled_size < self.total_size
    }

    pub fn unfilled_size(&self) -> BaseLots {
        BaseLots::new(self.total_size.saturating_sub(self.filled_size))
    }

    pub fn is_empty(&self) -> bool {
        self.density == 0
    }
}

impl Spline {
    pub fn is_enabled(&self) -> bool {
        self.is_active && self.mid_price != 0
    }

    pub fn active_bid_regions(&self) -> &[TickRegion] {
        active_regions(&self.bid_regions, self.bid_offset, self.bid_num_regions)
    }

    pub fn active_ask_regions(&self) -> &[TickRegion] {
        active_regions(&self.ask_regions, self.ask_offset, self.ask_num_regions)
    }

    pub fn active_bid_regions_at_slot(
        &self,
        current_slot: u64,
    ) -> impl Iterator<Item = &TickRegion> {
        let user_update_slot = self.user_update_slot;
        self.active_bid_regions()
            .iter()
            .filter(move |region| region.is_active(current_slot, user_update_slot))
    }

    pub fn active_ask_regions_at_slot(
        &self,
        current_slot: u64,
    ) -> impl Iterator<Item = &TickRegion> {
        let user_update_slot = self.user_update_slot;
        self.active_ask_regions()
            .iter()
            .filter(move |region| region.is_active(current_slot, user_update_slot))
    }

    pub fn max_position_size(&self, side: SplineSide) -> Option<BaseLots> {
        self.has_max_position_size.then(|| match side {
            SplineSide::Bid => BaseLots::new(self.max_position_size_long),
            SplineSide::Ask => BaseLots::new(self.max_position_size_short),
        })
    }

    pub fn position_limit_capacity(&self, base_position_lots: i64) -> SplineRiskCapacity {
        position_limit_capacity(
            self.max_position_size(SplineSide::Bid),
            self.max_position_size(SplineSide::Ask),
            base_position_lots,
        )
    }

    pub fn liquidity_at_slot_with_capacity(
        &self,
        current_slot: u64,
        capacity: SplineRiskCapacity,
    ) -> SplineLiquidity {
        liquidity_at_slot_from_parts(SplineLiquidityParts {
            trader: self.trader,
            enabled: self.is_enabled(),
            mid_price: Ticks::new(self.mid_price),
            user_update_slot: self.user_update_slot,
            bid_filled_amount: BaseLots::new(self.bid_filled_amount),
            ask_filled_amount: BaseLots::new(self.ask_filled_amount),
            bid_regions: self.active_bid_regions(),
            ask_regions: self.active_ask_regions(),
            current_slot,
            capacity,
        })
    }

    pub fn liquidity_at_slot_with_position_limits(
        &self,
        current_slot: u64,
        base_position_lots: i64,
    ) -> SplineLiquidity {
        self.liquidity_at_slot_with_capacity(
            current_slot,
            self.position_limit_capacity(base_position_lots),
        )
    }

    pub fn has_configured_regions(&self) -> bool {
        self.bid_offset < self.bid_num_regions || self.ask_offset < self.ask_num_regions
    }
}

fn active_regions(regions: &[TickRegion], offset: u64, num_regions: u64) -> &[TickRegion] {
    let start = offset as usize;
    let end = num_regions as usize;
    if start < end && end <= regions.len() {
        &regions[start..end]
    } else {
        &[]
    }
}

struct SplineLiquidityParts<'a> {
    trader: Pubkey,
    enabled: bool,
    mid_price: Ticks,
    user_update_slot: u64,
    bid_filled_amount: BaseLots,
    ask_filled_amount: BaseLots,
    bid_regions: &'a [TickRegion],
    ask_regions: &'a [TickRegion],
    current_slot: u64,
    capacity: SplineRiskCapacity,
}

fn liquidity_at_slot_from_parts(parts: SplineLiquidityParts<'_>) -> SplineLiquidity {
    let bid_regions = if parts.enabled {
        liquidity_regions_from_price_levels(
            SplineSide::Bid,
            liquidity_levels_from_regions(
                parts.mid_price,
                SplineSide::Bid,
                parts.bid_regions,
                parts.user_update_slot,
                parts.current_slot,
                parts.capacity.max_bid_lots,
            ),
        )
    } else {
        Vec::new()
    };
    let ask_regions = if parts.enabled {
        liquidity_regions_from_price_levels(
            SplineSide::Ask,
            liquidity_levels_from_regions(
                parts.mid_price,
                SplineSide::Ask,
                parts.ask_regions,
                parts.user_update_slot,
                parts.current_slot,
                parts.capacity.max_ask_lots,
            ),
        )
    } else {
        Vec::new()
    };

    SplineLiquidity {
        trader: parts.trader,
        mid_price_ticks: parts.mid_price,
        bid_filled_amount: parts.bid_filled_amount,
        ask_filled_amount: parts.ask_filled_amount,
        bid_regions,
        ask_regions,
    }
}

fn position_limit_capacity(
    max_position_size_long: Option<BaseLots>,
    max_position_size_short: Option<BaseLots>,
    base_position_lots: i64,
) -> SplineRiskCapacity {
    let (Some(max_long), Some(max_short)) = (max_position_size_long, max_position_size_short)
    else {
        return SplineRiskCapacity::unlimited();
    };

    let base_position_lots = i128::from(base_position_lots);
    let max_bid_lots = nonnegative_i128_to_base_lots(
        i128::from(max_long.as_inner()).saturating_sub(base_position_lots),
    );
    let max_ask_lots = nonnegative_i128_to_base_lots(
        i128::from(max_short.as_inner()).saturating_add(base_position_lots),
    );
    SplineRiskCapacity::new(max_bid_lots, max_ask_lots)
}

fn nonnegative_i128_to_base_lots(value: i128) -> BaseLots {
    BaseLots::new_saturating(value.clamp(0, i128::from(u64::MAX)) as u64)
}

fn liquidity_levels_from_regions(
    mid_price: Ticks,
    side: SplineSide,
    regions: &[TickRegion],
    user_update_slot: u64,
    current_slot: u64,
    max_lots: BaseLots,
) -> BTreeMap<Ticks, SplineLiquidityLevel> {
    let mut levels = BTreeMap::new();
    let mut lots_remaining = max_lots;

    for region in regions {
        if !region.is_active(current_slot, user_update_slot) {
            continue;
        }

        let density = BaseLots::new(region.density);
        if density == BaseLots::ZERO {
            continue;
        }

        let mut fills_to_consume = BaseLots::new(region.filled_size);
        let mut current_tick = Ticks::new(region.start_offset);
        let end_tick = Ticks::new(region.end_offset);

        while current_tick < end_tick && lots_remaining > BaseLots::ZERO {
            if fills_to_consume >= density {
                fills_to_consume = fills_to_consume.saturating_sub(density);
                current_tick = current_tick.saturating_add(Ticks::ONE);
                continue;
            }

            let filled_at_tick = fills_to_consume;
            let available_at_tick = density.saturating_sub(filled_at_tick);
            fills_to_consume = BaseLots::ZERO;

            if available_at_tick > BaseLots::ZERO {
                let lots_to_take = available_at_tick.min(lots_remaining);
                let price = match side {
                    SplineSide::Bid => mid_price.saturating_sub(current_tick),
                    SplineSide::Ask => mid_price.saturating_add(current_tick),
                };

                let level = levels.entry(price).or_insert(SplineLiquidityLevel {
                    available_lots: BaseLots::ZERO,
                    filled_lots: BaseLots::ZERO,
                });
                level.available_lots = level.available_lots.saturating_add(lots_to_take);
                level.filled_lots = level.filled_lots.saturating_add(filled_at_tick);
                lots_remaining = lots_remaining.saturating_sub(lots_to_take);
            }

            current_tick = current_tick.saturating_add(Ticks::ONE);
        }
    }

    levels
}

fn liquidity_regions_from_price_levels(
    side: SplineSide,
    levels: BTreeMap<Ticks, SplineLiquidityLevel>,
) -> Vec<SplineLiquidityRegion> {
    let mut regions = Vec::new();
    let mut current: Option<(Ticks, Ticks, BaseLots, BaseLots, BaseLots, BaseLots)> = None;

    for (price, level) in levels {
        if level.available_lots == BaseLots::ZERO {
            continue;
        }
        let total_lots_at_tick = level.available_lots.saturating_add(level.filled_lots);

        match current {
            Some((start, end, density, total_size, filled_size, filled_lots_at_tick))
                if price == end.saturating_add(Ticks::ONE)
                    && total_lots_at_tick == density
                    && level.filled_lots == filled_lots_at_tick =>
            {
                current = Some((
                    start,
                    price,
                    density,
                    total_size.saturating_add(total_lots_at_tick),
                    filled_size.saturating_add(level.filled_lots),
                    filled_lots_at_tick,
                ));
            }
            Some((start, end, density, total_size, filled_size, _)) => {
                regions.push(build_liquidity_region(
                    side,
                    start,
                    end,
                    density,
                    total_size,
                    filled_size,
                ));
                current = Some((
                    price,
                    price,
                    total_lots_at_tick,
                    total_lots_at_tick,
                    level.filled_lots,
                    level.filled_lots,
                ));
            }
            None => {
                current = Some((
                    price,
                    price,
                    total_lots_at_tick,
                    total_lots_at_tick,
                    level.filled_lots,
                    level.filled_lots,
                ));
            }
        }
    }

    if let Some((start, end, density, total_size, filled_size, _)) = current {
        regions.push(build_liquidity_region(
            side,
            start,
            end,
            density,
            total_size,
            filled_size,
        ));
    }

    regions
}

fn build_liquidity_region(
    side: SplineSide,
    start_tick: Ticks,
    end_tick: Ticks,
    density_lots_per_tick: BaseLots,
    total_size_lots: BaseLots,
    filled_size_lots: BaseLots,
) -> SplineLiquidityRegion {
    let (start_price_ticks, end_price_ticks) = match side {
        SplineSide::Bid => (start_tick.saturating_sub(Ticks::ONE), end_tick),
        SplineSide::Ask => (start_tick, end_tick.saturating_add(Ticks::ONE)),
    };

    SplineLiquidityRegion {
        start_price_ticks,
        end_price_ticks,
        density_lots_per_tick,
        total_size_lots,
        filled_size_lots,
    }
}

fn available_region_lots(regions: &[SplineLiquidityRegion]) -> BaseLots {
    regions.iter().fold(BaseLots::ZERO, |acc, region| {
        acc.saturating_add(region.available_size_lots())
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OrderFill {
    pub price: u64,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TradeEvent {
    pub slot: u64,
    pub timestamp: i64,
    pub trade_sequence_number: u64,
    pub prev_trade_sequence_number_slot: u64,
    pub maker_fill: OrderFill,
    pub maker: Pubkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TraderPositionId {
    pub trader_id: Option<u32>,
    pub asset_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OrderbookRestingOrder {
    pub trader_position_id: TraderPositionId,
    pub initial_trade_size: u64,
    pub num_base_lots_remaining: u64,
    pub order_flags: u8,
    pub optional_conditional_order_index: Option<u8>,
    pub expiration_offset: Option<u32>,
    pub initial_slot: u64,
    pub prev_node: Option<u32>,
    pub next_node: Option<u32>,
    pub last_valid_slot: Option<u64>,
    pub reduce_only: bool,
    pub is_stop_loss: bool,
    pub is_stop_loss_direction: bool,
    pub is_conditional_order: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FifoOrderId {
    pub price_in_ticks: u64,
    pub order_sequence_number: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OrderbookEntry {
    pub order_id: FifoOrderId,
    pub order: OrderbookRestingOrder,
}

impl From<borrowed_common::SequenceNumber> for SequenceNumber {
    fn from(value: borrowed_common::SequenceNumber) -> Self {
        Self {
            sequence_number: value.sequence_number,
            last_update_slot: value.last_update_slot,
        }
    }
}

impl From<borrowed_trader::TraderState> for TraderState {
    fn from(value: borrowed_trader::TraderState) -> Self {
        Self {
            quote_lot_collateral: value.quote_lot_collateral,
            flags: value.flags,
            global_position_sequence_number: value.global_position_sequence_number,
            maker_fee_override_multiplier: value.maker_fee_override_multiplier,
            taker_fee_override_multiplier: value.taker_fee_override_multiplier,
        }
    }
}

impl From<borrowed_trader::TraderPosition> for TraderPosition {
    fn from(value: borrowed_trader::TraderPosition) -> Self {
        Self {
            base_lot_position: value.base_lot_position,
            virtual_quote_lot_position: value.virtual_quote_lot_position,
            cumulative_funding_snapshot: value.cumulative_funding_snapshot,
            position_sequence_number: value.position_sequence_number,
            accumulated_funding_for_active_position: value.accumulated_funding_for_active_position,
        }
    }
}

impl From<&borrowed_perp::TicksAtSlot> for TicksAtSlot {
    fn from(value: &borrowed_perp::TicksAtSlot) -> Self {
        Self {
            slot: value.slot,
            ticks: value.ticks,
        }
    }
}

impl From<borrowed_perp::TicksAtSlot> for TicksAtSlot {
    fn from(value: borrowed_perp::TicksAtSlot) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::OracleData> for OracleData {
    fn from(value: &borrowed_perp::OracleData) -> Self {
        Self {
            oracle_pubkey: Pubkey::new_from_array(value.oracle_pubkey),
            divergence_score: value.divergence_score,
        }
    }
}

impl From<borrowed_perp::OracleData> for OracleData {
    fn from(value: borrowed_perp::OracleData) -> Self {
        Self::from(&value)
    }
}

impl From<borrowed_perp::OracleParameters> for OracleParameters {
    fn from(value: borrowed_perp::OracleParameters) -> Self {
        Self {
            oracle_divergence_radius: value.oracle_divergence_radius,
            min_oracle_responses: value.min_oracle_responses,
        }
    }
}

impl From<&borrowed_perp::BookPriceComponent> for BookPriceComponent {
    fn from(value: &borrowed_perp::BookPriceComponent) -> Self {
        Self {
            clamped_book_mid_price: (&value.clamped_book_mid_price).into(),
            weight: value.weight,
            stale_threshold: value.stale_threshold,
            book_price_radius: value.book_price_radius,
            last_best_bid: (&value.last_best_bid).into(),
            last_best_ask: (&value.last_best_ask).into(),
            last_trade_price: (&value.last_trade_price).into(),
        }
    }
}

impl From<borrowed_perp::BookPriceComponent> for BookPriceComponent {
    fn from(value: borrowed_perp::BookPriceComponent) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::PerpPriceComponent> for PerpPriceComponent {
    fn from(value: &borrowed_perp::PerpPriceComponent) -> Self {
        Self {
            last_exchange_perp_price: value
                .last_exchange_perp_price
                .iter()
                .map(Into::into)
                .collect(),
            weight: value.weight,
            stale_threshold: value.stale_threshold,
        }
    }
}

impl From<borrowed_perp::PerpPriceComponent> for PerpPriceComponent {
    fn from(value: borrowed_perp::PerpPriceComponent) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::SpotPriceComponent> for SpotPriceComponent {
    fn from(value: &borrowed_perp::SpotPriceComponent) -> Self {
        Self {
            last_exchange_spot_price: value
                .last_exchange_spot_price
                .iter()
                .map(Into::into)
                .collect(),
            weight: value.weight,
            stale_threshold: value.stale_threshold,
            slot: value.slot,
            mid_spot_diff_ema_ticks: value.mid_spot_diff_ema_ticks,
            mid_spot_diff_ema_ticks_dust: value.mid_spot_diff_ema_ticks_dust,
            ema_period_slots: value.ema_period_slots,
            ema_diff_radius: value.ema_diff_radius,
        }
    }
}

impl From<borrowed_perp::SpotPriceComponent> for SpotPriceComponent {
    fn from(value: borrowed_perp::SpotPriceComponent) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::MarkPrice> for MarkPrice {
    fn from(value: &borrowed_perp::MarkPrice) -> Self {
        Self {
            price: (&value.price).into(),
            spot_price_component: (&value.spot_price_component).into(),
            perp_price_component: (&value.perp_price_component).into(),
            book_price_component: (&value.book_price_component).into(),
            risk_action_price_validity_rules: value.risk_action_price_validity_rules.to_vec(),
            oracle_data: value.oracle_data.iter().map(Into::into).collect(),
            oracle_parameters: value.oracle_parameters().into(),
            mark_price_last_validated_slot: value.mark_price_last_validated_slot.to_vec(),
            oracle_last_updated_timestamps: value.oracle_last_updated_timestamps.to_vec(),
        }
    }
}

impl From<borrowed_perp::MarkPrice> for MarkPrice {
    fn from(value: borrowed_perp::MarkPrice) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::PriceComponent> for PriceComponent {
    fn from(value: &borrowed_perp::PriceComponent) -> Self {
        Self {
            price_sequence_number: value.price_sequence_number.into(),
            mark_price: (&value.mark_price).into(),
        }
    }
}

impl From<borrowed_perp::PriceComponent> for PriceComponent {
    fn from(value: borrowed_perp::PriceComponent) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::StaticMarketParams> for StaticMarketParams {
    fn from(value: &borrowed_perp::StaticMarketParams) -> Self {
        Self {
            market_account: Pubkey::new_from_array(value.market_account),
            tick_size: value.tick_size,
            asset_id: value.asset_id(),
            base_lot_decimals: value.base_lot_decimals,
        }
    }
}

impl From<borrowed_perp::StaticMarketParams> for StaticMarketParams {
    fn from(value: borrowed_perp::StaticMarketParams) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::LeverageTier> for LeverageTier {
    fn from(value: &borrowed_perp::LeverageTier) -> Self {
        Self {
            upper_bound_size: value.upper_bound_size,
            max_leverage: value.max_leverage,
            limit_order_risk_factor: value.limit_order_risk_factor,
        }
    }
}

impl From<borrowed_perp::LeverageTier> for LeverageTier {
    fn from(value: borrowed_perp::LeverageTier) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::TransferFeeTier> for TransferFeeTier {
    fn from(value: &borrowed_perp::TransferFeeTier) -> Self {
        Self {
            position_size_limit: value.position_size_limit,
            fee_rate: value.fee_rate,
        }
    }
}

impl From<borrowed_perp::TransferFeeTier> for TransferFeeTier {
    fn from(value: borrowed_perp::TransferFeeTier) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::RiskParams> for RiskParams {
    fn from(value: &borrowed_perp::RiskParams) -> Self {
        Self {
            leverage_tiers: value.leverage_tiers.iter().map(Into::into).collect(),
            upnl_risk_factor: value.upnl_risk_factor,
            max_liquidation_size: value.max_liquidation_size,
            transfer_fee_tiers: value.transfer_fee_tiers.iter().map(Into::into).collect(),
            risk_factors: value.risk_factors.to_vec(),
            cancel_order_risk_factor: value.cancel_order_risk_factor,
            upnl_risk_factor_for_withdrawals: value.upnl_risk_factor_for_withdrawals,
            isolated_only: value.isolated_only,
        }
    }
}

impl From<borrowed_perp::RiskParams> for RiskParams {
    fn from(value: borrowed_perp::RiskParams) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::FundingAccumulator> for FundingAccumulator {
    fn from(value: &borrowed_perp::FundingAccumulator) -> Self {
        Self {
            acc: value.acc,
            last_diff: value.last_diff,
            cumulative_funding_rate: value.cumulative_funding_rate,
            start_interval_timestamp: value.start_interval_timestamp,
            last_funding_update_timestamp: value.last_funding_update_timestamp,
            funding_interval_seconds: value.funding_interval_seconds,
            funding_period_seconds: value.funding_period_seconds,
            max_funding_rate: value.max_funding_rate,
        }
    }
}

impl From<borrowed_perp::FundingAccumulator> for FundingAccumulator {
    fn from(value: borrowed_perp::FundingAccumulator) -> Self {
        Self::from(&value)
    }
}

impl From<&borrowed_perp::OpenInterestParams> for OpenInterestParams {
    fn from(value: &borrowed_perp::OpenInterestParams) -> Self {
        Self {
            open_interest: value.open_interest,
            open_interest_cap: value.open_interest_cap,
        }
    }
}

impl From<borrowed_perp::OpenInterestParams> for OpenInterestParams {
    fn from(value: borrowed_perp::OpenInterestParams) -> Self {
        Self::from(&value)
    }
}

impl From<borrowed_perp::AssetFlags> for AssetFlags {
    fn from(value: borrowed_perp::AssetFlags) -> Self {
        Self {
            bits: value.bits,
            is_commodity: value.is_commodity,
            is_commodities_reopen: value.is_commodities_reopen,
            is_commodities_after_hours: value.is_commodities_after_hours,
        }
    }
}

impl From<borrowed_perp::PerpAssetMetadata> for PerpAssetMetadata {
    fn from(value: borrowed_perp::PerpAssetMetadata) -> Self {
        Self {
            oracle_price: value.oracle_price().into(),
            static_market_params: value.static_market_params().into(),
            risk_params: value.risk_params().into(),
            funding_accumulator: value.funding_accumulator().into(),
            open_interest_params: value.open_interest_params().into(),
            asset_flags: value.asset_flags().into(),
            commodities_after_hours_radius: value.commodities_after_hours_radius(),
            last_known_index_price: value.last_known_index_price(),
            last_index_expiry_timestamp: value.last_index_expiry_timestamp(),
            commodities_after_hours_radius_bps: value.commodities_after_hours_radius_bps() as u16,
        }
    }
}

pub(crate) fn read_sequence_number(
    reader: &mut Reader<'_>,
) -> Result<SequenceNumber, AccountDeserializeError> {
    Ok(SequenceNumber {
        sequence_number: reader.read_u64()?,
        last_update_slot: reader.read_u64()?,
    })
}

pub(crate) fn read_spline(reader: &mut Reader<'_>) -> Result<Spline, AccountDeserializeError> {
    let trader = reader.read_pubkey()?;
    let is_active = reader.read_u8()? != 0;
    reader.skip(7)?;
    let mid_price = reader.read_u64()?;
    let sequence_number = read_sequence_number(reader)?;
    let user_update_slot = reader.read_u64()?;
    let bid_offset = reader.read_u64()?;
    let ask_offset = reader.read_u64()?;
    let bid_filled_amount = reader.read_u64()?;
    let ask_filled_amount = reader.read_u64()?;
    let bid_num_regions = reader.read_u64()?;
    let ask_num_regions = reader.read_u64()?;
    let bid_regions = read_fixed_vec(reader, SPLINE_REGION_CAPACITY, read_tick_region)?;
    let ask_regions = read_fixed_vec(reader, SPLINE_REGION_CAPACITY, read_tick_region)?;
    let user_price_sequence_number = reader.read_u64()?;
    let user_parameter_sequence_number = reader.read_u64()?;
    let flags = reader.read_u32()?;
    let leverage_decrease_in_bps = u64::from(reader.read_u32()?);
    let max_position_size_long = u64::from(reader.read_u32()?);
    let max_position_size_short = u64::from(reader.read_u32()?);
    reader.skip(8 * 28)?;
    Ok(Spline {
        trader,
        is_active,
        mid_price,
        sequence_number,
        user_update_slot,
        bid_offset,
        ask_offset,
        bid_filled_amount,
        ask_filled_amount,
        bid_num_regions,
        ask_num_regions,
        bid_regions,
        ask_regions,
        user_price_sequence_number,
        user_parameter_sequence_number,
        flags,
        leverage_decrease_in_bps,
        max_position_size_long,
        max_position_size_short,
        has_max_position_size: flags & MAX_POSITION_SIZE_ACTIVE_FLAG != 0,
    })
}

pub(crate) fn read_trade_event(
    reader: &mut Reader<'_>,
) -> Result<TradeEvent, AccountDeserializeError> {
    Ok(TradeEvent {
        slot: reader.read_u64()?,
        timestamp: reader.read_i64()?,
        trade_sequence_number: reader.read_u64()?,
        prev_trade_sequence_number_slot: reader.read_u64()?,
        maker_fill: read_order_fill(reader)?,
        maker: reader.read_pubkey()?,
    })
}

pub(crate) fn read_fifo_order_id(
    reader: &mut Reader<'_>,
) -> Result<FifoOrderId, AccountDeserializeError> {
    Ok(FifoOrderId {
        price_in_ticks: reader.read_u64()?,
        order_sequence_number: reader.read_u64()?,
    })
}

pub(crate) fn read_orderbook_resting_order(
    reader: &mut Reader<'_>,
) -> Result<OrderbookRestingOrder, AccountDeserializeError> {
    let trader_position_id = TraderPositionId {
        trader_id: none_if_zero(reader.read_u32()?),
        asset_id: reader.read_u32()?,
    };
    let initial_trade_size = reader.read_u64()?;
    let num_base_lots_remaining = reader.read_u64()?;
    let order_flags = reader.read_u8()?;
    let optional_conditional_order_index = none_if_zero_u8(reader.read_u8()?);
    reader.skip(2)?;
    let expiration_offset = none_if_zero(reader.read_u32()?);
    let initial_slot = reader.read_u64()?;
    let prev_node = none_if_zero(reader.read_u32()?);
    let next_node = none_if_zero(reader.read_u32()?);
    Ok(OrderbookRestingOrder {
        trader_position_id,
        initial_trade_size,
        num_base_lots_remaining,
        order_flags,
        optional_conditional_order_index,
        expiration_offset,
        initial_slot,
        prev_node,
        next_node,
        last_valid_slot: expiration_offset
            .map(|offset| initial_slot.saturating_add(u64::from(offset))),
        reduce_only: order_flags & (1 << 7) != 0,
        is_stop_loss: order_flags & (1 << 6) != 0,
        is_stop_loss_direction: order_flags & (1 << 4) != 0,
        is_conditional_order: order_flags & (1 << 5) != 0,
    })
}

pub(crate) fn occupied_conditional_order_indices(bits: &[u8]) -> Vec<u8> {
    let mut occupied = Vec::new();
    for (byte_index, value) in bits.iter().copied().enumerate() {
        for bit in 0..8 {
            if value & (1 << bit) == 0 {
                continue;
            }
            let index = byte_index * 8 + bit;
            if (1..=191).contains(&index) {
                occupied.push(index as u8);
            }
        }
    }
    occupied
}

pub(crate) fn none_if_zero(value: u32) -> Option<u32> {
    if value == 0 { None } else { Some(value) }
}

fn none_if_zero_u8(value: u8) -> Option<u8> {
    if value == 0 { None } else { Some(value) }
}

fn read_tick_region(reader: &mut Reader<'_>) -> Result<TickRegion, AccountDeserializeError> {
    Ok(TickRegion {
        start_offset: reader.read_u64()?,
        end_offset: reader.read_u64()?,
        density: u64::from(reader.read_u32()?),
        top_level_hidden_take_size: u64::from(reader.read_u32()?),
        total_size: reader.read_u64()?,
        filled_size: u64::from(reader.read_u32()?),
        hidden_filled_size: u64::from(reader.read_u32()?),
        lifespan: reader.read_u64()?,
    })
}

fn read_order_fill(reader: &mut Reader<'_>) -> Result<OrderFill, AccountDeserializeError> {
    Ok(OrderFill {
        price: reader.read_u64()?,
        quantity: reader.read_i64()?,
    })
}

fn read_fixed_vec<T>(
    reader: &mut Reader<'_>,
    len: usize,
    mut read: impl FnMut(&mut Reader<'_>) -> Result<T, AccountDeserializeError>,
) -> Result<Vec<T>, AccountDeserializeError> {
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(read(reader)?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick_region(
        start_offset: u64,
        end_offset: u64,
        density: u64,
        filled_size: u64,
    ) -> TickRegion {
        TickRegion {
            start_offset,
            end_offset,
            density,
            top_level_hidden_take_size: 0,
            total_size: end_offset
                .saturating_sub(start_offset)
                .saturating_mul(density),
            filled_size,
            hidden_filled_size: 0,
            lifespan: GTC_LIFESPAN,
        }
    }

    fn test_spline() -> Spline {
        Spline {
            trader: Pubkey::new_unique(),
            is_active: true,
            mid_price: 100,
            sequence_number: SequenceNumber {
                sequence_number: 0,
                last_update_slot: 0,
            },
            user_update_slot: 10,
            bid_offset: 0,
            ask_offset: 0,
            bid_filled_amount: 3,
            ask_filled_amount: 0,
            bid_num_regions: 1,
            ask_num_regions: 1,
            bid_regions: vec![tick_region(1, 3, 5, 3)],
            ask_regions: vec![tick_region(1, 2, 7, 0)],
            user_price_sequence_number: 0,
            user_parameter_sequence_number: 0,
            flags: MAX_POSITION_SIZE_ACTIVE_FLAG,
            leverage_decrease_in_bps: 0,
            max_position_size_long: 15,
            max_position_size_short: 8,
            has_max_position_size: true,
        }
    }

    #[test]
    fn active_regions_at_slot_filter_expired_and_filled_regions() {
        let mut spline = test_spline();
        spline.bid_regions = vec![
            TickRegion {
                lifespan: 5,
                ..tick_region(1, 2, 5, 0)
            },
            tick_region(2, 3, 5, 5),
            tick_region(3, 4, 5, 0),
        ];
        spline.bid_num_regions = spline.bid_regions.len() as u64;

        let active: Vec<_> = spline.active_bid_regions_at_slot(16).collect();

        assert_eq!(active.len(), 1);
        assert_eq!(active[0].start_offset, 3);
    }

    #[test]
    fn position_limit_capacity_accounts_for_current_position() {
        let spline = test_spline();

        let capacity = spline.position_limit_capacity(12);
        assert_eq!(capacity.max_bid_lots, BaseLots::new(3));
        assert_eq!(capacity.max_ask_lots, BaseLots::new(20));

        let capacity = spline.position_limit_capacity(20);
        assert_eq!(capacity.max_bid_lots, BaseLots::ZERO);
        assert_eq!(capacity.max_ask_lots, BaseLots::new(28));
    }

    #[test]
    fn liquidity_materialization_applies_capacity_and_position_limits() {
        let spline = test_spline();

        let unlimited = spline.liquidity_at_slot_with_capacity(10, SplineRiskCapacity::unlimited());
        assert_eq!(unlimited.available_bid_lots(), BaseLots::new(7));
        assert_eq!(unlimited.available_ask_lots(), BaseLots::new(7));
        assert!(unlimited.has_liquidity());

        let limited = spline.liquidity_at_slot_with_position_limits(10, 12);
        assert_eq!(limited.available_bid_lots(), BaseLots::new(3));
        assert_eq!(limited.available_ask_lots(), BaseLots::new(7));
    }
}
