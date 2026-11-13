use mockall::Sequence;
use phoenix_rise_accounts::trader::TRADER_CAPABILITY_HOT;

use super::*;

mockall::mock! {
    Fetcher {}
    impl AccountDataFetcher for Fetcher {
        type Error = String;
        fn fetch_account_data(&self, address: &Pubkey) -> Result<Vec<u8>, String>;
    }
    impl AccountSnapshotFetcher for Fetcher {
        fn fetch_account_snapshot(&self, addresses: &[Pubkey], min_context_slot: Option<u64>) -> Result<FetchedAccountSnapshot, String>;
        fn max_snapshot_accounts(&self) -> Option<usize>;
    }
}

fn u32_at(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn u64_at(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

struct Fixture {
    program: Pubkey,
    trader: Pubkey,
    keys: Vec<Pubkey>,
    accounts: Vec<Option<FetchedAccount>>,
}

impl Fixture {
    fn new(hot: bool) -> Self {
        let program = Pubkey::new_from_array([9; 32]);
        let authority = Pubkey::new_from_array([5; 32]);
        let trader =
            Pubkey::find_program_address(&[b"trader", authority.as_ref(), &[2, 3]], &program).0;
        let config_key = Pubkey::find_program_address(&[b"global"], &program).0;
        let gti_key = derive_global_trader_index_address(&program, 0);
        let atb_key = derive_active_trader_buffer_address(&program, 0);
        let keys = vec![
            config_key,
            trader,
            gti_key,
            derive_global_trader_index_address(&program, 1),
            atb_key,
            derive_active_trader_buffer_address(&program, 1),
        ];
        // Full on-chain account size, so every decoded field is present.
        let mut config = vec![0; 2560];
        config[..8].copy_from_slice(&PhoenixAccount::GlobalConfiguration.discriminant());
        config[8..40].copy_from_slice(config_key.as_ref());
        config[392..424].copy_from_slice(gti_key.as_ref());
        config[424..456].copy_from_slice(atb_key.as_ref());
        let mut account = vec![0; 240 + 4 * 40];
        account[..8].copy_from_slice(&PhoenixAccount::Trader.discriminant());
        account[24..56].copy_from_slice(trader.as_ref());
        account[56..88].copy_from_slice(authority.as_ref());
        u64_at(&mut account, 88, 111);
        u32_at(
            &mut account,
            96,
            if hot { TRADER_CAPABILITY_HOT } else { 0 },
        );
        account[154..156].copy_from_slice(&[2, 3]);
        u64_at(&mut account, 224, 4);
        u64_at(&mut account, 232, 4);
        for (index, asset, value) in [
            (0, 42, (-5i64) as u64),
            (1, 0xFFFE_0002, 300),
            (2, NATIVE_SOL_ASSET_INDEX as u64, 700),
            (3, LOCKED_DEPOSIT_ASSET_INDEX as u64, 500),
        ] {
            u64_at(&mut account, 240 + index * 40, asset);
            u64_at(&mut account, 248 + index * 40, value);
        }
        u64_at(&mut account, 376, 120);
        let mut gti = tree_header(PhoenixAccount::GlobalTraderIndexHeader, 1, 64, 1);
        gti[112..144].copy_from_slice(if hot { trader.as_ref() } else { &[7; 32] });
        u64_at(&mut gti, 144, 900);
        u32_at(&mut gti, 152, TRADER_CAPABILITY_HOT);
        let gti_overflow = overflow(PhoenixAccount::GlobalTraderIndexArenaHeader, 0);
        let mut atb = tree_header(PhoenixAccount::ActiveTraderBufferHeader, 2, 88, 1);
        u32_at(&mut atb, 112, 1);
        u32_at(&mut atb, 116, 7);
        u64_at(&mut atb, 120, 12);
        let atb_overflow = overflow(PhoenixAccount::ActiveTraderBufferArenaHeader, 176);
        let accounts = [config, account, gti, gti_overflow, atb, atb_overflow]
            .into_iter()
            .map(|bytes| Some(FetchedAccount::new(bytes, program, false)))
            .collect();
        Self {
            program,
            trader,
            keys,
            accounts,
        }
    }

    fn discovery(&self) -> FetchedAccountSnapshot {
        FetchedAccountSnapshot {
            context_slot: 100,
            accounts: vec![self.accounts[2].clone(), self.accounts[4].clone()],
        }
    }

    fn snapshot(&self) -> FetchedAccountSnapshot {
        FetchedAccountSnapshot {
            context_slot: 102,
            accounts: self.accounts.clone(),
        }
    }

    fn client(
        &self,
        discovery: FetchedAccountSnapshot,
        final_snapshot: FetchedAccountSnapshot,
    ) -> PhoenixAccountClient<MockFetcher> {
        let mut fetcher = MockFetcher::new();
        let mut sequence = Sequence::new();
        fetcher
            .expect_max_snapshot_accounts()
            .return_const(Some(100));
        let headers = [self.keys[2], self.keys[4]];
        fetcher
            .expect_fetch_account_snapshot()
            .withf(move |keys, minimum| keys == headers && minimum.is_none())
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(|_, _| Ok(discovery));
        let expected_keys = self.keys.clone();
        fetcher
            .expect_fetch_account_snapshot()
            .withf(move |keys, minimum| keys == expected_keys && *minimum == Some(100))
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(|_, _| Ok(final_snapshot));
        PhoenixAccountClient::new(fetcher)
    }
}

fn tree_header(kind: PhoenixAccount, capacity: usize, node_size: usize, len: u32) -> Vec<u8> {
    let mut bytes = vec![0; 96 + capacity * node_size];
    bytes[..8].copy_from_slice(&kind.discriminant());
    u32_at(&mut bytes, 48, len);
    bytes[52..54].copy_from_slice(&2u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&1u16.to_le_bytes());
    u32_at(&mut bytes, 56, capacity as u32);
    u32_at(&mut bytes, 60, len + 1);
    u32_at(&mut bytes, 64, len + 1);
    u32_at(&mut bytes, 80, u32::from(len != 0));
    bytes
}

fn overflow(kind: PhoenixAccount, data_size: usize) -> Vec<u8> {
    let mut bytes = vec![0; 32 + data_size];
    bytes[..8].copy_from_slice(&kind.discriminant());
    bytes[8..10].copy_from_slice(&1u16.to_le_bytes());
    bytes
}

#[test]
fn hot_snapshot_uses_final_batch_and_never_resurrects_cold_entries() {
    let fixture = Fixture::new(true);
    let mut discovery = fixture.discovery();
    // Discovery data is never used as authoritative trader state.
    u64_at(&mut discovery.accounts[0].as_mut().unwrap().data, 144, 400);
    let client = fixture.client(discovery, fixture.snapshot());
    let snapshot = client
        .fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program)
        .unwrap();
    assert_eq!(snapshot.context_slot(), 102);
    assert_eq!(snapshot.address(), fixture.trader);
    assert_eq!(snapshot.global_trader_index_node(), Some(1));
    assert_eq!(snapshot.state().quote_lot_collateral.as_inner(), 900);
    assert_eq!(
        snapshot
            .header()
            .trader_state
            .quote_lot_collateral
            .as_inner(),
        111
    );
    assert_eq!(
        snapshot
            .positions()
            .iter()
            .map(|p| p.asset_id)
            .collect::<Vec<_>>(),
        [7]
    );
    assert_eq!(
        snapshot.positions()[0]
            .position
            .base_lot_position
            .as_inner(),
        12
    );
    assert_eq!(snapshot.active_positions().len(), 1);
    assert!(snapshot.spot_collaterals().is_empty());
    assert!(snapshot.locked_deposit().is_none());
}

#[test]
fn cold_snapshot_selects_trader_account_instead_of_another_traders_active_state() {
    let fixture = Fixture::new(false);
    let client = fixture.client(fixture.discovery(), fixture.snapshot());
    let snapshot = client
        .fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program)
        .unwrap();
    assert!(!snapshot.is_active());
    assert_eq!(snapshot.state().quote_lot_collateral.as_inner(), 111);
    assert_eq!(snapshot.positions()[0].asset_id, 42);
    assert_eq!(
        snapshot.positions()[0]
            .position
            .base_lot_position
            .as_inner(),
        -5
    );
    assert!(snapshot.active_positions().is_empty());
    assert_eq!(
        snapshot
            .spot_collaterals()
            .iter()
            .map(|s| s.balance)
            .collect::<Vec<_>>(),
        [300, 700]
    );
    let lock = snapshot.locked_deposit().unwrap();
    assert_eq!(
        (lock.locked_quote_lots, lock.consumed_quote_lots),
        (500, 120)
    );
}

#[test]
fn hot_snapshot_includes_native_sol_spot_and_lock_from_overflow_arena() {
    let mut fixture = Fixture::new(true);
    let head = &mut fixture.accounts[4].as_mut().unwrap().data;
    u32_at(head, 48, 4);
    u32_at(head, 60, 5);
    u32_at(head, 64, 5);
    u32_at(head, 80, 2);
    u32_at(head, 104, 2);
    u32_at(head, 184, 1);
    u32_at(head, 188, 3);
    u32_at(head, 200, 1);
    u32_at(head, 204, 0xFFFE_0002);
    u64_at(head, 208, 600);
    u64_at(head, 216, 900);
    let tail = &mut fixture.accounts[5].as_mut().unwrap().data;
    u32_at(tail, 36, 4);
    u32_at(tail, 40, 2);
    u32_at(tail, 48, 1);
    u32_at(tail, 52, NATIVE_SOL_ASSET_INDEX);
    u64_at(tail, 56, 800);
    u32_at(tail, 128, 3);
    u32_at(tail, 136, 1);
    u32_at(tail, 140, LOCKED_DEPOSIT_ASSET_INDEX);
    u64_at(tail, 144, 1000);
    u64_at(tail, 152, 200);
    let client = fixture.client(fixture.discovery(), fixture.snapshot());
    let snapshot = client
        .fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program)
        .unwrap();
    assert_eq!(snapshot.positions().len(), 1);
    assert_eq!(
        snapshot
            .spot_collaterals()
            .iter()
            .map(|s| s.balance)
            .collect::<Vec<_>>(),
        [600, 800]
    );
    assert_eq!(
        snapshot.spot_collaterals()[0].max_per_trader_balance_override,
        Some(900)
    );
    assert_eq!(
        snapshot.locked_deposit().unwrap().remaining_quote_lots(),
        800
    );
}

#[test]
fn snapshot_rejects_both_arena_growth_and_shrink_and_an_older_final_slot() {
    let fixture = Fixture::new(true);
    for index in [2, 4] {
        for count in [1u16, 3] {
            let mut final_snapshot = fixture.snapshot();
            final_snapshot.accounts[index].as_mut().unwrap().data[52..54]
                .copy_from_slice(&count.to_le_bytes());
            let client = fixture.client(fixture.discovery(), final_snapshot);
            assert!(
                matches!(client.fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program),
                Err(TraderSnapshotError::ArenaCountChanged { discovered: 2, actual, .. }) if actual == count)
            );
        }
    }
    let mut old = fixture.snapshot();
    old.context_slot = 99;
    let client = fixture.client(fixture.discovery(), old);
    assert!(matches!(
        client.fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program),
        Err(TraderSnapshotError::ContextTooOld {
            minimum: 100,
            actual: 99
        })
    ));
}

#[test]
fn snapshot_rejects_missing_accounts_owner_metadata_and_invalid_identities() {
    let fixture = Fixture::new(true);
    for index in 0..fixture.accounts.len() {
        let mut missing = fixture.snapshot();
        missing.accounts[index] = None;
        let client = fixture.client(fixture.discovery(), missing);
        assert!(
            matches!(client.fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program),
            Err(TraderSnapshotError::MissingAccount { address }) if address == fixture.keys[index])
        );
        for owner in [None, Some(Pubkey::new_from_array([6; 32]))] {
            let mut invalid = fixture.snapshot();
            invalid.accounts[index].as_mut().unwrap().owner = owner;
            let client = fixture.client(fixture.discovery(), invalid);
            assert!(matches!(
                client.fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program),
                Err(TraderSnapshotError::AccountClient(
                    PhoenixAccountClientError::OwnerUnavailable
                        | PhoenixAccountClientError::OwnerMismatch { .. }
                ))
            ));
        }
    }
    for (index, offset) in [(0, 392), (1, 24), (1, 56), (1, 154)] {
        let mut invalid = fixture.snapshot();
        invalid.accounts[index].as_mut().unwrap().data[offset] ^= 1;
        let client = fixture.client(fixture.discovery(), invalid);
        assert!(matches!(
            client.fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program),
            Err(TraderSnapshotError::InvalidIdentity { .. })
        ));
    }
}

#[test]
fn snapshot_reports_inconsistent_hot_membership_and_unsupported_entries() {
    for hot in [false, true] {
        let fixture = Fixture::new(hot);
        let mut invalid = fixture.snapshot();
        invalid.accounts[1].as_mut().unwrap().data[96] ^= 1;
        let client = fixture.client(fixture.discovery(), invalid);
        assert!(matches!(
            client.fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program),
            Err(TraderSnapshotError::InconsistentTraderState { .. })
        ));
        let mut invalid = fixture.snapshot();
        let (index, offset) = if hot { (4, 116) } else { (1, 240) };
        u32_at(
            &mut invalid.accounts[index].as_mut().unwrap().data,
            offset,
            0xFF00_0001,
        );
        let client = fixture.client(fixture.discovery(), invalid);
        assert!(matches!(
            client.fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program),
            Err(TraderSnapshotError::UnsupportedEntry {
                asset_index: 0xFF00_0001,
                ..
            })
        ));
    }
}

#[test]
fn snapshot_does_not_chunk_requests_beyond_transport_limit() {
    let fixture = Fixture::new(true);
    let mut fetcher = MockFetcher::new();
    fetcher.expect_max_snapshot_accounts().return_const(Some(5));
    let discovery = fixture.discovery();
    fetcher
        .expect_fetch_account_snapshot()
        .times(1)
        .return_once(|_, _| Ok(discovery));
    let client = PhoenixAccountClient::new(fetcher);
    assert!(matches!(
        client.fetch_trader_snapshot_for_program(&fixture.trader, &fixture.program),
        Err(TraderSnapshotError::TooManyAccounts {
            count: 6,
            maximum: 5
        })
    ));
}
