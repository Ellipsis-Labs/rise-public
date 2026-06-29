//! Helpers for fetching and deserializing Phoenix on-chain accounts.

use std::fmt;

use phoenix_rise_accounts::owned::{
    AccountDeserialize, AccountDeserializeError, ConditionalOrderCollection, GlobalConfiguration,
    Mint, Orderbook, OrderbookHeader, Permission, PerpAssetMapOwned, SplineCollection, StopLosses,
    TokenAccount, Trader, WithdrawQueueHeader,
};
use phoenix_rise_ix::constants::{PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_PROGRAM_ID};
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PhoenixAccountClientError {
    #[error("account data fetch failed: {0}")]
    Fetch(String),

    #[error(transparent)]
    Decode(#[from] AccountDeserializeError),

    #[error("account owner was not provided by the account fetcher")]
    OwnerUnavailable,

    #[error("account owner mismatch: expected {expected}, got {actual}")]
    OwnerMismatch { expected: Pubkey, actual: Pubkey },

    #[error("account is executable and cannot be decoded as data")]
    ExecutableAccount,
}

/// Account data plus authenticity metadata returned by owner-aware fetchers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedAccount {
    pub data: Vec<u8>,
    pub owner: Option<Pubkey>,
    pub executable: bool,
}

impl FetchedAccount {
    /// Construct a fetched account when the transport has owner metadata.
    pub fn new(data: Vec<u8>, owner: Pubkey, executable: bool) -> Self {
        Self {
            data,
            owner: Some(owner),
            executable,
        }
    }

    /// Construct raw fetched bytes for fixtures or already-verified data.
    pub fn unchecked(data: Vec<u8>) -> Self {
        Self {
            data,
            owner: None,
            executable: false,
        }
    }
}

/// Minimal account-data fetching interface used by [`PhoenixAccountClient`].
///
/// Implement this trait for your RPC client, cache, fixture loader, or indexer
/// adapter when you want the SDK to handle account deserialization but keep
/// ownership of the transport layer.
pub trait AccountDataFetcher {
    type Error: fmt::Display;

    fn fetch_account_data(&self, address: &Pubkey) -> Result<Vec<u8>, Self::Error>;

    /// Fetch account data together with owner/executable metadata.
    ///
    /// Override this for RPC/indexer-backed fetchers. The default preserves
    /// compatibility with byte-only fixtures, but owner-verified helpers will
    /// reject it with [`PhoenixAccountClientError::OwnerUnavailable`].
    fn fetch_account(&self, address: &Pubkey) -> Result<FetchedAccount, Self::Error> {
        self.fetch_account_data(address)
            .map(FetchedAccount::unchecked)
    }
}

/// Typed account reader over an arbitrary [`AccountDataFetcher`].
///
/// This is useful for off-chain tooling that wants owned, serde-friendly
/// decoded accounts. If you already have account bytes and need the
/// dependency-light borrowed view, use `phoenix-rise-accounts` directly.
#[derive(Clone)]
pub struct PhoenixAccountClient<F> {
    fetcher: F,
}

impl<F> PhoenixAccountClient<F> {
    /// Create a typed account client from an RPC/cache/indexer fetcher.
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
    /// Fetch raw account data through the configured fetcher.
    pub fn fetch_account_data(
        &self,
        address: &Pubkey,
    ) -> Result<Vec<u8>, PhoenixAccountClientError> {
        self.fetcher
            .fetch_account_data(address)
            .map_err(|error| PhoenixAccountClientError::Fetch(error.to_string()))
    }

    /// Fetch account bytes and deserialize them into any owned account type
    /// supported by `phoenix-rise-accounts`.
    ///
    /// This method only validates the account discriminator/layout. Use
    /// [`Self::fetch_verified_decoded`] or the typed Phoenix account helpers
    /// when reading directly from RPC.
    pub fn fetch_decoded<T: AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> Result<T, PhoenixAccountClientError> {
        let data = self.fetch_account_data(address)?;
        Ok(T::try_from_account_bytes(&data)?)
    }

    /// Fetch, verify owner metadata, and deserialize an account.
    pub fn fetch_verified_decoded<T: AccountDeserialize>(
        &self,
        address: &Pubkey,
        expected_owner: &Pubkey,
    ) -> Result<T, PhoenixAccountClientError> {
        let account = self
            .fetcher
            .fetch_account(address)
            .map_err(|error| PhoenixAccountClientError::Fetch(error.to_string()))?;

        if account.executable {
            return Err(PhoenixAccountClientError::ExecutableAccount);
        }

        let Some(owner) = account.owner else {
            return Err(PhoenixAccountClientError::OwnerUnavailable);
        };

        if owner != *expected_owner {
            return Err(PhoenixAccountClientError::OwnerMismatch {
                expected: *expected_owner,
                actual: owner,
            });
        }

        Ok(T::try_from_account_bytes(&account.data)?)
    }

    fn fetch_phoenix_decoded<T: AccountDeserialize>(
        &self,
        address: &Pubkey,
    ) -> Result<T, PhoenixAccountClientError> {
        let expected_owner = *PHOENIX_PROGRAM_ID;
        self.fetch_verified_decoded(address, &expected_owner)
    }

    pub fn fetch_global_configuration(
        &self,
        address: Option<Pubkey>,
    ) -> Result<GlobalConfiguration, PhoenixAccountClientError> {
        let address = address.unwrap_or(*PHOENIX_GLOBAL_CONFIGURATION);
        self.fetch_phoenix_decoded(&address)
    }

    pub fn fetch_conditional_order_collection(
        &self,
        address: &Pubkey,
    ) -> Result<ConditionalOrderCollection, PhoenixAccountClientError> {
        self.fetch_phoenix_decoded(address)
    }

    pub fn fetch_orderbook_header(
        &self,
        address: &Pubkey,
    ) -> Result<OrderbookHeader, PhoenixAccountClientError> {
        self.fetch_phoenix_decoded(address)
    }

    pub fn fetch_orderbook(
        &self,
        address: &Pubkey,
    ) -> Result<Orderbook, PhoenixAccountClientError> {
        self.fetch_phoenix_decoded(address)
    }

    pub fn fetch_perp_asset_map(
        &self,
        address: &Pubkey,
    ) -> Result<PerpAssetMapOwned, PhoenixAccountClientError> {
        self.fetch_phoenix_decoded(address)
    }

    pub fn fetch_permission(
        &self,
        address: &Pubkey,
    ) -> Result<Permission, PhoenixAccountClientError> {
        self.fetch_phoenix_decoded(address)
    }

    pub fn fetch_spline_collection(
        &self,
        address: &Pubkey,
    ) -> Result<SplineCollection, PhoenixAccountClientError> {
        self.fetch_phoenix_decoded(address)
    }

    pub fn fetch_stop_losses(
        &self,
        address: &Pubkey,
    ) -> Result<StopLosses, PhoenixAccountClientError> {
        self.fetch_phoenix_decoded(address)
    }

    pub fn fetch_trader(&self, address: &Pubkey) -> Result<Trader, PhoenixAccountClientError> {
        self.fetch_phoenix_decoded(address)
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
        self.fetch_phoenix_decoded(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct DummyAccount(Vec<u8>);

    impl AccountDeserialize for DummyAccount {
        fn try_from_account_bytes(data: &[u8]) -> Result<Self, AccountDeserializeError> {
            Ok(Self(data.to_vec()))
        }
    }

    #[derive(Clone)]
    struct TestFetcher {
        account: FetchedAccount,
    }

    impl AccountDataFetcher for TestFetcher {
        type Error = String;

        fn fetch_account_data(&self, _address: &Pubkey) -> Result<Vec<u8>, Self::Error> {
            Ok(self.account.data.clone())
        }

        fn fetch_account(&self, _address: &Pubkey) -> Result<FetchedAccount, Self::Error> {
            Ok(self.account.clone())
        }
    }

    #[test]
    fn verified_fetch_rejects_missing_owner_metadata() {
        let client = PhoenixAccountClient::new(TestFetcher {
            account: FetchedAccount::unchecked(vec![1, 2, 3]),
        });

        let result = client
            .fetch_verified_decoded::<DummyAccount>(&Pubkey::new_unique(), &Pubkey::new_unique());

        assert!(matches!(
            result,
            Err(PhoenixAccountClientError::OwnerUnavailable)
        ));
    }

    #[test]
    fn verified_fetch_rejects_wrong_owner() {
        let expected = Pubkey::new_unique();
        let actual = Pubkey::new_unique();
        let client = PhoenixAccountClient::new(TestFetcher {
            account: FetchedAccount::new(vec![1, 2, 3], actual, false),
        });

        let result =
            client.fetch_verified_decoded::<DummyAccount>(&Pubkey::new_unique(), &expected);

        assert!(matches!(
            result,
            Err(PhoenixAccountClientError::OwnerMismatch {
                expected: got_expected,
                actual: got_actual
            }) if got_expected == expected && got_actual == actual
        ));
    }

    #[test]
    fn verified_fetch_decodes_matching_owner() {
        let owner = Pubkey::new_unique();
        let client = PhoenixAccountClient::new(TestFetcher {
            account: FetchedAccount::new(vec![1, 2, 3], owner, false),
        });

        let decoded = client
            .fetch_verified_decoded::<DummyAccount>(&Pubkey::new_unique(), &owner)
            .unwrap();

        assert_eq!(decoded, DummyAccount(vec![1, 2, 3]));
    }
}
