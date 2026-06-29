//! Place multi-limit-order instruction construction.

use borsh::to_vec;
use solana_pubkey::Pubkey;

use crate::constants::{PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID};
use crate::error::PhoenixIxError;
use crate::order_packet::{CondensedOrder, MultipleOrderPacket, client_order_id_to_bytes};
use crate::types::{AccountMeta, Instruction, push_trader_index_accounts};

/// Parameters for placing multiple limit orders in a single instruction.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MultiLimitOrderParams {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    trader_account: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    perp_asset_map: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    orderbook: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    spline_collection: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    global_trader_index: Vec<Pubkey>,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey_vec"))]
    active_trader_buffer: Vec<Pubkey>,
    bids: Vec<CondensedOrder>,
    asks: Vec<CondensedOrder>,
    client_order_id: Option<u128>,
    slide: bool,
    /// Market symbol (e.g. "SOL"). Not serialized into the instruction.
    symbol: String,
}

impl MultiLimitOrderParams {
    pub fn builder() -> MultiLimitOrderParamsBuilder {
        MultiLimitOrderParamsBuilder::new()
    }

    pub fn trader(&self) -> Pubkey {
        self.trader
    }

    pub fn trader_account(&self) -> Pubkey {
        self.trader_account
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

    pub fn bids(&self) -> &[CondensedOrder] {
        &self.bids
    }

    pub fn asks(&self) -> &[CondensedOrder] {
        &self.asks
    }

    pub fn client_order_id(&self) -> Option<u128> {
        self.client_order_id
    }

    pub fn slide(&self) -> bool {
        self.slide
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

/// Builder for `MultiLimitOrderParams`.
#[derive(Default)]
pub struct MultiLimitOrderParamsBuilder {
    trader: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    bids: Vec<CondensedOrder>,
    asks: Vec<CondensedOrder>,
    client_order_id: Option<u128>,
    slide: bool,
    symbol: Option<String>,
}

impl MultiLimitOrderParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trader(mut self, trader: Pubkey) -> Self {
        self.trader = Some(trader);
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

    pub fn bids(mut self, bids: Vec<CondensedOrder>) -> Self {
        self.bids = bids;
        self
    }

    pub fn asks(mut self, asks: Vec<CondensedOrder>) -> Self {
        self.asks = asks;
        self
    }

    pub fn add_bid(mut self, order: CondensedOrder) -> Self {
        self.bids.push(order);
        self
    }

    pub fn add_ask(mut self, order: CondensedOrder) -> Self {
        self.asks.push(order);
        self
    }

    pub fn client_order_id(mut self, client_order_id: u128) -> Self {
        self.client_order_id = Some(client_order_id);
        self
    }

    pub fn slide(mut self, slide: bool) -> Self {
        self.slide = slide;
        self
    }

    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn build(self) -> Result<MultiLimitOrderParams, PhoenixIxError> {
        Ok(MultiLimitOrderParams {
            trader: self.trader.ok_or(PhoenixIxError::MissingField("trader"))?,
            trader_account: self
                .trader_account
                .ok_or(PhoenixIxError::MissingField("trader_account"))?,
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
            bids: self.bids,
            asks: self.asks,
            client_order_id: self.client_order_id,
            slide: self.slide,
            symbol: self.symbol.unwrap_or_default(),
        })
    }
}

/// Create a place multi-limit-order instruction.
///
/// This instruction places multiple post-only limit orders (bids and asks) in a
/// single transaction. Individual orders that fail (e.g. too many orders,
/// post-only cross) are skipped without failing the entire transaction.
pub fn create_place_multi_limit_order_ix(
    params: MultiLimitOrderParams,
) -> Result<Instruction, PhoenixIxError> {
    validate(&params)?;

    let data = encode_multi_limit_order(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn validate(params: &MultiLimitOrderParams) -> Result<(), PhoenixIxError> {
    if params.global_trader_index().is_empty() {
        return Err(PhoenixIxError::EmptyGlobalTraderIndex);
    }
    if params.active_trader_buffer().is_empty() {
        return Err(PhoenixIxError::EmptyActiveTraderBuffer);
    }
    Ok(())
}

fn encode_multi_limit_order(params: &MultiLimitOrderParams) -> Vec<u8> {
    let mut data = Vec::new();

    // Instruction discriminant (8 bytes)
    data.extend_from_slice(&crate::PhoenixInstruction::PlaceMultiLimitOrder.discriminant());

    let client_order_id = params.client_order_id().map(client_order_id_to_bytes);

    let packet = MultipleOrderPacket {
        bids: params.bids().to_vec(),
        asks: params.asks().to_vec(),
        client_order_id,
        slide: params.slide(),
    };

    data.extend_from_slice(&to_vec(&packet).expect("serialization should not fail"));

    data
}

fn build_accounts(params: &MultiLimitOrderParams) -> Vec<AccountMeta> {
    let mut accounts = Vec::new();

    // LogAccountGroupAccounts (2 accounts)
    accounts.push(AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY));

    // MarketActionInstructionGroupAccounts
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    accounts.push(AccountMeta::writable(params.trader_account()));
    accounts.push(AccountMeta::writable(params.perp_asset_map()));

    push_trader_index_accounts(
        &mut accounts,
        params.global_trader_index(),
        params.active_trader_buffer(),
    );

    accounts.push(AccountMeta::writable(params.orderbook()));
    accounts.push(AccountMeta::writable(params.spline_collection()));

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_multi_limit_order_ix() {
        let params = MultiLimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .add_bid(CondensedOrder {
                price_in_ticks: 50000,
                size_in_base_lots: 1000,
                last_valid_slot: None,
            })
            .add_ask(CondensedOrder {
                price_in_ticks: 51000,
                size_in_base_lots: 1000,
                last_valid_slot: None,
            })
            .slide(true)
            .build()
            .unwrap();

        let ix = create_place_multi_limit_order_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        // 2 log + 4 base + 1 global trader index + 1 active trader buffer + 2 market =
        // 10
        assert_eq!(ix.accounts.len(), 10);
        assert_eq!(
            &ix.data[..8],
            &crate::PhoenixInstruction::PlaceMultiLimitOrder.discriminant()
        );
    }

    #[test]
    fn test_empty_orders_allowed() {
        let params = MultiLimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build()
            .unwrap();

        let ix = create_place_multi_limit_order_ix(params);
        assert!(ix.is_ok());
    }

    #[test]
    fn test_empty_global_trader_index_fails() {
        let params = MultiLimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build()
            .unwrap();

        let result = create_place_multi_limit_order_ix(params);
        assert!(matches!(
            result,
            Err(PhoenixIxError::EmptyGlobalTraderIndex)
        ));
    }

    #[test]
    fn test_builder_missing_required_field() {
        let result = MultiLimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .build();

        assert!(matches!(result, Err(PhoenixIxError::MissingField(_))));
    }
}
