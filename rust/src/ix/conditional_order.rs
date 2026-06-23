//! Conditional order instruction construction.

use borsh::{BorshDeserialize, BorshSerialize, to_vec};
use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    cancel_conditional_order_discriminant, create_conditional_orders_account_discriminant,
    get_conditional_orders_address, place_attached_conditional_order_discriminant,
    place_limit_order_with_conditionals_discriminant,
    place_position_conditional_order_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::order_packet::OrderPacket;
use crate::ix::types::{AccountMeta, Direction, FifoOrderId, Instruction, Side, StopLossOrderKind};

const MAX_CONDITIONAL_ORDER_INDEX: u8 = 191;

/// Trigger parameters for a conditional order leg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct TriggerOrderParams {
    trigger_direction: Direction,
    trade_side: Side,
    order_kind: StopLossOrderKind,
    trigger_price: u64,
    execution_price: u64,
}

impl TriggerOrderParams {
    pub fn new(
        trigger_direction: Direction,
        trade_side: Side,
        order_kind: StopLossOrderKind,
        trigger_price: u64,
        execution_price: u64,
    ) -> Self {
        Self {
            trigger_direction,
            trade_side,
            order_kind,
            trigger_price,
            execution_price,
        }
    }

    pub fn trigger_direction(&self) -> Direction {
        self.trigger_direction
    }

    pub fn trade_side(&self) -> Side {
        self.trade_side
    }

    pub fn order_kind(&self) -> StopLossOrderKind {
        self.order_kind
    }

    pub fn trigger_price(&self) -> u64 {
        self.trigger_price
    }

    pub fn execution_price(&self) -> u64 {
        self.execution_price
    }
}

/// Parameters for creating a trader conditional-orders account.
#[derive(Debug, Clone)]
pub struct CreateConditionalOrdersAccountParams {
    payer: Pubkey,
    trader_wallet: Pubkey,
    trader_account: Pubkey,
    trader_conditional_orders: Pubkey,
    capacity: u8,
}

impl CreateConditionalOrdersAccountParams {
    pub fn builder() -> CreateConditionalOrdersAccountParamsBuilder {
        CreateConditionalOrdersAccountParamsBuilder::new()
    }

    pub fn payer(&self) -> Pubkey {
        self.payer
    }

    pub fn trader_wallet(&self) -> Pubkey {
        self.trader_wallet
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn trader_conditional_orders(&self) -> Pubkey {
        self.trader_conditional_orders
    }

    pub fn capacity(&self) -> u8 {
        self.capacity
    }
}

#[derive(Default)]
pub struct CreateConditionalOrdersAccountParamsBuilder {
    payer: Option<Pubkey>,
    trader_wallet: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    trader_conditional_orders: Option<Pubkey>,
    capacity: Option<u8>,
}

impl CreateConditionalOrdersAccountParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn trader_conditional_orders(mut self, trader_conditional_orders: Pubkey) -> Self {
        self.trader_conditional_orders = Some(trader_conditional_orders);
        self
    }

    pub fn capacity(mut self, capacity: u8) -> Self {
        self.capacity = Some(capacity);
        self
    }

    pub fn build(self) -> Result<CreateConditionalOrdersAccountParams, PhoenixIxError> {
        let trader_account = self
            .trader_account
            .ok_or(PhoenixIxError::MissingField("trader_account"))?;
        let trader_conditional_orders = self
            .trader_conditional_orders
            .unwrap_or_else(|| get_conditional_orders_address(&trader_account));
        Ok(CreateConditionalOrdersAccountParams {
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            trader_account,
            trader_conditional_orders,
            capacity: self
                .capacity
                .ok_or(PhoenixIxError::MissingField("capacity"))?,
        })
    }
}

#[derive(BorshSerialize)]
struct CreateConditionalOrdersAccountData {
    capacity: u8,
}

/// Create a conditional-orders account instruction.
pub fn create_create_conditional_orders_account_ix(
    params: CreateConditionalOrdersAccountParams,
) -> Result<Instruction, PhoenixIxError> {
    if params.capacity == 0 {
        return Err(PhoenixIxError::InvalidConditionalOrdersCapacity);
    }

    let mut data = Vec::new();
    data.extend_from_slice(&create_conditional_orders_account_discriminant());
    data.extend_from_slice(
        &to_vec(&CreateConditionalOrdersAccountData {
            capacity: params.capacity,
        })
        .expect("serialization should not fail"),
    );

    let accounts = vec![
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::writable_signer(params.payer),
        AccountMeta::readonly(params.trader_wallet),
        AccountMeta::readonly(params.trader_account),
        AccountMeta::writable(params.trader_conditional_orders),
        AccountMeta::readonly(SYSTEM_PROGRAM_ID),
    ];

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Parameters for placing a position conditional order.
#[derive(Debug, Clone)]
pub struct PlacePositionConditionalOrderParams {
    payer: Pubkey,
    trader_account: Pubkey,
    perp_asset_map: Pubkey,
    orderbook: Pubkey,
    spline_collection: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    trader_wallet: Pubkey,
    trader_conditional_orders: Pubkey,
    asset_id: u32,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
    size_base_lots: Option<u64>,
    size_percent: Option<u8>,
}

impl PlacePositionConditionalOrderParams {
    pub fn builder() -> PlacePositionConditionalOrderParamsBuilder {
        PlacePositionConditionalOrderParamsBuilder::new()
    }
}

#[derive(Default)]
pub struct PlacePositionConditionalOrderParamsBuilder {
    payer: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    trader_wallet: Option<Pubkey>,
    trader_conditional_orders: Option<Pubkey>,
    asset_id: Option<u32>,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
    size_base_lots: Option<u64>,
    size_percent: Option<u8>,
}

impl PlacePositionConditionalOrderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    pub fn orderbook(mut self, orderbook: Pubkey) -> Self {
        self.orderbook = Some(orderbook);
        self
    }

    pub fn spline_collection(mut self, spline_collection: Pubkey) -> Self {
        self.spline_collection = Some(spline_collection);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn active_trader_buffer(mut self, active_trader_buffer: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(active_trader_buffer);
        self
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn position_authority(self, position_authority: Pubkey) -> Self {
        self.trader_wallet(position_authority)
    }

    pub fn trader_conditional_orders(mut self, trader_conditional_orders: Pubkey) -> Self {
        self.trader_conditional_orders = Some(trader_conditional_orders);
        self
    }

    pub fn asset_id(mut self, asset_id: u32) -> Self {
        self.asset_id = Some(asset_id);
        self
    }

    pub fn greater_trigger_order(mut self, trigger: TriggerOrderParams) -> Self {
        self.greater_trigger_order = Some(trigger);
        self
    }

    pub fn less_trigger_order(mut self, trigger: TriggerOrderParams) -> Self {
        self.less_trigger_order = Some(trigger);
        self
    }

    pub fn size_base_lots(mut self, size_base_lots: u64) -> Self {
        self.size_base_lots = Some(size_base_lots);
        self
    }

    pub fn size_percent(mut self, size_percent: u8) -> Self {
        self.size_percent = Some(size_percent);
        self
    }

    pub fn build(self) -> Result<PlacePositionConditionalOrderParams, PhoenixIxError> {
        let trader_account = self
            .trader_account
            .ok_or(PhoenixIxError::MissingField("trader_account"))?;
        let trader_conditional_orders = self
            .trader_conditional_orders
            .unwrap_or_else(|| get_conditional_orders_address(&trader_account));
        Ok(PlacePositionConditionalOrderParams {
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            trader_account,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            orderbook: self
                .orderbook
                .ok_or(PhoenixIxError::MissingField("orderbook"))?,
            spline_collection: self
                .spline_collection
                .ok_or(PhoenixIxError::MissingField("spline_collection"))?,
            global_trader_index: self
                .global_trader_index
                .ok_or(PhoenixIxError::MissingField("global_trader_index"))?,
            active_trader_buffer: self
                .active_trader_buffer
                .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?,
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            trader_conditional_orders,
            asset_id: self
                .asset_id
                .ok_or(PhoenixIxError::MissingField("asset_id"))?,
            greater_trigger_order: self.greater_trigger_order,
            less_trigger_order: self.less_trigger_order,
            size_base_lots: self.size_base_lots,
            size_percent: self.size_percent,
        })
    }
}

#[derive(BorshSerialize)]
struct PlacePositionConditionalOrderData {
    asset_id: u32,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
    size_base_lots: Option<u64>,
    size_percent: Option<u8>,
}

/// Create a place-position-conditional-order instruction.
pub fn create_place_position_conditional_order_ix(
    params: PlacePositionConditionalOrderParams,
) -> Result<Instruction, PhoenixIxError> {
    validate_triggers(
        params.greater_trigger_order.as_ref(),
        params.less_trigger_order.as_ref(),
    )?;
    validate_sizing(params.size_base_lots, params.size_percent)?;
    validate_arenas(&params.global_trader_index, &params.active_trader_buffer)?;

    let mut data = Vec::new();
    data.extend_from_slice(&place_position_conditional_order_discriminant());
    data.extend_from_slice(
        &to_vec(&PlacePositionConditionalOrderData {
            asset_id: params.asset_id,
            greater_trigger_order: params.greater_trigger_order,
            less_trigger_order: params.less_trigger_order,
            size_base_lots: params.size_base_lots,
            size_percent: params.size_percent,
        })
        .expect("serialization should not fail"),
    );

    let mut accounts = vec![
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::writable_signer(params.payer),
        AccountMeta::writable(params.trader_account),
        AccountMeta::writable(params.perp_asset_map),
    ];
    push_arenas(
        &mut accounts,
        &params.global_trader_index,
        &params.active_trader_buffer,
    );
    accounts.push(AccountMeta::writable(params.orderbook));
    accounts.push(AccountMeta::writable(params.spline_collection));
    accounts.push(AccountMeta::readonly_signer(params.trader_wallet));
    accounts.push(AccountMeta::writable(params.trader_conditional_orders));
    accounts.push(AccountMeta::readonly(SYSTEM_PROGRAM_ID));

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Parameters for placing an attached conditional order.
#[derive(Debug, Clone)]
pub struct PlaceAttachedConditionalOrderParams {
    trader_account: Pubkey,
    trader_wallet: Pubkey,
    orderbook: Pubkey,
    trader_conditional_orders: Pubkey,
    payer: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    order_id: FifoOrderId,
    asset_id: u32,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
}

impl PlaceAttachedConditionalOrderParams {
    pub fn builder() -> PlaceAttachedConditionalOrderParamsBuilder {
        PlaceAttachedConditionalOrderParamsBuilder::new()
    }
}

#[derive(Default)]
pub struct PlaceAttachedConditionalOrderParamsBuilder {
    trader_account: Option<Pubkey>,
    trader_wallet: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    trader_conditional_orders: Option<Pubkey>,
    payer: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    order_id: Option<FifoOrderId>,
    asset_id: Option<u32>,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
}

impl PlaceAttachedConditionalOrderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn position_authority(self, position_authority: Pubkey) -> Self {
        self.trader_wallet(position_authority)
    }

    pub fn orderbook(mut self, orderbook: Pubkey) -> Self {
        self.orderbook = Some(orderbook);
        self
    }

    pub fn trader_conditional_orders(mut self, trader_conditional_orders: Pubkey) -> Self {
        self.trader_conditional_orders = Some(trader_conditional_orders);
        self
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn active_trader_buffer(mut self, active_trader_buffer: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(active_trader_buffer);
        self
    }

    pub fn order_id(mut self, order_id: FifoOrderId) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub fn asset_id(mut self, asset_id: u32) -> Self {
        self.asset_id = Some(asset_id);
        self
    }

    pub fn greater_trigger_order(mut self, trigger: TriggerOrderParams) -> Self {
        self.greater_trigger_order = Some(trigger);
        self
    }

    pub fn less_trigger_order(mut self, trigger: TriggerOrderParams) -> Self {
        self.less_trigger_order = Some(trigger);
        self
    }

    pub fn build(self) -> Result<PlaceAttachedConditionalOrderParams, PhoenixIxError> {
        let trader_account = self
            .trader_account
            .ok_or(PhoenixIxError::MissingField("trader_account"))?;
        let trader_conditional_orders = self
            .trader_conditional_orders
            .unwrap_or_else(|| get_conditional_orders_address(&trader_account));
        Ok(PlaceAttachedConditionalOrderParams {
            trader_account,
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            orderbook: self
                .orderbook
                .ok_or(PhoenixIxError::MissingField("orderbook"))?,
            trader_conditional_orders,
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            global_trader_index: self
                .global_trader_index
                .ok_or(PhoenixIxError::MissingField("global_trader_index"))?,
            active_trader_buffer: self
                .active_trader_buffer
                .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?,
            order_id: self
                .order_id
                .ok_or(PhoenixIxError::MissingField("order_id"))?,
            asset_id: self
                .asset_id
                .ok_or(PhoenixIxError::MissingField("asset_id"))?,
            greater_trigger_order: self.greater_trigger_order,
            less_trigger_order: self.less_trigger_order,
        })
    }
}

#[derive(BorshSerialize)]
struct PlaceAttachedConditionalOrderData {
    order_id: FifoOrderId,
    asset_id: u32,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
}

/// Create a place-attached-conditional-order instruction.
pub fn create_place_attached_conditional_order_ix(
    params: PlaceAttachedConditionalOrderParams,
) -> Result<Instruction, PhoenixIxError> {
    validate_triggers(
        params.greater_trigger_order.as_ref(),
        params.less_trigger_order.as_ref(),
    )?;
    validate_arenas(&params.global_trader_index, &params.active_trader_buffer)?;

    let mut data = Vec::new();
    data.extend_from_slice(&place_attached_conditional_order_discriminant());
    data.extend_from_slice(
        &to_vec(&PlaceAttachedConditionalOrderData {
            order_id: params.order_id,
            asset_id: params.asset_id,
            greater_trigger_order: params.greater_trigger_order,
            less_trigger_order: params.less_trigger_order,
        })
        .expect("serialization should not fail"),
    );

    let mut accounts = vec![
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::writable(params.trader_account),
        AccountMeta::readonly_signer(params.trader_wallet),
        AccountMeta::writable(params.orderbook),
        AccountMeta::writable(params.trader_conditional_orders),
        AccountMeta::writable_signer(params.payer),
    ];
    push_arenas(
        &mut accounts,
        &params.global_trader_index,
        &params.active_trader_buffer,
    );
    accounts.push(AccountMeta::readonly(SYSTEM_PROGRAM_ID));

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Parameters for placing a limit order with attached conditionals atomically.
#[derive(Debug, Clone)]
pub struct PlaceLimitOrderWithConditionalsParams {
    trader_wallet: Pubkey,
    trader_account: Pubkey,
    perp_asset_map: Pubkey,
    orderbook: Pubkey,
    spline_collection: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    payer: Pubkey,
    trader_conditional_orders: Pubkey,
    order_packet: OrderPacket,
    slot: u64,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
}

impl PlaceLimitOrderWithConditionalsParams {
    pub fn builder() -> PlaceLimitOrderWithConditionalsParamsBuilder {
        PlaceLimitOrderWithConditionalsParamsBuilder::new()
    }
}

#[derive(Default)]
pub struct PlaceLimitOrderWithConditionalsParamsBuilder {
    trader_wallet: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    payer: Option<Pubkey>,
    trader_conditional_orders: Option<Pubkey>,
    order_packet: Option<OrderPacket>,
    slot: Option<u64>,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
}

impl PlaceLimitOrderWithConditionalsParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn position_authority(self, position_authority: Pubkey) -> Self {
        self.trader_wallet(position_authority)
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn perp_asset_map(mut self, perp_asset_map: Pubkey) -> Self {
        self.perp_asset_map = Some(perp_asset_map);
        self
    }

    pub fn orderbook(mut self, orderbook: Pubkey) -> Self {
        self.orderbook = Some(orderbook);
        self
    }

    pub fn spline_collection(mut self, spline_collection: Pubkey) -> Self {
        self.spline_collection = Some(spline_collection);
        self
    }

    pub fn global_trader_index(mut self, global_trader_index: Vec<Pubkey>) -> Self {
        self.global_trader_index = Some(global_trader_index);
        self
    }

    pub fn active_trader_buffer(mut self, active_trader_buffer: Vec<Pubkey>) -> Self {
        self.active_trader_buffer = Some(active_trader_buffer);
        self
    }

    pub fn payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn trader_conditional_orders(mut self, trader_conditional_orders: Pubkey) -> Self {
        self.trader_conditional_orders = Some(trader_conditional_orders);
        self
    }

    pub fn order_packet(mut self, order_packet: OrderPacket) -> Self {
        self.order_packet = Some(order_packet);
        self
    }

    pub fn slot(mut self, slot: u64) -> Self {
        self.slot = Some(slot);
        self
    }

    pub fn greater_trigger_order(mut self, trigger: TriggerOrderParams) -> Self {
        self.greater_trigger_order = Some(trigger);
        self
    }

    pub fn less_trigger_order(mut self, trigger: TriggerOrderParams) -> Self {
        self.less_trigger_order = Some(trigger);
        self
    }

    pub fn build(self) -> Result<PlaceLimitOrderWithConditionalsParams, PhoenixIxError> {
        let trader_account = self
            .trader_account
            .ok_or(PhoenixIxError::MissingField("trader_account"))?;
        let trader_conditional_orders = self
            .trader_conditional_orders
            .unwrap_or_else(|| get_conditional_orders_address(&trader_account));
        Ok(PlaceLimitOrderWithConditionalsParams {
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            trader_account,
            perp_asset_map: self
                .perp_asset_map
                .ok_or(PhoenixIxError::MissingField("perp_asset_map"))?,
            orderbook: self
                .orderbook
                .ok_or(PhoenixIxError::MissingField("orderbook"))?,
            spline_collection: self
                .spline_collection
                .ok_or(PhoenixIxError::MissingField("spline_collection"))?,
            global_trader_index: self
                .global_trader_index
                .ok_or(PhoenixIxError::MissingField("global_trader_index"))?,
            active_trader_buffer: self
                .active_trader_buffer
                .ok_or(PhoenixIxError::MissingField("active_trader_buffer"))?,
            payer: self.payer.ok_or(PhoenixIxError::MissingField("payer"))?,
            trader_conditional_orders,
            order_packet: self
                .order_packet
                .ok_or(PhoenixIxError::MissingField("order_packet"))?,
            slot: self.slot.unwrap_or(0),
            greater_trigger_order: self.greater_trigger_order,
            less_trigger_order: self.less_trigger_order,
        })
    }
}

#[derive(BorshSerialize)]
struct PlaceLimitOrderWithConditionalsData {
    order_packet: OrderPacket,
    slot: u64,
    greater_trigger_order: Option<TriggerOrderParams>,
    less_trigger_order: Option<TriggerOrderParams>,
}

/// Create a place-limit-order-with-conditionals instruction.
pub fn create_place_limit_order_with_conditionals_ix(
    params: PlaceLimitOrderWithConditionalsParams,
) -> Result<Instruction, PhoenixIxError> {
    validate_triggers(
        params.greater_trigger_order.as_ref(),
        params.less_trigger_order.as_ref(),
    )?;
    validate_arenas(&params.global_trader_index, &params.active_trader_buffer)?;

    let mut data = Vec::new();
    data.extend_from_slice(&place_limit_order_with_conditionals_discriminant());
    data.extend_from_slice(
        &to_vec(&PlaceLimitOrderWithConditionalsData {
            order_packet: params.order_packet,
            slot: params.slot,
            greater_trigger_order: params.greater_trigger_order,
            less_trigger_order: params.less_trigger_order,
        })
        .expect("serialization should not fail"),
    );

    let mut accounts = vec![
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::readonly_signer(params.trader_wallet),
        AccountMeta::writable(params.trader_account),
        AccountMeta::writable(params.perp_asset_map),
    ];
    push_arenas(
        &mut accounts,
        &params.global_trader_index,
        &params.active_trader_buffer,
    );
    accounts.push(AccountMeta::writable(params.orderbook));
    accounts.push(AccountMeta::writable(params.spline_collection));
    accounts.push(AccountMeta::writable_signer(params.payer));
    accounts.push(AccountMeta::writable(params.trader_conditional_orders));
    accounts.push(AccountMeta::readonly(SYSTEM_PROGRAM_ID));

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Parameters for cancelling a conditional order.
#[derive(Debug, Clone)]
pub struct CancelConditionalOrderParams {
    trader_account: Pubkey,
    trader_wallet: Pubkey,
    orderbook: Pubkey,
    trader_conditional_orders: Pubkey,
    conditional_order_index: u8,
    disable_first: bool,
    disable_second: bool,
}

impl CancelConditionalOrderParams {
    pub fn builder() -> CancelConditionalOrderParamsBuilder {
        CancelConditionalOrderParamsBuilder::new()
    }
}

#[derive(Default)]
pub struct CancelConditionalOrderParamsBuilder {
    trader_account: Option<Pubkey>,
    trader_wallet: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    trader_conditional_orders: Option<Pubkey>,
    conditional_order_index: Option<u8>,
    disable_first: Option<bool>,
    disable_second: Option<bool>,
}

impl CancelConditionalOrderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn position_authority(self, position_authority: Pubkey) -> Self {
        self.trader_wallet(position_authority)
    }

    pub fn orderbook(mut self, orderbook: Pubkey) -> Self {
        self.orderbook = Some(orderbook);
        self
    }

    pub fn trader_conditional_orders(mut self, trader_conditional_orders: Pubkey) -> Self {
        self.trader_conditional_orders = Some(trader_conditional_orders);
        self
    }

    pub fn conditional_order_index(mut self, conditional_order_index: u8) -> Self {
        self.conditional_order_index = Some(conditional_order_index);
        self
    }

    pub fn disable_first(mut self, disable_first: bool) -> Self {
        self.disable_first = Some(disable_first);
        self
    }

    pub fn disable_second(mut self, disable_second: bool) -> Self {
        self.disable_second = Some(disable_second);
        self
    }

    pub fn build(self) -> Result<CancelConditionalOrderParams, PhoenixIxError> {
        let trader_account = self
            .trader_account
            .ok_or(PhoenixIxError::MissingField("trader_account"))?;
        let trader_conditional_orders = self
            .trader_conditional_orders
            .unwrap_or_else(|| get_conditional_orders_address(&trader_account));
        Ok(CancelConditionalOrderParams {
            trader_account,
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            orderbook: self
                .orderbook
                .ok_or(PhoenixIxError::MissingField("orderbook"))?,
            trader_conditional_orders,
            conditional_order_index: self
                .conditional_order_index
                .ok_or(PhoenixIxError::MissingField("conditional_order_index"))?,
            disable_first: self.disable_first.unwrap_or(false),
            disable_second: self.disable_second.unwrap_or(false),
        })
    }
}

#[derive(BorshSerialize)]
struct CancelConditionalOrderData {
    conditional_order_index: u8,
    disable_first: bool,
    disable_second: bool,
}

/// Create a cancel-conditional-order instruction.
pub fn create_cancel_conditional_order_ix(
    params: CancelConditionalOrderParams,
) -> Result<Instruction, PhoenixIxError> {
    validate_conditional_order_index(params.conditional_order_index)?;
    if !params.disable_first && !params.disable_second {
        return Err(PhoenixIxError::MissingConditionalOrderDisableFlag);
    }

    let mut data = Vec::new();
    data.extend_from_slice(&cancel_conditional_order_discriminant());
    data.extend_from_slice(
        &to_vec(&CancelConditionalOrderData {
            conditional_order_index: params.conditional_order_index,
            disable_first: params.disable_first,
            disable_second: params.disable_second,
        })
        .expect("serialization should not fail"),
    );

    let accounts = vec![
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::writable(params.trader_account),
        AccountMeta::readonly_signer(params.trader_wallet),
        AccountMeta::writable(params.orderbook),
        AccountMeta::writable(params.trader_conditional_orders),
    ];

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn validate_triggers(
    greater_trigger_order: Option<&TriggerOrderParams>,
    less_trigger_order: Option<&TriggerOrderParams>,
) -> Result<(), PhoenixIxError> {
    if greater_trigger_order.is_none() && less_trigger_order.is_none() {
        return Err(PhoenixIxError::MissingConditionalOrderTrigger);
    }
    if let Some(trigger) = greater_trigger_order {
        if trigger.trigger_direction != Direction::GreaterThan {
            return Err(PhoenixIxError::InvalidConditionalOrderTriggerDirection);
        }
    }
    if let Some(trigger) = less_trigger_order {
        if trigger.trigger_direction != Direction::LessThan {
            return Err(PhoenixIxError::InvalidConditionalOrderTriggerDirection);
        }
    }
    Ok(())
}

fn validate_sizing(
    size_base_lots: Option<u64>,
    size_percent: Option<u8>,
) -> Result<(), PhoenixIxError> {
    if size_base_lots.is_some() == size_percent.is_some() {
        return Err(PhoenixIxError::InvalidConditionalOrderSize);
    }
    if let Some(percent) = size_percent {
        if percent == 0 || percent > 100 {
            return Err(PhoenixIxError::InvalidConditionalOrderPercent);
        }
    }
    Ok(())
}

fn validate_arenas(
    global_trader_index: &[Pubkey],
    active_trader_buffer: &[Pubkey],
) -> Result<(), PhoenixIxError> {
    if global_trader_index.is_empty() {
        return Err(PhoenixIxError::EmptyGlobalTraderIndex);
    }
    if active_trader_buffer.is_empty() {
        return Err(PhoenixIxError::EmptyActiveTraderBuffer);
    }
    Ok(())
}

fn validate_conditional_order_index(index: u8) -> Result<(), PhoenixIxError> {
    if index == 0 || index > MAX_CONDITIONAL_ORDER_INDEX {
        return Err(PhoenixIxError::InvalidConditionalOrderIndex);
    }
    Ok(())
}

fn push_arenas(
    accounts: &mut Vec<AccountMeta>,
    global_trader_index: &[Pubkey],
    active_trader_buffer: &[Pubkey],
) {
    for addr in global_trader_index {
        accounts.push(AccountMeta::writable(*addr));
    }
    for addr in active_trader_buffer {
        accounts.push(AccountMeta::writable(*addr));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger(direction: Direction) -> TriggerOrderParams {
        TriggerOrderParams::new(direction, Side::Ask, StopLossOrderKind::IOC, 50000, 49999)
    }

    #[test]
    fn test_create_conditional_orders_account_ix() {
        let trader_account = Pubkey::new_unique();
        let params = CreateConditionalOrdersAccountParams::builder()
            .payer(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .trader_account(trader_account)
            .capacity(32)
            .build()
            .unwrap();

        let ix = create_create_conditional_orders_account_ix(params).unwrap();
        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 8);
        assert_eq!(
            &ix.data[..8],
            &create_conditional_orders_account_discriminant()
        );
        assert_eq!(ix.data[8], 32);
        assert_eq!(
            ix.accounts[6].pubkey,
            get_conditional_orders_address(&trader_account)
        );
    }

    #[test]
    fn test_place_position_conditional_order_ix() {
        let params = PlacePositionConditionalOrderParams::builder()
            .payer(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .trader_wallet(Pubkey::new_unique())
            .asset_id(1)
            .greater_trigger_order(trigger(Direction::GreaterThan))
            .size_base_lots(10)
            .build()
            .unwrap();

        let ix = create_place_position_conditional_order_ix(params).unwrap();
        assert_eq!(ix.accounts.len(), 13);
        assert_eq!(
            &ix.data[..8],
            &place_position_conditional_order_discriminant()
        );
    }

    #[test]
    fn test_cancel_conditional_order_validates_index() {
        let params = CancelConditionalOrderParams::builder()
            .trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .conditional_order_index(0)
            .disable_first(true)
            .build()
            .unwrap();

        let result = create_cancel_conditional_order_ix(params);
        assert!(matches!(
            result,
            Err(PhoenixIxError::InvalidConditionalOrderIndex)
        ));
    }

    #[test]
    fn test_invalid_trigger_direction_fails() {
        let params = PlaceAttachedConditionalOrderParams::builder()
            .trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .payer(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .order_id(FifoOrderId {
                price_in_ticks: 1,
                order_sequence_number: 2,
            })
            .asset_id(1)
            .greater_trigger_order(trigger(Direction::LessThan))
            .build()
            .unwrap();

        let result = create_place_attached_conditional_order_ix(params);
        assert!(matches!(
            result,
            Err(PhoenixIxError::InvalidConditionalOrderTriggerDirection)
        ));
    }
}
