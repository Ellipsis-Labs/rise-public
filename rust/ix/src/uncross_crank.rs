//! Uncross crank instruction construction.

use borsh::{BorshSerialize, to_vec};
use solana_pubkey::Pubkey;

use crate::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID,
    uncross_crank_discriminant,
};
use crate::ix::error::PhoenixIxError;
use crate::ix::types::{AccountMeta, Instruction};

const DEFAULT_MATCH_LIMIT: u64 = 100;

/// Parameters for cranking the orderbook to uncross matched orders.
#[derive(Debug, Clone)]
pub struct UncrossCrankParams {
    perp_asset_map: Pubkey,
    orderbook: Pubkey,
    spline_collection: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    match_limit: u64,
}

impl UncrossCrankParams {
    /// Start building with the builder pattern.
    pub fn builder() -> UncrossCrankParamsBuilder {
        UncrossCrankParamsBuilder::new()
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

    pub fn match_limit(&self) -> u64 {
        self.match_limit
    }
}

/// Builder for `UncrossCrankParams`.
#[derive(Default)]
pub struct UncrossCrankParamsBuilder {
    perp_asset_map: Option<Pubkey>,
    orderbook: Option<Pubkey>,
    spline_collection: Option<Pubkey>,
    global_trader_index: Option<Vec<Pubkey>>,
    active_trader_buffer: Option<Vec<Pubkey>>,
    match_limit: Option<u64>,
}

impl UncrossCrankParamsBuilder {
    pub fn new() -> Self {
        Self::default()
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

    pub fn match_limit(mut self, match_limit: u64) -> Self {
        self.match_limit = Some(match_limit);
        self
    }

    pub fn build(self) -> Result<UncrossCrankParams, PhoenixIxError> {
        Ok(UncrossCrankParams {
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
            match_limit: self.match_limit.unwrap_or(DEFAULT_MATCH_LIMIT),
        })
    }
}

/// Create an uncross crank instruction.
///
/// # Arguments
///
/// * `params` - The uncross crank parameters
///
/// # Returns
///
/// A Solana instruction ready to be included in a transaction.
pub fn create_uncross_crank_ix(params: UncrossCrankParams) -> Result<Instruction, PhoenixIxError> {
    validate(&params)?;

    Ok(Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: build_accounts(&params),
        data: encode_uncross_crank(&params),
    })
}

fn validate(params: &UncrossCrankParams) -> Result<(), PhoenixIxError> {
    if params.global_trader_index().is_empty() {
        return Err(PhoenixIxError::EmptyGlobalTraderIndex);
    }
    if params.active_trader_buffer().is_empty() {
        return Err(PhoenixIxError::EmptyActiveTraderBuffer);
    }
    Ok(())
}

#[derive(BorshSerialize)]
struct UncrossCrankData {
    match_limit: u64,
}

fn encode_uncross_crank(params: &UncrossCrankParams) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&uncross_crank_discriminant());
    data.extend_from_slice(
        &to_vec(&UncrossCrankData {
            match_limit: params.match_limit(),
        })
        .expect("serialization should not fail"),
    );
    data
}

fn build_accounts(params: &UncrossCrankParams) -> Vec<AccountMeta> {
    let mut accounts = Vec::new();

    // LogAccountGroupAccounts
    accounts.push(AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(*PHOENIX_LOG_AUTHORITY));

    // MaintenanceInstructionGroupAccounts
    accounts.push(AccountMeta::writable(*PHOENIX_GLOBAL_CONFIGURATION));
    accounts.push(AccountMeta::writable(params.perp_asset_map()));

    for addr in params.global_trader_index() {
        accounts.push(AccountMeta::writable(*addr));
    }

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
    fn test_create_uncross_crank_ix() {
        let params = UncrossCrankParams::builder()
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .match_limit(5)
            .build()
            .unwrap();

        let ix = create_uncross_crank_ix(params).unwrap();

        assert_eq!(ix.program_id, *PHOENIX_PROGRAM_ID);
        assert_eq!(ix.accounts.len(), 8);
        assert_eq!(&ix.data[..8], &uncross_crank_discriminant());
        assert_eq!(u64::from_le_bytes(ix.data[8..16].try_into().unwrap()), 5);
    }

    #[test]
    fn test_uncross_crank_defaults_match_limit() {
        let params = UncrossCrankParams::builder()
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .build()
            .unwrap();

        assert_eq!(params.match_limit(), DEFAULT_MATCH_LIMIT);
    }
}
