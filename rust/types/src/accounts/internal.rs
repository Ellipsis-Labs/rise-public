use solana_pubkey::Pubkey;

use super::AccountDeserializeError;

pub(crate) const MAX_NUMBER_OF_PERP_ASSETS: usize = 1024;
pub(crate) const ORDERBOOK_CAPACITY: usize = 8192;
pub(crate) const CONDITIONAL_ORDER_BITS_LEN: usize = 24;
pub(crate) const SPLINE_REGION_CAPACITY: usize = 10;

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

    pub(crate) fn offset(&self) -> usize {
        self.offset
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, AccountDeserializeError> {
        Ok(self.read_array::<1>()?[0])
    }

    pub(crate) fn read_i8(&mut self) -> Result<i8, AccountDeserializeError> {
        Ok(i8::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16, AccountDeserializeError> {
        Ok(u16::from_le_bytes(self.read_array()?))
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

    pub(crate) fn read_i128(&mut self) -> Result<i128, AccountDeserializeError> {
        Ok(i128::from_le_bytes(self.read_array()?))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceNumber {
    pub sequence_number: u64,
    pub last_update_slot: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortEntries<K, V> {
    pub len: u64,
    pub capacity: u64,
    pub entries: Vec<(K, V)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraderState {
    pub quote_lot_collateral: i64,
    pub flags: u32,
    pub global_position_sequence_number: u8,
    pub maker_fee_override_multiplier: i8,
    pub taker_fee_override_multiplier: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraderPosition {
    pub base_lot_position: i64,
    pub virtual_quote_lot_position: i64,
    pub cumulative_funding_snapshot: i64,
    pub position_sequence_number: u8,
    pub accumulated_funding_for_active_position: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TicksAtSlot {
    pub slot: u64,
    pub ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleData {
    pub oracle_pubkey: Pubkey,
    pub divergence_score: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OracleParameters {
    pub oracle_divergence_radius: u16,
    pub min_oracle_responses: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookPriceComponent {
    pub clamped_book_mid_price: TicksAtSlot,
    pub weight: u64,
    pub stale_threshold: u64,
    pub book_price_radius: u64,
    pub last_best_bid: TicksAtSlot,
    pub last_best_ask: TicksAtSlot,
    pub last_trade_price: TicksAtSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerpPriceComponent {
    pub last_exchange_perp_price: Vec<TicksAtSlot>,
    pub weight: u64,
    pub stale_threshold: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkPrice {
    pub price: TicksAtSlot,
    pub spot_price_component: SpotPriceComponent,
    pub perp_price_component: PerpPriceComponent,
    pub book_price_component: BookPriceComponent,
    pub risk_action_price_validity_rules: Vec<u8>,
    pub oracle_data: Vec<OracleData>,
    pub oracle_parameters: OracleParameters,
    pub mark_price_last_validated_slot: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceComponent {
    pub price_sequence_number: SequenceNumber,
    pub mark_price: MarkPrice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticMarketParams {
    pub market_account: Pubkey,
    pub tick_size: u64,
    pub asset_id: u32,
    pub base_lot_decimals: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeverageTier {
    pub upper_bound_size: u64,
    pub max_leverage: u64,
    pub limit_order_risk_factor: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferFeeTier {
    pub position_size_limit: u64,
    pub fee_rate: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FundingAccumulator {
    pub acc: i128,
    pub last_diff: i128,
    pub cumulative_funding_rate: i64,
    pub start_interval_timestamp: u64,
    pub last_funding_update_timestamp: u64,
    pub funding_interval_seconds: u64,
    pub funding_period_seconds: u64,
    pub max_funding_rate: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenInterestParams {
    pub open_interest: u64,
    pub open_interest_cap: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetFlags {
    pub bits: u8,
    pub is_commodity: bool,
    pub is_commodities_reopen: bool,
    pub is_commodities_after_hours: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerpAssetEntry {
    pub symbol: String,
    pub metadata: PerpAssetMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraderPositionEntry {
    pub asset_id: u64,
    pub position: TraderPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderFill {
    pub price: u64,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeEvent {
    pub slot: u64,
    pub timestamp: i64,
    pub trade_sequence_number: u64,
    pub prev_trade_sequence_number_slot: u64,
    pub maker_fill: OrderFill,
    pub maker: Pubkey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraderPositionId {
    pub trader_id: Option<u32>,
    pub asset_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FifoOrderId {
    pub price_in_ticks: u64,
    pub order_sequence_number: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderbookEntry {
    pub order_id: FifoOrderId,
    pub order: OrderbookRestingOrder,
}

pub(crate) fn read_sequence_number(
    reader: &mut Reader<'_>,
) -> Result<SequenceNumber, AccountDeserializeError> {
    Ok(SequenceNumber {
        sequence_number: reader.read_u64()?,
        last_update_slot: reader.read_u64()?,
    })
}

pub(crate) fn read_authority_set(
    reader: &mut Reader<'_>,
) -> Result<AuthoritySet, AccountDeserializeError> {
    Ok(AuthoritySet {
        root_authority: reader.read_pubkey()?,
        risk_authority: reader.read_pubkey()?,
        market_authority: reader.read_pubkey()?,
        oracle_authority: reader.read_pubkey()?,
        reserved_authority: reader.read_pubkey()?,
        adl_authority: reader.read_pubkey()?,
        cancel_authority: reader.read_pubkey()?,
        backstop_authority: reader.read_pubkey()?,
    })
}

pub(crate) fn read_trader_state(
    reader: &mut Reader<'_>,
) -> Result<TraderState, AccountDeserializeError> {
    let quote_lot_collateral = reader.read_i64()?;
    let flags = reader.read_u32()?;
    reader.skip(1)?;
    Ok(TraderState {
        quote_lot_collateral,
        flags,
        global_position_sequence_number: reader.read_u8()?,
        maker_fee_override_multiplier: reader.read_i8()?,
        taker_fee_override_multiplier: reader.read_i8()?,
    })
}

pub(crate) fn read_trader_position(
    reader: &mut Reader<'_>,
) -> Result<TraderPosition, AccountDeserializeError> {
    Ok(TraderPosition {
        base_lot_position: reader.read_i64()?,
        virtual_quote_lot_position: reader.read_i64()?,
        cumulative_funding_snapshot: reader.read_i64()?,
        position_sequence_number: reader.read_u8()?,
        accumulated_funding_for_active_position: read_i56(reader)?,
    })
}

pub(crate) fn read_ticks_at_slot(
    reader: &mut Reader<'_>,
) -> Result<TicksAtSlot, AccountDeserializeError> {
    Ok(TicksAtSlot {
        slot: reader.read_u64()?,
        ticks: reader.read_u64()?,
    })
}

pub(crate) fn read_perp_asset_metadata(
    reader: &mut Reader<'_>,
) -> Result<(PerpAssetMetadata, bool), AccountDeserializeError> {
    let oracle_price = read_price_component(reader)?;
    reader.skip(8 * 2)?;
    let static_market_params = read_static_market_params(reader)?;
    reader.skip(8 * 8)?;
    let risk_params = read_risk_params(reader)?;
    reader.skip(8 * 8)?;
    let funding_accumulator = read_funding_accumulator(reader)?;
    reader.skip(8 * 6)?;
    let open_interest_params = OpenInterestParams {
        open_interest: reader.read_u64()?,
        open_interest_cap: reader.read_u64()?,
    };
    reader.skip(8)?;
    reader.skip(2)?;
    let is_tombstoned = reader.read_u8()? != 0;
    reader.skip(5)?;
    let asset_flags_bits = reader.read_u8()?;
    let asset_flags = AssetFlags {
        bits: asset_flags_bits,
        is_commodity: asset_flags_bits & (1 << 0) != 0,
        is_commodities_reopen: asset_flags_bits & (1 << 1) != 0,
        is_commodities_after_hours: asset_flags_bits & (1 << 2) != 0,
    };
    reader.skip(7)?;
    let commodities_after_hours_radius = reader.read_u64()?;
    let last_known_index_price = none_if_zero_u64(reader.read_u64()?);
    let last_index_expiry_timestamp = reader.read_u64()?;
    let commodities_after_hours_radius_bps = reader.read_u16()?;
    reader.skip(6)?;
    reader.skip(8 * 9)?;
    Ok((
        PerpAssetMetadata {
            oracle_price,
            static_market_params,
            risk_params,
            funding_accumulator,
            open_interest_params,
            asset_flags,
            commodities_after_hours_radius,
            last_known_index_price,
            last_index_expiry_timestamp,
            commodities_after_hours_radius_bps,
        },
        !is_tombstoned,
    ))
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
        has_max_position_size: flags & 1 != 0,
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

fn none_if_zero_u64(value: u64) -> Option<u64> {
    if value == 0 { None } else { Some(value) }
}

fn none_if_zero_u8(value: u8) -> Option<u8> {
    if value == 0 { None } else { Some(value) }
}

fn read_i56(reader: &mut Reader<'_>) -> Result<i64, AccountDeserializeError> {
    let bytes = reader.read_array::<7>()?;
    let mut extended = [0u8; 8];
    extended[..7].copy_from_slice(&bytes);
    if bytes[6] & 0x80 != 0 {
        extended[7] = u8::MAX;
    }
    Ok(i64::from_le_bytes(extended))
}

fn read_price_component(
    reader: &mut Reader<'_>,
) -> Result<PriceComponent, AccountDeserializeError> {
    Ok(PriceComponent {
        price_sequence_number: read_sequence_number(reader)?,
        mark_price: read_mark_price(reader)?,
    })
}

fn read_mark_price(reader: &mut Reader<'_>) -> Result<MarkPrice, AccountDeserializeError> {
    let price = read_ticks_at_slot(reader)?;
    let spot_price_component = SpotPriceComponent {
        last_exchange_spot_price: read_fixed_vec(reader, 5, read_ticks_at_slot)?,
        weight: reader.read_u64()?,
        stale_threshold: reader.read_u64()?,
        slot: reader.read_u64()?,
        mid_spot_diff_ema_ticks: reader.read_i64()?,
        mid_spot_diff_ema_ticks_dust: reader.read_i64()?,
        ema_period_slots: reader.read_u64()?,
        ema_diff_radius: reader.read_u64()?,
    };
    let perp_price_component = PerpPriceComponent {
        last_exchange_perp_price: read_fixed_vec(reader, 5, read_ticks_at_slot)?,
        weight: reader.read_u64()?,
        stale_threshold: reader.read_u64()?,
    };
    let book_price_component = BookPriceComponent {
        clamped_book_mid_price: read_ticks_at_slot(reader)?,
        weight: reader.read_u64()?,
        stale_threshold: reader.read_u64()?,
        book_price_radius: reader.read_u64()?,
        last_best_bid: read_ticks_at_slot(reader)?,
        last_best_ask: read_ticks_at_slot(reader)?,
        last_trade_price: read_ticks_at_slot(reader)?,
    };
    let risk_action_price_validity_rules = reader.read_bytes(256)?.to_vec();
    let oracle_data = read_fixed_vec(reader, 5, |reader| {
        let oracle_pubkey = reader.read_pubkey()?;
        let divergence_score = reader.read_u8()?;
        reader.skip(7)?;
        Ok(OracleData {
            oracle_pubkey,
            divergence_score,
        })
    })?;
    let oracle_parameters = OracleParameters {
        oracle_divergence_radius: reader.read_u16()?,
        min_oracle_responses: reader.read_u8()?,
    };
    reader.skip(5)?;
    let mark_price_last_validated_slot = read_fixed_vec(reader, 4, |reader| reader.read_u64())?;
    reader.skip(8 * 5)?;
    Ok(MarkPrice {
        price,
        spot_price_component,
        perp_price_component,
        book_price_component,
        risk_action_price_validity_rules,
        oracle_data,
        oracle_parameters,
        mark_price_last_validated_slot,
    })
}

fn read_static_market_params(
    reader: &mut Reader<'_>,
) -> Result<StaticMarketParams, AccountDeserializeError> {
    let market_account = reader.read_pubkey()?;
    let tick_size = reader.read_u64()?;
    let asset_id_lower_bytes = reader.read_u16()?;
    let base_lot_decimals = reader.read_i8()?;
    reader.skip(1)?;
    let asset_id_upper_bytes = reader.read_u16()?;
    reader.skip(2)?;
    Ok(StaticMarketParams {
        market_account,
        tick_size,
        asset_id: u32::from(asset_id_lower_bytes) | (u32::from(asset_id_upper_bytes) << 16),
        base_lot_decimals,
    })
}

fn read_risk_params(reader: &mut Reader<'_>) -> Result<RiskParams, AccountDeserializeError> {
    let leverage_tiers = read_fixed_vec(reader, 4, |reader| {
        Ok(LeverageTier {
            upper_bound_size: reader.read_u64()?,
            max_leverage: reader.read_u64()?,
            limit_order_risk_factor: reader.read_u64()?,
        })
    })?;
    let upnl_risk_factor = reader.read_u64()?;
    let max_liquidation_size = reader.read_u64()?;
    let transfer_fee_tiers = read_fixed_vec(reader, 4, |reader| {
        let position_size_limit = reader.read_u64()?;
        let fee_rate = reader.read_u16()?;
        reader.skip(6)?;
        Ok(TransferFeeTier {
            position_size_limit,
            fee_rate,
        })
    })?;
    let risk_factors = read_fixed_vec(reader, 3, |reader| reader.read_u16())?;
    let cancel_order_risk_factor = reader.read_u16()?;
    let upnl_risk_factor_for_withdrawals = reader.read_u64()?;
    let isolated_only = reader.read_u8()?;
    reader.skip(7)?;
    Ok(RiskParams {
        leverage_tiers,
        upnl_risk_factor,
        max_liquidation_size,
        transfer_fee_tiers,
        risk_factors,
        cancel_order_risk_factor,
        upnl_risk_factor_for_withdrawals,
        isolated_only,
    })
}

fn read_funding_accumulator(
    reader: &mut Reader<'_>,
) -> Result<FundingAccumulator, AccountDeserializeError> {
    let acc = reader.read_i128()?;
    let last_diff = reader.read_i128()?;
    let cumulative_funding_rate = reader.read_i64()?;
    let start_interval_timestamp = reader.read_u64()?;
    let last_funding_update_timestamp = reader.read_u64()?;
    let funding_interval_seconds = reader.read_u64()?;
    let funding_period_seconds = reader.read_u64()?;
    let max_funding_rate = reader.read_i64()?;
    reader.skip(1)?;
    reader.skip(15)?;
    Ok(FundingAccumulator {
        acc,
        last_diff,
        cumulative_funding_rate,
        start_interval_timestamp,
        last_funding_update_timestamp,
        funding_interval_seconds,
        funding_period_seconds,
        max_funding_rate,
    })
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
