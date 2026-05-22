//! Helpers for fetching and deserializing Phoenix on-chain accounts.

use std::fmt;

use solana_pubkey::Pubkey;
use thiserror::Error;

pub use crate::phoenix_rise_types::accounts::*;

#[derive(Debug, Error)]
pub enum PhoenixAccountClientError {
    #[error("account data fetch failed: {0}")]
    Fetch(String),

    #[error(transparent)]
    Decode(#[from] AccountDeserializeError),
}

pub trait AccountDataFetcher {
    type Error: fmt::Display;

    fn fetch_account_data(&self, address: &Pubkey) -> Result<Vec<u8>, Self::Error>;
}

#[derive(Clone)]
pub struct PhoenixAccountClient<F> {
    fetcher: F,
}

impl<F> PhoenixAccountClient<F> {
    pub fn new(fetcher: F) -> Self {
        Self { fetcher }
    }

    pub fn fetcher(&self) -> &F {
        &self.fetcher
    }
}

impl<F> PhoenixAccountClient<F>
where
    F: AccountDataFetcher,
{
    pub fn fetch_account_data(
        &self,
        address: &Pubkey,
    ) -> Result<Vec<u8>, PhoenixAccountClientError> {
        self.fetcher
            .fetch_account_data(address)
            .map_err(|error| PhoenixAccountClientError::Fetch(error.to_string()))
    }

    pub fn fetch_decoded<T: AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> Result<T, PhoenixAccountClientError> {
        let data = self.fetch_account_data(address)?;
        Ok(T::try_from_account_bytes(&data)?)
    }

    pub fn fetch_global_configuration(
        &self,
        address: Option<Pubkey>,
    ) -> Result<GlobalConfiguration, PhoenixAccountClientError> {
        let address = address.unwrap_or(*crate::phoenix_rise_ix::PHOENIX_GLOBAL_CONFIGURATION);
        self.fetch_decoded(&address)
    }

    pub fn fetch_conditional_order_collection(
        &self,
        address: &Pubkey,
    ) -> Result<ConditionalOrderCollection, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_orderbook_header(
        &self,
        address: &Pubkey,
    ) -> Result<OrderbookHeader, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_orderbook(
        &self,
        address: &Pubkey,
    ) -> Result<Orderbook, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_perp_asset_map(
        &self,
        address: &Pubkey,
    ) -> Result<PerpAssetMap, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_permission(
        &self,
        address: &Pubkey,
    ) -> Result<Permission, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_spline_collection(
        &self,
        address: &Pubkey,
    ) -> Result<SplineCollection, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_stop_losses(
        &self,
        address: &Pubkey,
    ) -> Result<StopLosses, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_trader(&self, address: &Pubkey) -> Result<Trader, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_mint(&self, address: &Pubkey) -> Result<Mint, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_token_account(
        &self,
        address: &Pubkey,
    ) -> Result<TokenAccount, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }

    pub fn fetch_withdraw_queue_header(
        &self,
        address: &Pubkey,
    ) -> Result<WithdrawQueueHeader, PhoenixAccountClientError> {
        self.fetch_decoded(address)
    }
}
