//! Cancel orders by ID instruction construction.

use borsh::to_vec;
use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID,
    cancel_orders_by_id_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, CancelId, Instruction};

/// Maximum number of orders that can be cancelled in a single instruction.
pub const MAX_CANCEL_ORDER_IDS: usize = 100;

/// Parameters for cancelling orders by ID.
#[derive(Debug, Clone)]
pub struct CancelOrdersByIdParams {
    trader: Pubkey,
    trader_account: Pubkey,
    perp_asset_map: Pubkey,
    orderbook: Pubkey,
    spline_collection: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    order_ids: Vec<CancelId>,
}

impl CancelOrdersByIdParams {
    /// Start building with the builder pattern.
    pub fn builder() -> CancelOrdersByIdParamsBuilder {
        CancelOrdersByIdParamsBuilder::new()
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

    pub fn order_ids(&self) -> &[CancelId] {
        &self.order_ids
    }
}

/// Builder for `CancelOrdersByIdParams`.
#[derive(Default)]
pub struct CancelOrdersByIdParamsBuilder {
    trader: Option<Pubkey>,
    trader_account: Option<Pubkey>,
    perp_asset_map: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    order_ids: Option<Vec<CancelId>>,
}

impl CancelOrdersByIdParamsBuilder {
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

    pub fn order_ids(mut self, order_ids: Vec<CancelId>) -> Self {
        self.order_ids = Some(order_ids);
        self
    }

    pub fn build(self) -> Result<CancelOrdersByIdParams, PhoenixIxError> {
        Ok(CancelOrdersByIdParams {
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
            order_ids: self
                .order_ids
                .ok_or(PhoenixIxError::MissingField("order_ids"))?,
        })
    }
}

/// Create a cancel orders by ID instruction.
///
/// # Arguments
///
/// * `params` - The cancel order parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
///
/// # Errors
///
/// Returns an error if:
/// - No order IDs are provided
/// - Too many order IDs (> 100)
/// - Required account arrays are empty
pub fn create_cancel_orders_by_id_ix(
    params: CancelOrdersByIdParams,
) -> Result<Instruction, PhoenixIxError> {
    validate(&params)?;

    let data = encode_cancel_orders(&params);
    let accounts = build_accounts(&params);

    Ok(Instruction {
        program_id: PHOENIX_PROGRAM_ID,
        accounts,
        data,
    })
}

fn validate(params: &CancelOrdersByIdParams) -> Result<(), PhoenixIxError> {
    if params.global_trader_index().is_empty() {
        return Err(PhoenixIxError::EmptyGlobalTraderIndex);
    }
    if params.active_trader_buffer().is_empty() {
        return Err(PhoenixIxError::EmptyActiveTraderBuffer);
    }
    if params.order_ids().is_empty() {
        return Err(PhoenixIxError::NoOrderIds);
    }
    if params.order_ids().len() > MAX_CANCEL_ORDER_IDS {
        return Err(PhoenixIxError::TooManyOrderIds);
    }
    Ok(())
}

fn encode_cancel_orders(params: &CancelOrdersByIdParams) -> Vec<u8> {
    let mut data = Vec::new();

    // Instruction discriminant (8 bytes)
    data.extend_from_slice(&cancel_orders_by_id_discriminant());

    // Array length as u32 (Borsh array encoding)
    let len = params.order_ids().len() as u32;
    data.extend_from_slice(&len.to_le_bytes());

    // Each CancelId
    for cancel_id in params.order_ids() {
        data.extend_from_slice(&to_vec(cancel_id).expect("serialization should not fail"));
    }

    data
}

fn build_accounts(params: &CancelOrdersByIdParams) -> Vec<AccountMeta> {
    let mut accounts = Vec::new();

    // LogAccountGroupAccounts (2 accounts)
    accounts.push(AccountMeta::readonly(PHOENIX_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(PHOENIX_LOG_AUTHORITY));

    // CancelOrdersByIdInstructionGroupAccounts
    accounts.push(AccountMeta::writable(PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::readonly_signer(params.trader()));
    accounts.push(AccountMeta::writable(params.trader_account()));
    accounts.push(AccountMeta::writable(params.perp_asset_map()));

    // Global trader index addresses
    for addr in params.global_trader_index() {
        accounts.push(AccountMeta::writable(*addr));
    }

    // Active trader buffer addresses
    for addr in params.active_trader_buffer() {
        accounts.push(AccountMeta::writable(*addr));
    }

    accounts.push(AccountMeta::writable(params.orderbook()));
    accounts.push(AccountMeta::writable(params.spline_collection()));

    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_cancel_orders_ix() {
        let params = CancelOrdersByIdParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .order_ids(vec![CancelId::new(50000, 12345)])
            .build()
            .unwrap();

        let ix = create_cancel_orders_by_id_ix(params).unwrap();

        assert_eq!(ix.program_id, PHOENIX_PROGRAM_ID);
        // 2 log accounts + 4 base accounts + 1 global trader index + 1 active trader
        // buffer + 2 market accounts = 10
        assert_eq!(ix.accounts.len(), 10);
        // Data should start with discriminant
        assert_eq!(&ix.data[..8], &cancel_orders_by_id_discriminant());
    }

    #[test]
    fn test_cancel_multiple_orders() {
        let params = CancelOrdersByIdParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .order_ids(vec![
                CancelId::new(50000, 1),
                CancelId::new(50100, 2),
                CancelId::new(49900, 3),
            ])
            .build()
            .unwrap();

        let ix = create_cancel_orders_by_id_ix(params).unwrap();

        // Verify data contains array length
        let array_len = u32::from_le_bytes([ix.data[8], ix.data[9], ix.data[10], ix.data[11]]);
        assert_eq!(array_len, 3);
    }

    #[test]
    fn test_empty_order_ids_fails() {
        let params = CancelOrdersByIdParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .order_ids(vec![])
            .build()
            .unwrap();

        let result = create_cancel_orders_by_id_ix(params);
        assert!(matches!(result, Err(PhoenixIxError::NoOrderIds)));
    }

    #[test]
    fn test_too_many_order_ids_fails() {
        let params = CancelOrdersByIdParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .order_ids((0..101).map(|i| CancelId::new(50000, i)).collect())
            .build()
            .unwrap();

        let result = create_cancel_orders_by_id_ix(params);
        assert!(matches!(result, Err(PhoenixIxError::TooManyOrderIds)));
    }

    #[test]
    fn test_builder_missing_required_field() {
        let result = CancelOrdersByIdParams::builder()
            .trader(Pubkey::new_unique())
            // Missing other required fields
            .build();

        assert!(matches!(result, Err(PhoenixIxError::MissingField(_))));
    }
}
