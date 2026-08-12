//! Flicker/TWAP instruction construction.

use solana_pubkey::Pubkey;

use crate::constants::{PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID};
use crate::discriminants::FlickerInstruction;
use crate::error::PhoenixIxError;
use crate::hawkeye::HAWKEYE_PROGRAM_ID;
use crate::types::{AccountMeta, Instruction, OrderFlags, SelfTradeBehavior, Side};

/// The Flicker program ID.
pub const FLICKER_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("FLickryJ5sBSyBRj7sSzMnsP93jvPxEqNKjvG49q9CEU");

/// TWAP global-state PDA seed.
pub const TWAP_GLOBAL_STATE_SEED: &[u8] = b"global_state";
/// TWAP order-account PDA seed.
pub const TWAP_ACCOUNT_SEED: &[u8] = b"flicker";
/// TWAP log-authority PDA seed.
pub const TWAP_LOG_AUTHORITY_SEED: &[u8] = b"log";

/// Fixed Flicker IOC packet discriminant.
pub const TWAP_IOC_ORDER_PACKET_DISCRIMINANT: u64 = 1;
/// Fixed byte length of Flicker's IOC order packet.
pub const TWAP_IOC_ORDER_PACKET_LEN: usize = 152;

/// User-facing alias for the Flicker instruction discriminant enum.
pub type TwapInstruction = FlickerInstruction;

/// Derives the Flicker global-state PDA for a Phoenix program.
///
/// Seeds: `[phoenix_program_id, "global_state"]` against the Flicker program.
pub fn get_twap_global_state_address(
    phoenix_program_id: &Pubkey,
) -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(
        &[phoenix_program_id.as_ref(), TWAP_GLOBAL_STATE_SEED],
        &FLICKER_PROGRAM_ID,
    )
}

/// Derives a Flicker TWAP account PDA.
///
/// Seeds: `["flicker", phoenix_program_id, trader_authority,
/// [trader_pda_index, trader_subaccount_index], market_id_le]`.
pub fn get_twap_account_address(
    phoenix_program_id: &Pubkey,
    trader_authority: &Pubkey,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    market_id: u32,
) -> Result<Pubkey, PhoenixIxError> {
    let trader_index_seed = [trader_pda_index, trader_subaccount_index];
    let market_id_seed = market_id.to_le_bytes();
    derive_program_address(
        &[
            TWAP_ACCOUNT_SEED,
            phoenix_program_id.as_ref(),
            trader_authority.as_ref(),
            &trader_index_seed,
            &market_id_seed,
        ],
        &FLICKER_PROGRAM_ID,
    )
}

/// Derives the Flicker log-authority PDA.
///
/// Seeds: `["log"]` against the Flicker program.
pub fn get_twap_log_authority_address() -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(&[TWAP_LOG_AUTHORITY_SEED], &FLICKER_PROGRAM_ID)
}

fn derive_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Result<Pubkey, PhoenixIxError> {
    let (pda, _bump) = Pubkey::find_program_address(seeds, program_id);
    Ok(pda)
}

/// Fixed-layout IOC order packet used inside Flicker TWAP instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TwapIocOrderPacket {
    side: Side,
    price_in_ticks: Option<u64>,
    num_base_lots: u64,
    num_quote_lots: Option<u64>,
    min_base_lots_to_fill: u64,
    min_quote_lots_to_fill: u64,
    self_trade_behavior: SelfTradeBehavior,
    match_limit: Option<u64>,
    client_order_id: u128,
    last_valid_slot: Option<u64>,
    order_flags: OrderFlags,
    cancel_existing: bool,
}

impl TwapIocOrderPacket {
    pub fn builder() -> TwapIocOrderPacketBuilder {
        TwapIocOrderPacketBuilder::new()
    }

    pub fn side(&self) -> Side {
        self.side
    }

    pub fn price_in_ticks(&self) -> Option<u64> {
        self.price_in_ticks
    }

    pub fn num_base_lots(&self) -> u64 {
        self.num_base_lots
    }

    pub fn num_quote_lots(&self) -> Option<u64> {
        self.num_quote_lots
    }

    pub fn min_base_lots_to_fill(&self) -> u64 {
        self.min_base_lots_to_fill
    }

    pub fn min_quote_lots_to_fill(&self) -> u64 {
        self.min_quote_lots_to_fill
    }

    pub fn self_trade_behavior(&self) -> SelfTradeBehavior {
        self.self_trade_behavior
    }

    pub fn match_limit(&self) -> Option<u64> {
        self.match_limit
    }

    pub fn client_order_id(&self) -> u128 {
        self.client_order_id
    }

    pub fn last_valid_slot(&self) -> Option<u64> {
        self.last_valid_slot
    }

    pub fn order_flags(&self) -> OrderFlags {
        self.order_flags
    }

    pub fn cancel_existing(&self) -> bool {
        self.cancel_existing
    }
}

/// Builder for [`TwapIocOrderPacket`].
#[derive(Default)]
pub struct TwapIocOrderPacketBuilder {
    side: Option<Side>,
    price_in_ticks: Option<u64>,
    num_base_lots: Option<u64>,
    num_quote_lots: Option<u64>,
    min_base_lots_to_fill: Option<u64>,
    min_quote_lots_to_fill: Option<u64>,
    self_trade_behavior: Option<SelfTradeBehavior>,
    match_limit: Option<u64>,
    client_order_id: Option<u128>,
    last_valid_slot: Option<u64>,
    order_flags: Option<OrderFlags>,
    cancel_existing: Option<bool>,
}

impl TwapIocOrderPacketBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    pub fn price_in_ticks(mut self, price_in_ticks: u64) -> Self {
        self.price_in_ticks = Some(price_in_ticks);
        self
    }

    pub fn num_base_lots(mut self, num_base_lots: u64) -> Self {
        self.num_base_lots = Some(num_base_lots);
        self
    }

    pub fn num_quote_lots(mut self, num_quote_lots: u64) -> Self {
        self.num_quote_lots = Some(num_quote_lots);
        self
    }

    pub fn min_base_lots_to_fill(mut self, min_base_lots_to_fill: u64) -> Self {
        self.min_base_lots_to_fill = Some(min_base_lots_to_fill);
        self
    }

    pub fn min_quote_lots_to_fill(mut self, min_quote_lots_to_fill: u64) -> Self {
        self.min_quote_lots_to_fill = Some(min_quote_lots_to_fill);
        self
    }

    pub fn self_trade_behavior(mut self, self_trade_behavior: SelfTradeBehavior) -> Self {
        self.self_trade_behavior = Some(self_trade_behavior);
        self
    }

    pub fn match_limit(mut self, match_limit: u64) -> Self {
        self.match_limit = Some(match_limit);
        self
    }

    pub fn client_order_id(mut self, client_order_id: u128) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }

    pub fn last_valid_slot(mut self, last_valid_slot: u64) -> Self {
        self.last_valid_slot = Some(last_valid_slot);
        self
    }

    pub fn order_flags(mut self, order_flags: OrderFlags) -> Self {
        self.order_flags = Some(order_flags);
        self
    }

    pub fn cancel_existing(mut self, cancel_existing: bool) -> Self {
        self.cancel_existing = Some(cancel_existing);
        self
    }

    pub fn build(self) -> Result<TwapIocOrderPacket, PhoenixIxError> {
        validate_optional_nonzero("price_in_ticks", self.price_in_ticks)?;
        validate_optional_nonzero("num_quote_lots", self.num_quote_lots)?;
        validate_optional_nonzero("match_limit", self.match_limit)?;
        validate_optional_nonzero("last_valid_slot", self.last_valid_slot)?;

        Ok(TwapIocOrderPacket {
            side: self.side.ok_or(PhoenixIxError::MissingField("side"))?,
            price_in_ticks: self.price_in_ticks,
            num_base_lots: self
                .num_base_lots
                .ok_or(PhoenixIxError::MissingField("num_base_lots"))?,
            num_quote_lots: self.num_quote_lots,
            min_base_lots_to_fill: self.min_base_lots_to_fill.unwrap_or(0),
            min_quote_lots_to_fill: self.min_quote_lots_to_fill.unwrap_or(0),
            self_trade_behavior: self.self_trade_behavior.unwrap_or(SelfTradeBehavior::Abort),
            match_limit: self.match_limit,
            client_order_id: self.client_order_id.unwrap_or(0),
            last_valid_slot: self.last_valid_slot,
            order_flags: self.order_flags.unwrap_or(OrderFlags::None),
            cancel_existing: self.cancel_existing.unwrap_or(false),
        })
    }
}

/// Parameters for creating a TWAP account.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateTwapAccountParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_global_state: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    phoenix_program_id: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    payer: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_log_authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    flicker_program_id: Pubkey,
    market_id: u32,
}

impl CreateTwapAccountParams {
    pub fn builder() -> CreateTwapAccountParamsBuilder {
        CreateTwapAccountParamsBuilder::new()
    }

    pub fn twap_global_state(&self) -> Pubkey {
        self.twap_global_state
    }

    pub fn phoenix_program_id(&self) -> Pubkey {
        self.phoenix_program_id
    }

    pub fn twap_account(&self) -> Pubkey {
        self.twap_account
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn payer(&self) -> Pubkey {
        self.payer
    }

    pub fn twap_log_authority(&self) -> Pubkey {
        self.twap_log_authority
    }

    pub fn flicker_program_id(&self) -> Pubkey {
        self.flicker_program_id
    }

    pub fn market_id(&self) -> u32 {
        self.market_id
    }
}

#[derive(Default)]
pub struct CreateTwapAccountParamsBuilder {
    twap_global_state: Option<Pubkey>,
    phoenix_program_id: Option<Pubkey>,
    twap_account: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    payer: Option<Pubkey>,
    twap_log_authority: Option<Pubkey>,
    flicker_program_id: Option<Pubkey>,
    market_id: Option<u32>,
}

impl CreateTwapAccountParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn twap_global_state(mut self, twap_global_state: Pubkey) -> Self {
        self.twap_global_state = Some(twap_global_state);
        self
    }

    pub fn phoenix_program_id(mut self, phoenix_program_id: Pubkey) -> Self {
        self.phoenix_program_id = Some(phoenix_program_id);
        self
    }

    pub fn twap_account(mut self, twap_account: Pubkey) -> Self {
        self.twap_account = Some(twap_account);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn twap_log_authority(mut self, twap_log_authority: Pubkey) -> Self {
        self.twap_log_authority = Some(twap_log_authority);
        self
    }

    pub fn flicker_program_id(mut self, flicker_program_id: Pubkey) -> Self {
        self.flicker_program_id = Some(flicker_program_id);
        self
    }

    pub fn market_id(mut self, market_id: u32) -> Self {
        self.market_id = Some(market_id);
        self
    }

    pub fn build(self) -> Result<CreateTwapAccountParams, PhoenixIxError> {
        let phoenix_program_id = self.phoenix_program_id.unwrap_or(*PHOENIX_PROGRAM_ID);
        let flicker_program_id = self.flicker_program_id.unwrap_or(FLICKER_PROGRAM_ID);
        Ok(CreateTwapAccountParams {
            twap_global_state: match self.twap_global_state {
                Some(twap_global_state) => twap_global_state,
                None => get_twap_global_state_address(&phoenix_program_id)?,
            },
            phoenix_program_id,
            twap_account: self
                .twap_account
                .ok_or(PhoenixIxError::MissingField("twap_account"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            twap_log_authority: match self.twap_log_authority {
                Some(twap_log_authority) => twap_log_authority,
                None => get_twap_log_authority_address()?,
            },
            flicker_program_id,
            market_id: self
                .market_id
                .ok_or(PhoenixIxError::MissingField("market_id"))?,
        })
    }
}

/// Parameters for placing a TWAP order.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlaceTwapOrderParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_global_state: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    phoenix_program_id: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    hawkeye_program_id: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_log_authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    flicker_program_id: Pubkey,
    cooldown_slots: u64,
    n_child_orders: u64,
    child_order_max_slippage_bps: u64,
    child_order_min_price_in_ticks: Option<u64>,
    child_order_max_price_in_ticks: Option<u64>,
    child_order_packet: TwapIocOrderPacket,
    child_order_collateral_quote_lots_to_transfer: Option<u64>,
    last_valid_slot: Option<u64>,
    transfer_accounts: Vec<AccountMeta>,
    order_accounts: Vec<AccountMeta>,
}

impl PlaceTwapOrderParams {
    pub fn builder() -> PlaceTwapOrderParamsBuilder {
        PlaceTwapOrderParamsBuilder::new()
    }

    pub fn twap_global_state(&self) -> Pubkey {
        self.twap_global_state
    }

    pub fn phoenix_program_id(&self) -> Pubkey {
        self.phoenix_program_id
    }

    pub fn hawkeye_program_id(&self) -> Pubkey {
        self.hawkeye_program_id
    }

    pub fn twap_log_authority(&self) -> Pubkey {
        self.twap_log_authority
    }

    pub fn twap_account(&self) -> Pubkey {
        self.twap_account
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn flicker_program_id(&self) -> Pubkey {
        self.flicker_program_id
    }

    pub fn cooldown_slots(&self) -> u64 {
        self.cooldown_slots
    }

    pub fn n_child_orders(&self) -> u64 {
        self.n_child_orders
    }

    pub fn child_order_max_slippage_bps(&self) -> u64 {
        self.child_order_max_slippage_bps
    }

    pub fn child_order_min_price_in_ticks(&self) -> Option<u64> {
        self.child_order_min_price_in_ticks
    }

    pub fn child_order_max_price_in_ticks(&self) -> Option<u64> {
        self.child_order_max_price_in_ticks
    }

    pub fn child_order_packet(&self) -> &TwapIocOrderPacket {
        &self.child_order_packet
    }

    pub fn child_order_collateral_quote_lots_to_transfer(&self) -> Option<u64> {
        self.child_order_collateral_quote_lots_to_transfer
    }

    pub fn last_valid_slot(&self) -> Option<u64> {
        self.last_valid_slot
    }

    pub fn transfer_accounts(&self) -> &[AccountMeta] {
        &self.transfer_accounts
    }

    pub fn order_accounts(&self) -> &[AccountMeta] {
        &self.order_accounts
    }
}

#[derive(Default)]
pub struct PlaceTwapOrderParamsBuilder {
    twap_global_state: Option<Pubkey>,
    phoenix_program_id: Option<Pubkey>,
    hawkeye_program_id: Option<Pubkey>,
    twap_log_authority: Option<Pubkey>,
    twap_account: Option<Pubkey>,
    authority: Option<Pubkey>,
    flicker_program_id: Option<Pubkey>,
    cooldown_slots: Option<u64>,
    n_child_orders: Option<u64>,
    child_order_max_slippage_bps: Option<u64>,
    child_order_min_price_in_ticks: Option<u64>,
    child_order_max_price_in_ticks: Option<u64>,
    child_order_packet: Option<TwapIocOrderPacket>,
    child_order_collateral_quote_lots_to_transfer: Option<u64>,
    last_valid_slot: Option<u64>,
    transfer_accounts: Option<Vec<AccountMeta>>,
    order_accounts: Option<Vec<AccountMeta>>,
}

impl PlaceTwapOrderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn twap_global_state(mut self, twap_global_state: Pubkey) -> Self {
        self.twap_global_state = Some(twap_global_state);
        self
    }

    pub fn phoenix_program_id(mut self, phoenix_program_id: Pubkey) -> Self {
        self.phoenix_program_id = Some(phoenix_program_id);
        self
    }

    pub fn hawkeye_program_id(mut self, hawkeye_program_id: Pubkey) -> Self {
        self.hawkeye_program_id = Some(hawkeye_program_id);
        self
    }

    pub fn twap_log_authority(mut self, twap_log_authority: Pubkey) -> Self {
        self.twap_log_authority = Some(twap_log_authority);
        self
    }

    pub fn twap_account(mut self, twap_account: Pubkey) -> Self {
        self.twap_account = Some(twap_account);
        self
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn flicker_program_id(mut self, flicker_program_id: Pubkey) -> Self {
        self.flicker_program_id = Some(flicker_program_id);
        self
    }

    pub fn cooldown_slots(mut self, cooldown_slots: u64) -> Self {
        self.cooldown_slots = Some(cooldown_slots);
        self
    }

    pub fn n_child_orders(mut self, n_child_orders: u64) -> Self {
        self.n_child_orders = Some(n_child_orders);
        self
    }

    pub fn child_order_max_slippage_bps(mut self, child_order_max_slippage_bps: u64) -> Self {
        self.child_order_max_slippage_bps = Some(child_order_max_slippage_bps);
        self
    }

    pub fn child_order_min_price_in_ticks(mut self, price_in_ticks: u64) -> Self {
        self.child_order_min_price_in_ticks = Some(price_in_ticks);
        self
    }

    pub fn child_order_max_price_in_ticks(mut self, price_in_ticks: u64) -> Self {
        self.child_order_max_price_in_ticks = Some(price_in_ticks);
        self
    }

    pub fn child_order_packet(mut self, child_order_packet: TwapIocOrderPacket) -> Self {
        self.child_order_packet = Some(child_order_packet);
        self
    }

    pub fn child_order_collateral_quote_lots_to_transfer(mut self, quote_lots: u64) -> Self {
        self.child_order_collateral_quote_lots_to_transfer = Some(quote_lots);
        self
    }

    pub fn last_valid_slot(mut self, last_valid_slot: u64) -> Self {
        self.last_valid_slot = Some(last_valid_slot);
        self
    }

    pub fn transfer_accounts(mut self, transfer_accounts: Vec<AccountMeta>) -> Self {
        self.transfer_accounts = Some(transfer_accounts);
        self
    }

    pub fn order_accounts(mut self, order_accounts: Vec<AccountMeta>) -> Self {
        self.order_accounts = Some(order_accounts);
        self
    }

    pub fn build(self) -> Result<PlaceTwapOrderParams, PhoenixIxError> {
        validate_optional_nonzero(
            "child_order_min_price_in_ticks",
            self.child_order_min_price_in_ticks,
        )?;
        validate_optional_nonzero(
            "child_order_max_price_in_ticks",
            self.child_order_max_price_in_ticks,
        )?;
        validate_optional_nonzero(
            "child_order_collateral_quote_lots_to_transfer",
            self.child_order_collateral_quote_lots_to_transfer,
        )?;

        let phoenix_program_id = self.phoenix_program_id.unwrap_or(*PHOENIX_PROGRAM_ID);
        Ok(PlaceTwapOrderParams {
            twap_global_state: match self.twap_global_state {
                Some(twap_global_state) => twap_global_state,
                None => get_twap_global_state_address(&phoenix_program_id)?,
            },
            phoenix_program_id,
            hawkeye_program_id: self.hawkeye_program_id.unwrap_or(HAWKEYE_PROGRAM_ID),
            twap_log_authority: match self.twap_log_authority {
                Some(twap_log_authority) => twap_log_authority,
                None => get_twap_log_authority_address()?,
            },
            twap_account: self
                .twap_account
                .ok_or(PhoenixIxError::MissingField("twap_account"))?,
            authority: self
                .authority
                .ok_or(PhoenixIxError::MissingField("authority"))?,
            flicker_program_id: self.flicker_program_id.unwrap_or(FLICKER_PROGRAM_ID),
            cooldown_slots: self
                .cooldown_slots
                .ok_or(PhoenixIxError::MissingField("cooldown_slots"))?,
            n_child_orders: self
                .n_child_orders
                .ok_or(PhoenixIxError::MissingField("n_child_orders"))?,
            child_order_max_slippage_bps: self
                .child_order_max_slippage_bps
                .ok_or(PhoenixIxError::MissingField("child_order_max_slippage_bps"))?,
            child_order_min_price_in_ticks: self.child_order_min_price_in_ticks,
            child_order_max_price_in_ticks: self.child_order_max_price_in_ticks,
            child_order_packet: self
                .child_order_packet
                .ok_or(PhoenixIxError::MissingField("child_order_packet"))?,
            child_order_collateral_quote_lots_to_transfer: self
                .child_order_collateral_quote_lots_to_transfer,
            last_valid_slot: self.last_valid_slot,
            transfer_accounts: self.transfer_accounts.unwrap_or_default(),
            order_accounts: self
                .order_accounts
                .ok_or(PhoenixIxError::MissingField("order_accounts"))?,
        })
    }
}

/// Parameters for executing a scheduled TWAP child order.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecuteTwapOrderParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_global_state: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    phoenix_program_id: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    hawkeye_program_id: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_log_authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    flicker_program_id: Pubkey,
    transfer_accounts: Vec<AccountMeta>,
    order_accounts: Vec<AccountMeta>,
}

impl ExecuteTwapOrderParams {
    pub fn builder() -> ExecuteTwapOrderParamsBuilder {
        ExecuteTwapOrderParamsBuilder::new()
    }

    pub fn twap_global_state(&self) -> Pubkey {
        self.twap_global_state
    }

    pub fn phoenix_program_id(&self) -> Pubkey {
        self.phoenix_program_id
    }

    pub fn hawkeye_program_id(&self) -> Pubkey {
        self.hawkeye_program_id
    }

    pub fn twap_log_authority(&self) -> Pubkey {
        self.twap_log_authority
    }

    pub fn twap_account(&self) -> Pubkey {
        self.twap_account
    }

    pub fn flicker_program_id(&self) -> Pubkey {
        self.flicker_program_id
    }

    pub fn transfer_accounts(&self) -> &[AccountMeta] {
        &self.transfer_accounts
    }

    pub fn order_accounts(&self) -> &[AccountMeta] {
        &self.order_accounts
    }
}

#[derive(Default)]
pub struct ExecuteTwapOrderParamsBuilder {
    twap_global_state: Option<Pubkey>,
    phoenix_program_id: Option<Pubkey>,
    hawkeye_program_id: Option<Pubkey>,
    twap_log_authority: Option<Pubkey>,
    twap_account: Option<Pubkey>,
    flicker_program_id: Option<Pubkey>,
    transfer_accounts: Option<Vec<AccountMeta>>,
    order_accounts: Option<Vec<AccountMeta>>,
}

impl ExecuteTwapOrderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn twap_global_state(mut self, twap_global_state: Pubkey) -> Self {
        self.twap_global_state = Some(twap_global_state);
        self
    }

    pub fn phoenix_program_id(mut self, phoenix_program_id: Pubkey) -> Self {
        self.phoenix_program_id = Some(phoenix_program_id);
        self
    }

    pub fn hawkeye_program_id(mut self, hawkeye_program_id: Pubkey) -> Self {
        self.hawkeye_program_id = Some(hawkeye_program_id);
        self
    }

    pub fn twap_log_authority(mut self, twap_log_authority: Pubkey) -> Self {
        self.twap_log_authority = Some(twap_log_authority);
        self
    }

    pub fn twap_account(mut self, twap_account: Pubkey) -> Self {
        self.twap_account = Some(twap_account);
        self
    }

    pub fn flicker_program_id(mut self, flicker_program_id: Pubkey) -> Self {
        self.flicker_program_id = Some(flicker_program_id);
        self
    }

    pub fn transfer_accounts(mut self, transfer_accounts: Vec<AccountMeta>) -> Self {
        self.transfer_accounts = Some(transfer_accounts);
        self
    }

    pub fn order_accounts(mut self, order_accounts: Vec<AccountMeta>) -> Self {
        self.order_accounts = Some(order_accounts);
        self
    }

    pub fn build(self) -> Result<ExecuteTwapOrderParams, PhoenixIxError> {
        let phoenix_program_id = self.phoenix_program_id.unwrap_or(*PHOENIX_PROGRAM_ID);
        Ok(ExecuteTwapOrderParams {
            twap_global_state: match self.twap_global_state {
                Some(twap_global_state) => twap_global_state,
                None => get_twap_global_state_address(&phoenix_program_id)?,
            },
            phoenix_program_id,
            hawkeye_program_id: self.hawkeye_program_id.unwrap_or(HAWKEYE_PROGRAM_ID),
            twap_log_authority: match self.twap_log_authority {
                Some(twap_log_authority) => twap_log_authority,
                None => get_twap_log_authority_address()?,
            },
            twap_account: self
                .twap_account
                .ok_or(PhoenixIxError::MissingField("twap_account"))?,
            flicker_program_id: self.flicker_program_id.unwrap_or(FLICKER_PROGRAM_ID),
            transfer_accounts: self.transfer_accounts.unwrap_or_default(),
            order_accounts: self
                .order_accounts
                .ok_or(PhoenixIxError::MissingField("order_accounts"))?,
        })
    }
}

/// Parameters for cancelling an active TWAP order.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CancelTwapOrderParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_global_state: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    phoenix_program_id: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_log_authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    flicker_program_id: Pubkey,
}

impl CancelTwapOrderParams {
    pub fn builder() -> CancelTwapOrderParamsBuilder {
        CancelTwapOrderParamsBuilder::new()
    }

    pub fn twap_global_state(&self) -> Pubkey {
        self.twap_global_state
    }

    pub fn phoenix_program_id(&self) -> Pubkey {
        self.phoenix_program_id
    }

    pub fn twap_log_authority(&self) -> Pubkey {
        self.twap_log_authority
    }

    pub fn twap_account(&self) -> Pubkey {
        self.twap_account
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn flicker_program_id(&self) -> Pubkey {
        self.flicker_program_id
    }
}

#[derive(Default)]
pub struct CancelTwapOrderParamsBuilder {
    twap_global_state: Option<Pubkey>,
    phoenix_program_id: Option<Pubkey>,
    twap_log_authority: Option<Pubkey>,
    twap_account: Option<Pubkey>,
    authority: Option<Pubkey>,
    flicker_program_id: Option<Pubkey>,
}

impl CancelTwapOrderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn twap_global_state(mut self, twap_global_state: Pubkey) -> Self {
        self.twap_global_state = Some(twap_global_state);
        self
    }

    pub fn phoenix_program_id(mut self, phoenix_program_id: Pubkey) -> Self {
        self.phoenix_program_id = Some(phoenix_program_id);
        self
    }

    pub fn twap_log_authority(mut self, twap_log_authority: Pubkey) -> Self {
        self.twap_log_authority = Some(twap_log_authority);
        self
    }

    pub fn twap_account(mut self, twap_account: Pubkey) -> Self {
        self.twap_account = Some(twap_account);
        self
    }

    pub fn authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn flicker_program_id(mut self, flicker_program_id: Pubkey) -> Self {
        self.flicker_program_id = Some(flicker_program_id);
        self
    }

    pub fn build(self) -> Result<CancelTwapOrderParams, PhoenixIxError> {
        let phoenix_program_id = self.phoenix_program_id.unwrap_or(*PHOENIX_PROGRAM_ID);
        Ok(CancelTwapOrderParams {
            twap_global_state: match self.twap_global_state {
                Some(twap_global_state) => twap_global_state,
                None => get_twap_global_state_address(&phoenix_program_id)?,
            },
            phoenix_program_id,
            twap_log_authority: match self.twap_log_authority {
                Some(twap_log_authority) => twap_log_authority,
                None => get_twap_log_authority_address()?,
            },
            twap_account: self
                .twap_account
                .ok_or(PhoenixIxError::MissingField("twap_account"))?,
            authority: self
                .authority
                .ok_or(PhoenixIxError::MissingField("authority"))?,
            flicker_program_id: self.flicker_program_id.unwrap_or(FLICKER_PROGRAM_ID),
        })
    }
}

/// Parameters for closing an inactive TWAP account.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CloseInactiveTwapAccountParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_global_state: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    phoenix_program_id: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    recipient: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    twap_log_authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    flicker_program_id: Pubkey,
}

impl CloseInactiveTwapAccountParams {
    pub fn builder() -> CloseInactiveTwapAccountParamsBuilder {
        CloseInactiveTwapAccountParamsBuilder::new()
    }

    pub fn twap_global_state(&self) -> Pubkey {
        self.twap_global_state
    }

    pub fn phoenix_program_id(&self) -> Pubkey {
        self.phoenix_program_id
    }

    pub fn twap_account(&self) -> Pubkey {
        self.twap_account
    }

    pub fn recipient(&self) -> Pubkey {
        self.recipient
    }

    pub fn twap_log_authority(&self) -> Pubkey {
        self.twap_log_authority
    }

    pub fn flicker_program_id(&self) -> Pubkey {
        self.flicker_program_id
    }
}

#[derive(Default)]
pub struct CloseInactiveTwapAccountParamsBuilder {
    twap_global_state: Option<Pubkey>,
    phoenix_program_id: Option<Pubkey>,
    twap_account: Option<Pubkey>,
    recipient: Option<Pubkey>,
    twap_log_authority: Option<Pubkey>,
    flicker_program_id: Option<Pubkey>,
}

impl CloseInactiveTwapAccountParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn twap_global_state(mut self, twap_global_state: Pubkey) -> Self {
        self.twap_global_state = Some(twap_global_state);
        self
    }

    pub fn phoenix_program_id(mut self, phoenix_program_id: Pubkey) -> Self {
        self.phoenix_program_id = Some(phoenix_program_id);
        self
    }

    pub fn twap_account(mut self, twap_account: Pubkey) -> Self {
        self.twap_account = Some(twap_account);
        self
    }

    pub fn recipient(mut self, recipient: Pubkey) -> Self {
        self.recipient = Some(recipient);
        self
    }

    pub fn twap_log_authority(mut self, twap_log_authority: Pubkey) -> Self {
        self.twap_log_authority = Some(twap_log_authority);
        self
    }

    pub fn flicker_program_id(mut self, flicker_program_id: Pubkey) -> Self {
        self.flicker_program_id = Some(flicker_program_id);
        self
    }

    pub fn build(self) -> Result<CloseInactiveTwapAccountParams, PhoenixIxError> {
        let phoenix_program_id = self.phoenix_program_id.unwrap_or(*PHOENIX_PROGRAM_ID);
        Ok(CloseInactiveTwapAccountParams {
            twap_global_state: match self.twap_global_state {
                Some(twap_global_state) => twap_global_state,
                None => get_twap_global_state_address(&phoenix_program_id)?,
            },
            phoenix_program_id,
            twap_account: self
                .twap_account
                .ok_or(PhoenixIxError::MissingField("twap_account"))?,
            recipient: self
                .recipient
                .ok_or(PhoenixIxError::MissingField("recipient"))?,
            twap_log_authority: match self.twap_log_authority {
                Some(twap_log_authority) => twap_log_authority,
                None => get_twap_log_authority_address()?,
            },
            flicker_program_id: self.flicker_program_id.unwrap_or(FLICKER_PROGRAM_ID),
        })
    }
}

/// Create a Flicker `CreateTWAPAccount` instruction.
pub fn create_twap_account_ix(
    params: CreateTwapAccountParams,
) -> Result<Instruction, PhoenixIxError> {
    let mut data = FlickerInstruction::CreateTWAPAccount
        .discriminant()
        .to_vec();
    push_u32(&mut data, params.market_id());

    let accounts = vec![
        AccountMeta::readonly(params.twap_global_state()),
        AccountMeta::readonly(params.phoenix_program_id()),
        AccountMeta::writable(params.twap_account()),
        AccountMeta::readonly(params.trader_account()),
        AccountMeta::writable_signer(params.payer()),
        AccountMeta::readonly(SYSTEM_PROGRAM_ID),
        AccountMeta::readonly(params.twap_log_authority()),
        AccountMeta::readonly(params.flicker_program_id()),
    ];

    Ok(Instruction {
        program_id: params.flicker_program_id(),
        accounts,
        data,
    })
}

/// Create a Flicker `PlaceTWAPOrder` instruction.
pub fn create_place_twap_order_ix(
    params: PlaceTwapOrderParams,
) -> Result<Instruction, PhoenixIxError> {
    let transfer_collateral_account_count = transfer_account_count(params.transfer_accounts())?;
    let data = encode_place_twap_order(&params, transfer_collateral_account_count)?;

    let mut accounts = vec![
        AccountMeta::readonly(params.twap_global_state()),
        AccountMeta::readonly(params.phoenix_program_id()),
        AccountMeta::readonly(params.hawkeye_program_id()),
        AccountMeta::readonly(params.twap_log_authority()),
        AccountMeta::writable(params.twap_account()),
        AccountMeta::readonly_signer(params.authority()),
        AccountMeta::readonly(params.flicker_program_id()),
    ];
    accounts.extend_from_slice(params.transfer_accounts());
    accounts.extend_from_slice(params.order_accounts());

    Ok(Instruction {
        program_id: params.flicker_program_id(),
        accounts,
        data,
    })
}

/// Create a Flicker `ExecuteTWAPOrder` instruction.
pub fn create_execute_twap_order_ix(
    params: ExecuteTwapOrderParams,
) -> Result<Instruction, PhoenixIxError> {
    let transfer_collateral_account_count = transfer_account_count(params.transfer_accounts())?;
    let mut data = FlickerInstruction::ExecuteTWAPOrder.discriminant().to_vec();
    push_u8(&mut data, transfer_collateral_account_count);

    let mut accounts = vec![
        AccountMeta::readonly(params.twap_global_state()),
        AccountMeta::readonly(params.phoenix_program_id()),
        AccountMeta::readonly(params.hawkeye_program_id()),
        AccountMeta::readonly(params.twap_log_authority()),
        AccountMeta::writable(params.twap_account()),
        AccountMeta::readonly(params.flicker_program_id()),
    ];
    accounts.extend_from_slice(params.transfer_accounts());
    accounts.extend_from_slice(params.order_accounts());

    Ok(Instruction {
        program_id: params.flicker_program_id(),
        accounts,
        data,
    })
}

/// Create a Flicker `CancelTWAPOrder` instruction.
pub fn create_cancel_twap_order_ix(
    params: CancelTwapOrderParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = FlickerInstruction::CancelTWAPOrder.discriminant().to_vec();
    let accounts = vec![
        AccountMeta::readonly(params.twap_global_state()),
        AccountMeta::readonly(params.phoenix_program_id()),
        AccountMeta::readonly(params.twap_log_authority()),
        AccountMeta::writable(params.twap_account()),
        AccountMeta::readonly_signer(params.authority()),
        AccountMeta::readonly(params.flicker_program_id()),
    ];

    Ok(Instruction {
        program_id: params.flicker_program_id(),
        accounts,
        data,
    })
}

/// Create a Flicker `CloseInactiveTWAPAccount` instruction.
pub fn create_close_inactive_twap_account_ix(
    params: CloseInactiveTwapAccountParams,
) -> Result<Instruction, PhoenixIxError> {
    let data = FlickerInstruction::CloseInactiveTWAPAccount
        .discriminant()
        .to_vec();
    let accounts = vec![
        AccountMeta::readonly(params.twap_global_state()),
        AccountMeta::readonly(params.phoenix_program_id()),
        AccountMeta::writable(params.twap_account()),
        AccountMeta::writable(params.recipient()),
        AccountMeta::readonly(params.twap_log_authority()),
        AccountMeta::readonly(params.flicker_program_id()),
    ];

    Ok(Instruction {
        program_id: params.flicker_program_id(),
        accounts,
        data,
    })
}

/// Encode the fixed-layout Flicker IOC child order packet.
pub fn encode_twap_ioc_order_packet(
    packet: &TwapIocOrderPacket,
) -> Result<Vec<u8>, PhoenixIxError> {
    let mut data = Vec::with_capacity(TWAP_IOC_ORDER_PACKET_LEN);
    push_u64(&mut data, TWAP_IOC_ORDER_PACKET_DISCRIMINANT);
    push_u8(&mut data, encode_side(packet.side()));
    data.extend_from_slice(&[0; 7]);
    push_optional_raw_nonzero_u64(&mut data, "price_in_ticks", packet.price_in_ticks())?;
    push_u64(&mut data, packet.num_base_lots());
    push_optional_raw_nonzero_u64(&mut data, "num_quote_lots", packet.num_quote_lots())?;
    push_u64(&mut data, packet.min_base_lots_to_fill());
    push_u64(&mut data, packet.min_quote_lots_to_fill());
    push_u8(
        &mut data,
        encode_self_trade_behavior(packet.self_trade_behavior()),
    );
    data.extend_from_slice(&[0; 7]);
    push_optional_raw_nonzero_u64(&mut data, "match_limit", packet.match_limit())?;
    data.extend_from_slice(&packet.client_order_id().to_le_bytes());
    push_optional_raw_nonzero_u64(&mut data, "last_valid_slot", packet.last_valid_slot())?;
    push_u8(&mut data, packet.order_flags() as u8);
    push_u8(&mut data, u8::from(packet.cancel_existing()));
    data.extend_from_slice(&[0; 6]);
    data.extend_from_slice(&[0; 48]);
    debug_assert_eq!(data.len(), TWAP_IOC_ORDER_PACKET_LEN);
    Ok(data)
}

fn encode_place_twap_order(
    params: &PlaceTwapOrderParams,
    transfer_collateral_account_count: u8,
) -> Result<Vec<u8>, PhoenixIxError> {
    let mut data = FlickerInstruction::PlaceTWAPOrder.discriminant().to_vec();
    push_u64(&mut data, params.cooldown_slots());
    push_u64(&mut data, params.n_child_orders());
    push_u64(&mut data, params.child_order_max_slippage_bps());
    push_option_nonzero_u64(&mut data, params.child_order_min_price_in_ticks());
    push_option_nonzero_u64(&mut data, params.child_order_max_price_in_ticks());
    data.extend_from_slice(&encode_twap_ioc_order_packet(params.child_order_packet())?);
    push_option_nonzero_u64(
        &mut data,
        params.child_order_collateral_quote_lots_to_transfer(),
    );
    push_option_u64(&mut data, params.last_valid_slot());
    push_u8(&mut data, transfer_collateral_account_count);
    Ok(data)
}

fn transfer_account_count(accounts: &[AccountMeta]) -> Result<u8, PhoenixIxError> {
    u8::try_from(accounts.len()).map_err(|_| PhoenixIxError::InstructionDataTooLarge)
}

fn validate_optional_nonzero(
    field: &'static str,
    value: Option<u64>,
) -> Result<(), PhoenixIxError> {
    if value == Some(0) {
        return Err(PhoenixIxError::InvalidTwapOptionalU64 { field });
    }
    Ok(())
}

fn push_optional_raw_nonzero_u64(
    data: &mut Vec<u8>,
    field: &'static str,
    value: Option<u64>,
) -> Result<(), PhoenixIxError> {
    validate_optional_nonzero(field, value)?;
    push_u64(data, value.unwrap_or(0));
    Ok(())
}

fn push_option_nonzero_u64(data: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            push_u8(data, 1);
            push_u64(data, value);
        }
        None => push_u8(data, 0),
    }
}

fn push_option_u64(data: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            push_u8(data, 1);
            push_u64(data, value);
        }
        None => push_u8(data, 0),
    }
}

fn push_u8(data: &mut Vec<u8>, value: u8) {
    data.push(value);
}

fn push_u32(data: &mut Vec<u8>, value: u32) {
    data.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(data: &mut Vec<u8>, value: u64) {
    data.extend_from_slice(&value.to_le_bytes());
}

fn encode_side(side: Side) -> u8 {
    match side {
        Side::Bid => 0,
        Side::Ask => 1,
    }
}

fn encode_self_trade_behavior(self_trade_behavior: SelfTradeBehavior) -> u8 {
    match self_trade_behavior {
        SelfTradeBehavior::Abort => 0,
        SelfTradeBehavior::CancelProvide => 1,
        SelfTradeBehavior::DecrementTake => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet() -> TwapIocOrderPacket {
        TwapIocOrderPacket::builder()
            .side(Side::Bid)
            .price_in_ticks(50_000)
            .num_base_lots(1_000)
            .min_base_lots_to_fill(1_000)
            .min_quote_lots_to_fill(1)
            .self_trade_behavior(SelfTradeBehavior::CancelProvide)
            .match_limit(25)
            .client_order_id(0x1122_3344_5566_7788_99aa_bbcc_ddee_ff00)
            .last_valid_slot(42)
            .order_flags(OrderFlags::ReduceOnly)
            .cancel_existing(true)
            .build()
            .unwrap()
    }

    #[test]
    fn derives_twap_pdas() {
        let trader_authority = Pubkey::new_unique();
        let phoenix_program_id = *PHOENIX_PROGRAM_ID;
        let global_state = get_twap_global_state_address(&phoenix_program_id).unwrap();
        let account =
            get_twap_account_address(&phoenix_program_id, &trader_authority, 2, 3, 7).unwrap();
        let log_authority = get_twap_log_authority_address().unwrap();

        assert_ne!(global_state, account);
        assert_ne!(global_state, log_authority);
        assert_ne!(account, log_authority);
    }

    #[test]
    fn encodes_fixed_twap_ioc_packet() {
        let data = encode_twap_ioc_order_packet(&packet()).unwrap();

        assert_eq!(data.len(), TWAP_IOC_ORDER_PACKET_LEN);
        assert_eq!(
            u64::from_le_bytes(data[0..8].try_into().unwrap()),
            TWAP_IOC_ORDER_PACKET_DISCRIMINANT
        );
        assert_eq!(data[8], 0);
        assert_eq!(u64::from_le_bytes(data[16..24].try_into().unwrap()), 50_000);
        assert_eq!(u64::from_le_bytes(data[24..32].try_into().unwrap()), 1_000);
        assert_eq!(data[56], 1);
        assert_eq!(u64::from_le_bytes(data[64..72].try_into().unwrap()), 25);
        assert_eq!(data[96], OrderFlags::ReduceOnly as u8);
        assert_eq!(data[97], 1);
        assert!(data[104..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn rejects_zero_optional_values() {
        let err = TwapIocOrderPacket::builder()
            .side(Side::Bid)
            .price_in_ticks(0)
            .num_base_lots(1)
            .build()
            .unwrap_err();

        assert!(matches!(
            err,
            PhoenixIxError::InvalidTwapOptionalU64 {
                field: "price_in_ticks"
            }
        ));
    }

    #[test]
    fn creates_twap_account_instruction() {
        let params = CreateTwapAccountParams::builder()
            .twap_account(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .payer(Pubkey::new_unique())
            .market_id(7)
            .build()
            .unwrap();
        let payer = params.payer();
        let ix = create_twap_account_ix(params).unwrap();

        assert_eq!(ix.program_id, FLICKER_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 8);
        assert_eq!(ix.accounts[4], AccountMeta::writable_signer(payer));
        assert_eq!(
            &ix.data[..8],
            &FlickerInstruction::CreateTWAPAccount.discriminant()
        );
        assert_eq!(u32::from_le_bytes(ix.data[8..12].try_into().unwrap()), 7);
    }

    #[test]
    fn creates_place_twap_order_instruction_with_cpi_tails() {
        let transfer_account = AccountMeta::readonly(Pubkey::new_unique());
        let order_account = AccountMeta::writable(Pubkey::new_unique());
        let params = PlaceTwapOrderParams::builder()
            .twap_account(Pubkey::new_unique())
            .authority(Pubkey::new_unique())
            .cooldown_slots(10)
            .n_child_orders(4)
            .child_order_max_slippage_bps(25)
            .child_order_packet(packet())
            .transfer_accounts(vec![transfer_account.clone()])
            .order_accounts(vec![order_account.clone()])
            .build()
            .unwrap();
        let authority = params.authority();

        let ix = create_place_twap_order_ix(params).unwrap();

        assert_eq!(ix.program_id, FLICKER_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 9);
        assert_eq!(ix.accounts[5], AccountMeta::readonly_signer(authority));
        assert_eq!(ix.accounts[7], transfer_account);
        assert_eq!(ix.accounts[8], order_account);
        assert_eq!(
            &ix.data[..8],
            &FlickerInstruction::PlaceTWAPOrder.discriminant()
        );
        assert_eq!(*ix.data.last().unwrap(), 1);
    }

    #[test]
    fn creates_execute_cancel_and_close_instructions() {
        let order_account = AccountMeta::readonly(Pubkey::new_unique());
        let execute_params = ExecuteTwapOrderParams::builder()
            .twap_account(Pubkey::new_unique())
            .order_accounts(vec![order_account.clone()])
            .build()
            .unwrap();
        let execute_ix = create_execute_twap_order_ix(execute_params).unwrap();
        assert_eq!(execute_ix.accounts.len(), 7);
        assert_eq!(execute_ix.accounts[6], order_account);
        assert_eq!(
            &execute_ix.data[..8],
            &FlickerInstruction::ExecuteTWAPOrder.discriminant()
        );
        assert_eq!(execute_ix.data[8], 0);

        let cancel_params = CancelTwapOrderParams::builder()
            .twap_account(Pubkey::new_unique())
            .authority(Pubkey::new_unique())
            .build()
            .unwrap();
        let cancel_ix = create_cancel_twap_order_ix(cancel_params).unwrap();
        assert_eq!(cancel_ix.accounts.len(), 6);
        assert_eq!(
            &cancel_ix.data[..8],
            &FlickerInstruction::CancelTWAPOrder.discriminant()
        );

        let close_params = CloseInactiveTwapAccountParams::builder()
            .twap_account(Pubkey::new_unique())
            .recipient(Pubkey::new_unique())
            .build()
            .unwrap();
        let close_ix = create_close_inactive_twap_account_ix(close_params).unwrap();
        assert_eq!(close_ix.accounts.len(), 6);
        assert_eq!(
            &close_ix.data[..8],
            &FlickerInstruction::CloseInactiveTWAPAccount.discriminant()
        );
    }
}
