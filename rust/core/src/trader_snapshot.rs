//! Consistent account snapshots and authoritative hot/cold trader state.

use phoenix_rise_accounts::active_trader_buffer::{
    ActiveTraderBuffer, ActiveTraderPosition, TraderPositionId,
};
use phoenix_rise_accounts::global_config::GlobalConfig;
use phoenix_rise_accounts::global_trader_index::GlobalTraderIndex;
use phoenix_rise_accounts::multi_arena::MultiArenaHeader;
use phoenix_rise_accounts::pda::{
    derive_active_trader_buffer_address, derive_global_trader_index_address,
};
use phoenix_rise_accounts::trader::{
    LOCKED_DEPOSIT_ASSET_INDEX, NATIVE_SOL_ASSET_INDEX, Trader, TraderHeader, TraderLockedDeposit,
    TraderPositionEntry, TraderSpotCollateral, TraderState,
};
use phoenix_rise_accounts::{PhoenixAccount, PhoenixAccountDecodeError};
use phoenix_rise_ix::constants::PHOENIX_PROGRAM_ID;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::client_error::ErrorKind;
use solana_rpc_client_api::config::RpcAccountInfoConfig;
use solana_rpc_client_api::response::UiAccountEncoding;
use thiserror::Error;

use crate::account_client::{
    AccountDataFetcher, FetchedAccount, PhoenixAccountClient, PhoenixAccountClientError,
    verify_account_owner,
};

/// Account values from one atomic source snapshot, in requested-address order.
/// Missing accounts must occupy a `None` entry, preserving the batch length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedAccountSnapshot {
    pub context_slot: u64,
    pub accounts: Vec<Option<FetchedAccount>>,
}

/// Optional atomic batch capability for account-client transports.
///
/// All returned values must come from the same source snapshot. Do not
/// implement this by joining sequential account requests, even when they report
/// the same slot: multiple banks or updates within that slot can disagree. A
/// minimum context slot is a freshness floor, not a substitute for an atomic
/// batch. Existing byte-only [`AccountDataFetcher`] implementations remain
/// valid and do not gain snapshot methods until they implement this capability.
pub trait AccountSnapshotFetcher: AccountDataFetcher {
    fn fetch_account_snapshot(
        &self,
        addresses: &[Pubkey],
        min_context_slot: Option<u64>,
    ) -> Result<FetchedAccountSnapshot, Self::Error>;

    /// Maximum accounts in one atomic request, or no SDK-imposed limit.
    fn max_snapshot_accounts(&self) -> Option<usize> {
        None
    }
}

impl AccountSnapshotFetcher for RpcClient {
    fn fetch_account_snapshot(
        &self,
        addresses: &[Pubkey],
        min_context_slot: Option<u64>,
    ) -> Result<FetchedAccountSnapshot, Self::Error> {
        if addresses.len() > 100 {
            return Err(ErrorKind::Custom(
                "a consistent RPC snapshot cannot exceed 100 accounts".into(),
            )
            .into());
        }
        let response = self.get_multiple_ui_accounts_with_config(
            addresses,
            RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base64),
                commitment: Some(self.commitment()),
                min_context_slot,
                ..Default::default()
            },
        )?;
        let accounts = response
            .value
            .into_iter()
            .map(|account| {
                account
                    .map(|account| {
                        account
                            .to_account()
                            .map(|account| {
                                FetchedAccount::new(account.data, account.owner, account.executable)
                            })
                            .ok_or_else(|| {
                                ErrorKind::Custom(
                                    "RPC returned invalid account bytes or owner".into(),
                                )
                                .into()
                            })
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>, Self::Error>>()?;
        Ok(FetchedAccountSnapshot {
            context_slot: response.context.slot,
            accounts,
        })
    }

    fn max_snapshot_accounts(&self) -> Option<usize> {
        Some(100)
    }
}

#[derive(Debug, Error)]
pub enum TraderSnapshotError {
    #[error(transparent)]
    AccountClient(#[from] PhoenixAccountClientError),
    #[error(transparent)]
    Decode(#[from] PhoenixAccountDecodeError),
    #[error("required snapshot account {address} is missing")]
    MissingAccount { address: Pubkey },
    #[error("snapshot returned {actual} accounts for {expected} requested addresses")]
    BatchLength { expected: usize, actual: usize },
    #[error("snapshot slot {actual} precedes discovery slot {minimum}")]
    ContextTooOld { minimum: u64, actual: u64 },
    #[error(
        "consistent snapshot needs {count} accounts, exceeding this fetcher's limit of {maximum}"
    )]
    TooManyAccounts { count: usize, maximum: usize },
    #[error(
        "{account} arena count changed from {discovered} to {actual}; retry the complete snapshot \
         load"
    )]
    ArenaCountChanged {
        account: &'static str,
        discovered: u16,
        actual: u16,
    },
    #[error("snapshot account {address} has an invalid identity: {reason}")]
    InvalidIdentity {
        address: Pubkey,
        reason: &'static str,
    },
    #[error("trader {address} hot/cold flags disagree with its global-index membership")]
    InconsistentTraderState { address: Pubkey },
    #[error(
        "trader {address} has unsupported entry {asset_index:#x}; cannot return a complete typed \
         snapshot"
    )]
    UnsupportedEntry { address: Pubkey, asset_index: u32 },
}

/// A trader's authoritative state from one contextual account batch.
///
/// GTI membership selects all mutable state: GTI collateral/state plus ATB
/// positions, spot collateral and locked deposits for hot traders; the trader
/// account for cold traders. Absent hot entries never fall back to stale cold
/// copies. Market prices, risk parameters and orderbook contents are outside
/// this snapshot.
#[derive(Debug, Clone)]
pub struct TraderSnapshot {
    context_slot: u64,
    address: Pubkey,
    header: TraderHeader,
    state: TraderState,
    global_trader_index_node: Option<u32>,
    positions: Vec<TraderPositionEntry>,
    active_positions: Vec<(TraderPositionId, ActiveTraderPosition)>,
    spot_collaterals: Vec<TraderSpotCollateral>,
    locked_deposit: Option<TraderLockedDeposit>,
}

impl TraderSnapshot {
    pub const fn context_slot(&self) -> u64 {
        self.context_slot
    }

    pub const fn address(&self) -> Pubkey {
        self.address
    }

    /// Raw account metadata. Its cached `trader_state` may be stale while hot;
    /// use [`Self::state`] for authoritative collateral and capability flags.
    pub const fn header(&self) -> &TraderHeader {
        &self.header
    }

    pub const fn state(&self) -> &TraderState {
        &self.state
    }

    /// Snapshot-local GTI pointer; a later snapshot may reuse it for another
    /// trader.
    pub const fn global_trader_index_node(&self) -> Option<u32> {
        self.global_trader_index_node
    }

    pub const fn is_active(&self) -> bool {
        self.global_trader_index_node.is_some()
    }

    pub fn positions(&self) -> &[TraderPositionEntry] {
        &self.positions
    }

    /// Hot positions with resting-order totals. Empty for a cold trader.
    pub fn active_positions(&self) -> &[(TraderPositionId, ActiveTraderPosition)] {
        &self.active_positions
    }

    /// Native SOL and SPL spot collateral selected from the authoritative
    /// source.
    pub fn spot_collaterals(&self) -> &[TraderSpotCollateral] {
        &self.spot_collaterals
    }

    pub const fn locked_deposit(&self) -> Option<TraderLockedDeposit> {
        self.locked_deposit
    }
}

impl<F: AccountSnapshotFetcher> PhoenixAccountClient<F> {
    /// Load a consistent trader snapshot using the configured Phoenix
    /// deployment.
    ///
    /// Discovers arena counts, then fetches the trader, global configuration
    /// and every GTI/ATB arena together. Arena growth or shrinkage between
    /// requests returns [`TraderSnapshotError::ArenaCountChanged`]; retry
    /// this method to rediscover. No sequential batching or stale-state
    /// fallback is performed.
    pub fn fetch_trader_snapshot(
        &self,
        trader: &Pubkey,
    ) -> Result<TraderSnapshot, TraderSnapshotError> {
        self.fetch_trader_snapshot_for_program(trader, &PHOENIX_PROGRAM_ID)
    }

    /// Load against an explicit deployment, deriving all account PDAs from it.
    /// The underlying fetcher's configured commitment is retained.
    pub fn fetch_trader_snapshot_for_program(
        &self,
        trader: &Pubkey,
        program: &Pubkey,
    ) -> Result<TraderSnapshot, TraderSnapshotError> {
        let gti_key = derive_global_trader_index_address(program, 0);
        let atb_key = derive_active_trader_buffer_address(program, 0);
        let discovery_keys = [gti_key, atb_key];
        let discovery = self.fetch_snapshot_batch(&discovery_keys, None)?;
        let gti_count = arena_count(
            required_account(&discovery, 0, &gti_key, program)?,
            PhoenixAccount::GlobalTraderIndexHeader,
        )?;
        let atb_count = arena_count(
            required_account(&discovery, 1, &atb_key, program)?,
            PhoenixAccount::ActiveTraderBufferHeader,
        )?;
        let config_key = Pubkey::find_program_address(&[b"global"], program).0;
        let keys = [config_key, *trader]
            .into_iter()
            .chain((0..gti_count).map(|index| derive_global_trader_index_address(program, index)))
            .chain((0..atb_count).map(|index| derive_active_trader_buffer_address(program, index)))
            .collect::<Vec<_>>();
        let snapshot = self.fetch_snapshot_batch(&keys, Some(discovery.context_slot))?;
        let atb_start = 2 + usize::from(gti_count);
        for (index, kind, discovered) in [
            (2, PhoenixAccount::GlobalTraderIndexHeader, gti_count),
            (
                atb_start,
                PhoenixAccount::ActiveTraderBufferHeader,
                atb_count,
            ),
        ] {
            let actual = arena_count(
                required_account(&snapshot, index, &keys[index], program)?,
                kind,
            )?;
            if actual != discovered {
                return Err(TraderSnapshotError::ArenaCountChanged {
                    account: kind.account_name(),
                    discovered,
                    actual,
                });
            }
        }
        let accounts = keys
            .iter()
            .enumerate()
            .map(|(index, key)| required_account(&snapshot, index, key, program))
            .collect::<Result<Vec<_>, _>>()?;
        let config = GlobalConfig::try_from_account_bytes(&accounts[0].data)?;
        if config.account_key() != config_key.to_bytes()
            || config.global_trader_index_header_key() != gti_key.to_bytes()
            || config.active_trader_buffer_header_key() != atb_key.to_bytes()
        {
            return Err(TraderSnapshotError::InvalidIdentity {
                address: config_key,
                reason: "configuration does not reference the derived account PDAs",
            });
        }
        let gti = GlobalTraderIndex::try_from_account_buffers(
            accounts[2..atb_start].iter().map(|a| a.data.as_slice()),
        )?;
        let atb = ActiveTraderBuffer::try_from_account_buffers(
            accounts[atb_start..].iter().map(|a| a.data.as_slice()),
        )?;
        // The trader view borrows aligned POD fields; transports only promise
        // bytes.
        let data = &accounts[1].data;
        let mut aligned = vec![0u64; data.len().div_ceil(8)];
        bytemuck::cast_slice_mut::<u64, u8>(&mut aligned)[..data.len()].copy_from_slice(data);
        let trader_view = Trader::try_from_account_bytes(
            &bytemuck::cast_slice::<u64, u8>(&aligned)[..data.len()],
        )?;
        build_trader_snapshot(
            snapshot.context_slot,
            *trader,
            program,
            trader_view,
            &gti,
            &atb,
        )
    }

    fn fetch_snapshot_batch(
        &self,
        keys: &[Pubkey],
        minimum: Option<u64>,
    ) -> Result<FetchedAccountSnapshot, TraderSnapshotError> {
        if let Some(maximum) = self.fetcher().max_snapshot_accounts()
            && keys.len() > maximum
        {
            return Err(TraderSnapshotError::TooManyAccounts {
                count: keys.len(),
                maximum,
            });
        }
        let snapshot = self
            .fetcher()
            .fetch_account_snapshot(keys, minimum)
            .map_err(|e| PhoenixAccountClientError::Fetch(e.to_string()))?;
        if snapshot.accounts.len() != keys.len() {
            return Err(TraderSnapshotError::BatchLength {
                expected: keys.len(),
                actual: snapshot.accounts.len(),
            });
        }
        if let Some(minimum) = minimum
            && snapshot.context_slot < minimum
        {
            return Err(TraderSnapshotError::ContextTooOld {
                minimum,
                actual: snapshot.context_slot,
            });
        }
        Ok(snapshot)
    }
}

fn required_account<'a>(
    snapshot: &'a FetchedAccountSnapshot,
    index: usize,
    address: &Pubkey,
    program: &Pubkey,
) -> Result<&'a FetchedAccount, TraderSnapshotError> {
    let account = snapshot
        .accounts
        .get(index)
        .and_then(Option::as_ref)
        .ok_or(TraderSnapshotError::MissingAccount { address: *address })?;
    verify_account_owner(account, program)?;
    Ok(account)
}

fn arena_count(account: &FetchedAccount, kind: PhoenixAccount) -> Result<u16, TraderSnapshotError> {
    let count = MultiArenaHeader::try_from_account_bytes(
        kind.account_name(),
        &account.data,
        kind.discriminant(),
    )?
    .num_arenas();
    if count == 0 {
        return Err(PhoenixAccountDecodeError::InvalidData {
            account: kind.account_name(),
            reason: "uninitialized multi-arena tree",
        }
        .into());
    }
    Ok(count)
}

fn supported_entry(asset: u32) -> bool {
    asset < 0xFF00_0000
        || asset & 0xFFFF_0000 == 0xFFFE_0000
        || asset == NATIVE_SOL_ASSET_INDEX
        || asset == LOCKED_DEPOSIT_ASSET_INDEX
}

fn build_trader_snapshot(
    context_slot: u64,
    address: Pubkey,
    program: &Pubkey,
    trader: Trader<'_>,
    gti: &GlobalTraderIndex<'_>,
    atb: &ActiveTraderBuffer<'_>,
) -> Result<TraderSnapshot, TraderSnapshotError> {
    let header = *trader.header();
    let derived = Pubkey::find_program_address(
        &[
            b"trader",
            &header.authority,
            &[header.trader_pda_index, header.trader_subaccount_index],
        ],
        program,
    )
    .0;
    if header.key != address.to_bytes() || derived != address {
        return Err(TraderSnapshotError::InvalidIdentity {
            address,
            reason: "trader key or authority/index seeds do not match its address",
        });
    }
    let node = gti.index(&address);
    if header.trader_state.is_hot() != node.is_some() {
        return Err(TraderSnapshotError::InconsistentTraderState { address });
    }
    let (state, positions, active_positions, spot_collaterals, locked_deposit) =
        if let Some(node) = node {
            if let Some(id) = atb
                .iter_entry_ids(node)
                .find(|id| !supported_entry(id.asset_id()))
            {
                return Err(TraderSnapshotError::UnsupportedEntry {
                    address,
                    asset_index: id.asset_id(),
                });
            }
            let state = gti
                .get(&address)
                .ok_or(TraderSnapshotError::InconsistentTraderState { address })?;
            if !state.is_hot() {
                return Err(TraderSnapshotError::InconsistentTraderState { address });
            }
            let active: Vec<_> = atb.iter_positions(node).collect();
            let positions = active
                .iter()
                .map(|(id, position)| TraderPositionEntry {
                    asset_id: u64::from(id.asset_id()),
                    position: position.position,
                })
                .collect();
            let spot = atb
                .iter_spot_collaterals(node)
                .chain(atb.get_native_sol_collateral(node))
                .collect();
            (state, positions, active, spot, atb.get_locked_deposit(node))
        } else {
            if let Some(entry) = trader
                .allocated_position_entries()
                .take(trader.raw_len() as usize)
                .find(|entry| !supported_entry(entry.asset_id() as u32))
            {
                return Err(TraderSnapshotError::UnsupportedEntry {
                    address,
                    asset_index: entry.asset_id() as u32,
                });
            }
            let positions = trader
                .positions()
                .map(|(asset_id, position)| TraderPositionEntry {
                    asset_id,
                    position: position.to_position(),
                })
                .collect();
            (
                header.trader_state,
                positions,
                Vec::new(),
                trader.spot_collaterals().collect(),
                trader.locked_deposit(),
            )
        };
    Ok(TraderSnapshot {
        context_slot,
        address,
        header,
        state,
        global_trader_index_node: node,
        positions,
        active_positions,
        spot_collaterals,
        locked_deposit,
    })
}

#[cfg(test)]
mod tests;
