//! Place legacy stop-loss/take-profit instruction construction.
//!
//! This builds the `PlaceStopLoss` instruction used by trader-state position
//! trigger updates. New conditional-order instructions live in
//! `conditional_order.rs`.

use borsh::{BorshSerialize, to_vec};
use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    get_stop_loss_address, place_stop_loss_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Direction, Instruction, Side, StopLossOrderKind};

/// Parameters for placing a legacy stop-loss or take-profit trigger.
#[derive(Debug, Clone)]
pub struct PlaceStopLossParams {
    funder: Pubkey,
    trader_account: Pubkey,
    position_authority: Pubkey,
    perp_asset_map: Pubkey,
    orderbook: Pubkey,
    spline_collection: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    stop_loss_account: Pubkey,
    asset_id: u64,
    trigger_price: u64,
    execution_price: u64,
    trade_side: Side,
    execution_direction: Direction,
    order_kind: StopLossOrderKind,
}

impl PlaceStopLossParams {
    pub fn builder() -> PlaceStopLossParamsBuilder {
        PlaceStopLossParamsBuilder::new()
    }

    pub fn funder(&self) -> Pubkey {
        self.funder
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
    }

    pub fn position_authority(&self) -> Pubkey {
        self.position_authority
    }

    pub fn perp_asset_map(&self) -> Pubkey {
        self.perp_asset_map
    }

    pub fn orderbook(&self) -> Pubkey {
        self.orderbook
    }

    pub fn spline_collection(&self) -> Pubkey {
        self.spline_collection
    }

    pub fn global_trader_index(&self) -> &[Pubkey] {
        &self.global_trader_index
    }

    pub fn active_trader_buffer(&self) -> &[Pubkey] {
        &self.active_trader_buffer
    }

    pub fn stop_loss_account(&self) -> Pubkey {
        self.stop_loss_account
    }

    pub fn asset_id(&self) -> u64 {
        self.asset_id
    }

    pub fn trigger_price(&self) -> u64 {
        self.trigger_price
    }

    pub fn execution_price(&self) -> u64 {
        self.execution_price
    }

    pub fn trade_side(&self) -> Side {
        self.trade_side
    }

    pub fn execution_direction(&self) -> Direction {
        self.execution_direction
    }

    pub fn order_kind(&self) -> StopLossOrderKind {
        self.order_kind
    }
}

/// Builder for [`PlaceStopLossParams`].
#[derive(Default)]
pub struct PlaceStopLossParamsBuilder {
    funder: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    position_authority: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    stop_loss_account: Option<Pubkey>,
    asset_id: Option<u64>,
    trigger_price: Option<u64>,
    execution_price: Option<u64>,
    trade_side: Option<Side>,
    execution_direction: Option<Direction>,
    order_kind: Option<StopLossOrderKind>,
}

impl PlaceStopLossParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn funder(mut self, funder: Pubkey) -> Self {
        self.funder = Some(funder);
        self
    }

    pub fn trader_account(mut self, trader_account: Pubkey) -> Self {
        self.trader_account = Some(trader_account);
        self
    }

    pub fn position_authority(mut self, position_authority: Pubkey) -> Self {
        self.position_authority = Some(position_authority);
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

    pub fn stop_loss_account(mut self, stop_loss_account: Pubkey) -> Self {
        self.stop_loss_account = Some(stop_loss_account);
        self
    }

    pub fn asset_id(mut self, asset_id: u64) -> Self {
        self.asset_id = Some(asset_id);
        self
    }

    pub fn trigger_price(mut self, trigger_price: u64) -> Self {
        self.trigger_price = Some(trigger_price);
        self
    }

    pub fn execution_price(mut self, execution_price: u64) -> Self {
        self.execution_price = Some(execution_price);
        self
    }

    pub fn trade_side(mut self, trade_side: Side) -> Self {
        self.trade_side = Some(trade_side);
        self
    }

    pub fn execution_direction(mut self, execution_direction: Direction) -> Self {
        self.execution_direction = Some(execution_direction);
        self
    }

    pub fn order_kind(mut self, order_kind: StopLossOrderKind) -> Self {
        self.order_kind = Some(order_kind);
        self
    }

    pub fn build(self) -> Result<PlaceStopLossParams, PhoenixIxError> {
        let trader_account = self
            .trader_account
            .ok_or(PhoenixIxError::MissingField("trader_account"))?;
        let asset_id = self
            .asset_id
            .ok_or(PhoenixIxError::MissingField("asset_id"))?;

        Ok(PlaceStopLossParams {
            funder: self.funder.ok_or(PhoenixIxError::MissingField("funder"))?,
            trader_account,
            position_authority: self
                .position_authority
                .ok_or(PhoenixIxError::MissingField("position_authority"))?,
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
            stop_loss_account: self
                .stop_loss_account
                .unwrap_or_else(|| get_stop_loss_address(&trader_account, asset_id)),
            asset_id,
            trigger_price: self
                .trigger_price
                .ok_or(PhoenixIxError::MissingField("trigger_price"))?,
            execution_price: self.execution_price.unwrap_or(0),
            trade_side: self
                .trade_side
                .ok_or(PhoenixIxError::MissingField("trade_side"))?,
            execution_direction: self
                .execution_direction
                .ok_or(PhoenixIxError::MissingField("execution_direction"))?,
            order_kind: self.order_kind.unwrap_or(StopLossOrderKind::IOC),
        })
    }
}

#[derive(BorshSerialize)]
struct PlaceStopLossData {
    trigger_price: u64,
    execution_price: u64,
    trade_size: u64,
    trade_side: Side,
    trigger_direction: Direction,
    order_kind: StopLossOrderKind,
}

/// Create a legacy place-stop-loss instruction.
pub fn create_place_stop_loss_ix(
    params: PlaceStopLossParams,
) -> Result<Instruction, PhoenixIxError> {
    validate(&params)?;

    let data = encode_place_stop_loss(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn validate(params: &PlaceStopLossParams) -> Result<(), PhoenixIxError> {
    if params.global_trader_index().is_empty() {
        return Err(PhoenixIxError::EmptyGlobalTraderIndex);
    }
    if params.active_trader_buffer().is_empty() {
        return Err(PhoenixIxError::EmptyActiveTraderBuffer);
    }
    Ok(())
}

fn encode_place_stop_loss(params: &PlaceStopLossParams) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&place_stop_loss_discriminant());
    data.extend_from_slice(
        &to_vec(&PlaceStopLossData {
            trigger_price: params.trigger_price(),
            execution_price: params.execution_price(),
            trade_size: 0,
            trade_side: params.trade_side(),
            trigger_direction: params.execution_direction(),
            order_kind: params.order_kind(),
        })
        .expect("serialization should not fail"),
    );
    data
}

fn build_accounts(params: &PlaceStopLossParams) -> Vec<AccountMeta> {
    let mut accounts = vec![
        AccountMeta::readonly(*PHOENIX_PROGRAM_ID),
        AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY),
        AccountMeta::readonly(*PHOENIX_GLOBAL_CONFIGURATION),
        AccountMeta::writable_signer(params.funder()),
        AccountMeta::writable(params.trader_account()),
        AccountMeta::writable(params.perp_asset_map()),
    ];
    push_arenas(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );
    accounts.push(AccountMeta::writable(params.orderbook()));
    accounts.push(AccountMeta::writable(params.spline_collection()));
    accounts.push(AccountMeta::readonly_signer(params.position_authority()));
    accounts.push(AccountMeta::writable(params.stop_loss_account()));
    accounts.push(AccountMeta::readonly(SYSTEM_PROGRAM_ID));
    accounts
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

    fn test_params() -> PlaceStopLossParams {
        let trader_account = Pubkey::new_unique();
        PlaceStopLossParams::builder()
            .funder(Pubkey::new_unique())
            .trader_account(trader_account)
            .position_authority(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .asset_id(1)
            .trigger_price(50_000)
            .execution_price(49_500)
            .trade_side(Side::Ask)
            .execution_direction(Direction::LessThan)
            .order_kind(StopLossOrderKind::IOC)
            .build()
            .unwrap()
    }

    #[test]
    fn test_create_place_stop_loss_ix() {
        let params = test_params();
        let ix = create_place_stop_loss_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 13);
        assert_eq!(&ix.data[..8], &place_stop_loss_discriminant());
        assert_eq!(ix.data.len(), 35);
    }

    #[test]
    fn test_place_stop_loss_account_positions() {
        let params = test_params();
        let ix = create_place_stop_loss_ix(params.clone()).unwrap();

        assert_eq!(ix.accounts[0].pubkey, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_LOG_AUTHORITY);
        assert_eq!(ix.accounts[2].pubkey, *PHOENIX_GLOBAL_CONFIGURATION);
        assert_eq!(ix.accounts[3].pubkey, params.funder());
        assert!(ix.accounts[3].is_signer);
        assert!(ix.accounts[3].is_writable);
        assert_eq!(ix.accounts[4].pubkey, params.trader_account());
        assert_eq!(ix.accounts[5].pubkey, params.perp_asset_map());
        assert_eq!(ix.accounts[8].pubkey, params.orderbook());
        assert_eq!(ix.accounts[9].pubkey, params.spline_collection());
        assert_eq!(ix.accounts[10].pubkey, params.position_authority());
        assert_eq!(ix.accounts[11].pubkey, params.stop_loss_account());
        assert_eq!(ix.accounts[12].pubkey, SYSTEM_PROGRAM_ID);
    }

    #[test]
    fn test_place_stop_loss_validates_arenas() {
        let result = PlaceStopLossParams::builder()
            .funder(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .position_authority(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(Vec::new())
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .asset_id(1)
            .trigger_price(50_000)
            .execution_price(49_500)
            .trade_side(Side::Ask)
            .execution_direction(Direction::LessThan)
            .build()
            .and_then(create_place_stop_loss_ix);

        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }
}
