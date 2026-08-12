//! High-level helper mirroring the TS `PhoenixFlightClient`
//! (`ts/src/flight/client.ts`).
//!
//! A Flight builder wraps supported Phoenix placement instructions in a
//! `proxy_instruction` before forwarding execution to Phoenix.

use phoenix_rise_ix::flight::{ProxyInstructionParams, create_proxy_instruction_ix};
use phoenix_rise_ix::types::{AccountMeta, Instruction as RiseInstruction};
use phoenix_rise_ix::{PhoenixInstruction, PhoenixIxError};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::exchange_cache::SharedExchangeCacheStore;
use crate::trader_key::TraderKey;

/// Builder identity (authority plus trader-subaccount indices) of a
/// registered Flight builder, together with the exchange snapshot store used
/// to derive the collateral-transfer permission PDA for position-authority
/// wraps and an optional client-level builder fee override.
#[derive(Debug, Clone)]
pub struct PhoenixFlightClient {
    pub builder_authority: Pubkey,
    pub builder_pda_index: u8,
    pub builder_subaccount_index: u8,
    /// Source of the Phoenix root authority for position-authority wraps,
    /// resolved fresh per wrap so on-chain root-authority rotations
    /// propagate. Obtain the store from `PhoenixWSClient::exchange_store()`,
    /// which owns the subscription pump and returns it populated; manual
    /// construction plus `subscribe_to_exchange_cache` is the advanced path
    /// for custom stores. When `None`, position-authority wraps fail with
    /// [`PhoenixIxError::MissingRootAuthority`].
    pub exchange_store: Option<SharedExchangeCacheStore>,
    /// Client-level builder fee override applied by
    /// [`Self::try_wrap_order_instruction`]; the per-call argument of
    /// [`Self::try_wrap_order_instruction_with_fee_bps_override`] wins over
    /// it.
    pub fee_bps_override: Option<u64>,
}

impl PhoenixFlightClient {
    /// Client without an exchange store. Owner-signed wraps work as usual;
    /// position-authority wraps fail with
    /// [`PhoenixIxError::MissingRootAuthority`] because no root authority can
    /// be resolved. Prefer [`Self::from_exchange_store`] when
    /// position-authority wraps are needed.
    pub fn new(
        builder_authority: Pubkey,
        builder_pda_index: u8,
        builder_subaccount_index: u8,
    ) -> Self {
        Self {
            builder_authority,
            builder_pda_index,
            builder_subaccount_index,
            exchange_store: None,
            fee_bps_override: None,
        }
    }

    /// Resolve the root authority at wrap time from a live exchange snapshot
    /// store, so on-chain root-authority rotations are picked up
    /// automatically. Obtain the store from
    /// `PhoenixWSClient::exchange_store()`, which owns the subscription pump
    /// (keep the websocket client alive — dropping it stops updates) and
    /// returns the store populated. Manual construction via
    /// `SharedExchangeCacheStore::new` plus `subscribe_to_exchange_cache` is
    /// the advanced path for custom stores.
    pub fn from_exchange_store(
        builder_authority: Pubkey,
        builder_pda_index: u8,
        builder_subaccount_index: u8,
        store: SharedExchangeCacheStore,
    ) -> Self {
        Self {
            builder_authority,
            builder_pda_index,
            builder_subaccount_index,
            exchange_store: Some(store),
            fee_bps_override: None,
        }
    }

    /// Use `fee_bps_override` for every wrap made through
    /// [`Self::try_wrap_order_instruction`]. The per-call argument of
    /// [`Self::try_wrap_order_instruction_with_fee_bps_override`] wins over
    /// this client-level configuration.
    pub fn with_fee_bps_override(mut self, fee_bps_override: u64) -> Self {
        self.fee_bps_override = Some(fee_bps_override);
        self
    }

    /// The builder's own trader subaccount PDA, which receives fee credit.
    pub fn builder_trader_account(&self) -> Pubkey {
        TraderKey::derive_pda(
            &self.builder_authority,
            self.builder_pda_index,
            self.builder_subaccount_index,
        )
    }

    /// Wrap `ix` in a Flight `proxy_instruction` if Flight supports routing it;
    /// return it unchanged otherwise.
    ///
    /// `signer` is placed verbatim in the proxy's trader-wallet slot.
    /// `use_position_authority` declares the signer kind and is the one rule
    /// deciding the collateral-transfer tail; it mirrors the API-server DTO
    /// shape, where callers holding the trader account's state compute it as
    /// `position_authority.is_some_and(|pa| pa != authority)` — when the flag
    /// is set, `signer` is that delegate key. Owner-signed orders (`false`,
    /// including owner-signed `PlaceMarketOrderDelegated`) carry no tail:
    /// on-chain, Flight collects the fee through the plain
    /// `TransferCollateral` path. Position-authority-signed orders (`true`)
    /// get the collateral-transfer authority and permission accounts appended
    /// so Flight can collect the builder fee via
    /// `AuthorizedTransferCollateral` (a plain `TransferCollateral` needs the
    /// owner's signature, which is absent). The permission account is derived
    /// from the root authority in the client's configured exchange snapshot
    /// store, resolved fresh at call time so on-chain rotations propagate;
    /// the position-authority wrap fails with
    /// [`PhoenixIxError::MissingRootAuthority`] when no store is configured,
    /// the store has not applied a snapshot yet, or the store's root
    /// authority is unset or invalid.
    ///
    /// Routable instructions are wrapped with the client-level
    /// [`Self::with_fee_bps_override`] configuration, if any; use
    /// [`Self::try_wrap_order_instruction_with_fee_bps_override`] for a
    /// per-call override.
    ///
    /// Mirrors TS `PhoenixFlightClient.tryWrapOrderInstruction` at
    /// `ts/src/flight/client.ts`.
    pub fn try_wrap_order_instruction(
        &self,
        ix: Instruction,
        signer: Pubkey,
        use_position_authority: bool,
    ) -> Result<Instruction, PhoenixIxError> {
        self.try_wrap_order_instruction_with_fee_bps_override(
            ix,
            signer,
            use_position_authority,
            self.fee_bps_override,
        )
    }

    /// [`Self::try_wrap_order_instruction`] with a per-call builder fee
    /// override: when `fee_bps_override` is `Some`, routable instructions are
    /// wrapped in Flight's `proxy_instruction_with_fee_override` variant. The
    /// explicit argument wins over the client-level
    /// [`Self::with_fee_bps_override`] configuration for this call — passing
    /// `None` wraps without a fee override even when the client-level
    /// configuration is set.
    pub fn try_wrap_order_instruction_with_fee_bps_override(
        &self,
        ix: Instruction,
        signer: Pubkey,
        use_position_authority: bool,
        fee_bps_override: Option<u64>,
    ) -> Result<Instruction, PhoenixIxError> {
        if !is_flight_routable_instruction(&ix) {
            return Ok(ix);
        }

        let inner = to_phoenix_rise_ix_instruction(ix);

        let mut params_builder = ProxyInstructionParams::builder()
            .builder_authority(self.builder_authority)
            .builder_trader_account(self.builder_trader_account())
            .trader_wallet(signer)
            .inner_instruction(inner);
        if let Some(fee_bps_override) = fee_bps_override {
            params_builder = params_builder.fee_bps_override(fee_bps_override);
        }
        if use_position_authority {
            let root_authority = self
                .exchange_store
                .as_ref()
                .and_then(|store| store.root_authority())
                .ok_or(PhoenixIxError::MissingRootAuthority)?;
            params_builder = params_builder.root_authority(root_authority);
        }
        let params = params_builder.build()?;

        Ok(create_proxy_instruction_ix(params)?.into())
    }
}

/// Returns true if the instruction targets a Flight-routable placement
/// instruction. Mirrors TS `isFlightRoutableInstruction`
/// (`ts/src/flight/helper.ts`).
pub fn is_flight_routable_instruction(ix: &Instruction) -> bool {
    has_discriminant(
        &ix.data,
        &PhoenixInstruction::PlaceLimitOrder.discriminant(),
    ) || has_discriminant(
        &ix.data,
        &PhoenixInstruction::PlaceMarketOrder.discriminant(),
    ) || has_discriminant(
        &ix.data,
        &PhoenixInstruction::PlaceMarketOrderDelegated.discriminant(),
    ) || has_discriminant(&ix.data, &PhoenixInstruction::PlaceStopLoss.discriminant())
        || has_discriminant(
            &ix.data,
            &PhoenixInstruction::PlacePositionConditionalOrder.discriminant(),
        )
        || has_discriminant(
            &ix.data,
            &PhoenixInstruction::PlaceAttachedConditionalOrder.discriminant(),
        )
        || has_discriminant(
            &ix.data,
            &PhoenixInstruction::PlaceLimitOrderWithConditionals.discriminant(),
        )
}

fn has_discriminant(data: &[u8], discriminant: &[u8; 8]) -> bool {
    data.len() >= discriminant.len() && data[..discriminant.len()] == *discriminant
}

/// Convert a `solana_instruction::Instruction` into a
/// `phoenix_rise_ix::types::Instruction` (same wire shape; distinct types in
/// the IX crate to keep it solana-dependency-light).
fn to_phoenix_rise_ix_instruction(ix: Instruction) -> RiseInstruction {
    let accounts = ix
        .accounts
        .into_iter()
        .map(|meta| AccountMeta {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();

    RiseInstruction {
        program_id: ix.program_id,
        accounts,
        data: ix.data,
    }
}

#[cfg(test)]
mod tests {
    use phoenix_rise_ix::limit_order::{LimitOrderParams, create_place_limit_order_ix};
    use phoenix_rise_ix::types::Side;
    use phoenix_rise_types::prelude::{
        AuthoritySet, ExchangeDeltaMessage, ExchangeDeltaOp, ExchangeSnapshotView,
        ExchangeStateSnapshot,
    };
    use solana_instruction::AccountMeta as SolAccountMeta;

    use super::*;

    fn build_sample_limit_ix() -> Instruction {
        let params = LimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(Side::Bid)
            .price_in_ticks(1000)
            .num_base_lots(100)
            .build()
            .unwrap();
        create_place_limit_order_ix(params).unwrap().into()
    }

    fn build_sample_phoenix_ix(discriminant: [u8; 8]) -> Instruction {
        Instruction {
            program_id: *phoenix_rise_ix::PHOENIX_PROGRAM_ID,
            accounts: vec![SolAccountMeta::new_readonly(Pubkey::new_unique(), false)],
            data: discriminant.to_vec(),
        }
    }

    fn exchange_state_snapshot(root_authority: Pubkey) -> ExchangeStateSnapshot {
        ExchangeStateSnapshot {
            program_id: Pubkey::new_unique().to_string(),
            global_config: Pubkey::new_unique().to_string(),
            current_authorities: AuthoritySet {
                root_authority: root_authority.to_string(),
                risk_authority: Pubkey::new_unique().to_string(),
                market_authority: Pubkey::new_unique().to_string(),
                oracle_authority: Pubkey::new_unique().to_string(),
                adl_authority: Pubkey::new_unique().to_string(),
                cancel_authority: Pubkey::new_unique().to_string(),
                backstop_authority: Pubkey::new_unique().to_string(),
            },
            canonical_mint: Pubkey::new_unique().to_string(),
            usdc_mint: Pubkey::new_unique().to_string(),
            global_vault: Pubkey::new_unique().to_string(),
            perp_asset_map: Pubkey::new_unique().to_string(),
            global_trader_index: vec![Pubkey::new_unique().to_string()],
            active_trader_buffer: vec![Pubkey::new_unique().to_string()],
            withdraw_queue: Pubkey::new_unique().to_string(),
            exchange_status_bits: 129,
            exchange_status_features: vec!["initialized".to_string(), "active".to_string()],
            active: true,
            gated: false,
            withdrawals_available: true,
        }
    }

    fn exchange_store(root_authority: Pubkey) -> SharedExchangeCacheStore {
        SharedExchangeCacheStore::new(ExchangeSnapshotView {
            version: 1,
            sequence_number: Some(10u64.into()),
            slot: 1,
            slot_index: 0,
            exchange: exchange_state_snapshot(root_authority),
            markets: Vec::new(),
            spot_collaterals: Vec::new(),
        })
    }

    fn rotate_store_root_authority(store: &SharedExchangeCacheStore, new_root_authority: Pubkey) {
        let sequence_number = store
            .snapshot()
            .expect("store should be populated")
            .sequence_number
            .expect("store should have a sequence baseline")
            .into_inner();
        store
            .apply_delta(&ExchangeDeltaMessage {
                version: 1,
                sequence_number: (sequence_number + 1).into(),
                slot: 2,
                slot_index: 0,
                ops: vec![ExchangeDeltaOp::ExchangeKeysUpdated {
                    exchange: exchange_state_snapshot(new_root_authority),
                }],
            })
            .expect("keys delta should apply");
    }

    fn permission_account(ix: &Instruction) -> Pubkey {
        ix.accounts[ix.accounts.len() - 1].pubkey
    }

    #[test]
    fn test_wraps_order_placing_ix() {
        let client = PhoenixFlightClient::new(Pubkey::new_unique(), 0, 0);
        let trader_wallet = Pubkey::new_unique();
        let inner = build_sample_limit_ix();
        let inner_data = inner.data.clone();

        let wrapped = client
            .try_wrap_order_instruction(inner, trader_wallet, false)
            .unwrap();

        // Wrapped ix targets Flight, not Phoenix.
        assert_eq!(
            wrapped.program_id,
            phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID
        );
        // Data = flight discriminant + inner data.
        assert_eq!(
            &wrapped.data[..8],
            &phoenix_rise_ix::FlightInstruction::ProxyInstruction.discriminant()
        );
        assert_eq!(&wrapped.data[8..], &inner_data[..]);
    }

    #[test]
    fn test_wraps_order_placing_ix_with_fee_bps_override() {
        let client = PhoenixFlightClient::new(Pubkey::new_unique(), 0, 0);
        let trader_wallet = Pubkey::new_unique();
        let inner = build_sample_limit_ix();
        let inner_data = inner.data.clone();

        let wrapped = client
            .try_wrap_order_instruction_with_fee_bps_override(inner, trader_wallet, false, Some(5))
            .unwrap();

        assert_eq!(
            wrapped.program_id,
            phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID
        );
        assert_eq!(
            &wrapped.data[..8],
            &phoenix_rise_ix::FlightInstruction::ProxyInstructionWithFeeOverride.discriminant()
        );
        assert_eq!(wrapped.data[8], 1);
        assert_eq!(&wrapped.data[9..17], &5u64.to_le_bytes());
        assert_eq!(&wrapped.data[17..], &inner_data[..]);
    }

    #[test]
    fn test_client_level_fee_bps_override_applies_and_per_call_arg_wins() {
        let client = PhoenixFlightClient::new(Pubkey::new_unique(), 0, 0).with_fee_bps_override(7);
        let trader_wallet = Pubkey::new_unique();
        let inner = build_sample_limit_ix();

        // The client-level configuration applies to plain wraps.
        let wrapped = client
            .try_wrap_order_instruction(inner.clone(), trader_wallet, false)
            .unwrap();
        assert_eq!(
            &wrapped.data[..8],
            &phoenix_rise_ix::FlightInstruction::ProxyInstructionWithFeeOverride.discriminant()
        );
        assert_eq!(&wrapped.data[9..17], &7u64.to_le_bytes());

        // The per-call argument wins over the client-level configuration...
        let wrapped = client
            .try_wrap_order_instruction_with_fee_bps_override(
                inner.clone(),
                trader_wallet,
                false,
                Some(3),
            )
            .unwrap();
        assert_eq!(&wrapped.data[9..17], &3u64.to_le_bytes());

        // ...including `None`, which wraps this call without an override.
        let wrapped = client
            .try_wrap_order_instruction_with_fee_bps_override(inner, trader_wallet, false, None)
            .unwrap();
        assert_eq!(
            &wrapped.data[..8],
            &phoenix_rise_ix::FlightInstruction::ProxyInstruction.discriminant()
        );
    }

    #[test]
    fn test_non_order_ix_passthrough() {
        let client = PhoenixFlightClient::new(Pubkey::new_unique(), 0, 0);
        let trader_wallet = Pubkey::new_unique();

        let unrelated = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![SolAccountMeta::new_readonly(Pubkey::new_unique(), false)],
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };

        let result = client
            .try_wrap_order_instruction(unrelated.clone(), trader_wallet, false)
            .unwrap();

        assert_eq!(result.program_id, unrelated.program_id);
        assert_eq!(result.data, unrelated.data);
        assert_eq!(result.accounts.len(), unrelated.accounts.len());
    }

    #[test]
    fn test_plain_wrap_of_delegated_market_order_appends_no_tail() {
        let root_authority = Pubkey::new_unique();
        let client = PhoenixFlightClient::from_exchange_store(
            Pubkey::new_unique(),
            0,
            0,
            exchange_store(root_authority),
        );
        let trader_wallet = Pubkey::new_unique();
        let inner =
            build_sample_phoenix_ix(PhoenixInstruction::PlaceMarketOrderDelegated.discriminant());
        let inner_accounts = inner.accounts.len();

        let wrapped = client
            .try_wrap_order_instruction_with_fee_bps_override(inner, trader_wallet, false, None)
            .unwrap();

        assert_eq!(
            wrapped.program_id,
            phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID
        );
        // Owner-signed delegated market orders need no collateral-transfer
        // tail; only the position-authority wrap appends it.
        assert_eq!(wrapped.accounts.len(), 6 + inner_accounts);
        assert!(!wrapped.accounts.iter().any(|meta| {
            meta.pubkey
                == phoenix_rise_ix::flight::get_flight_collateral_transfer_authority_address()
                    .unwrap()
        }));
    }

    #[test]
    fn test_owner_signed_wrap_appends_no_tail() {
        let client = PhoenixFlightClient::from_exchange_store(
            Pubkey::new_unique(),
            0,
            0,
            exchange_store(Pubkey::new_unique()),
        );
        let owner = Pubkey::new_unique();
        let inner = build_sample_phoenix_ix(PhoenixInstruction::PlaceMarketOrder.discriminant());
        let inner_accounts = inner.accounts.len();

        let wrapped = client
            .try_wrap_order_instruction(inner, owner, false)
            .unwrap();

        assert_eq!(wrapped.accounts.len(), 6 + inner_accounts);
        assert_eq!(wrapped.accounts[5].pubkey, owner);
        assert!(!wrapped.accounts.iter().any(|meta| {
            meta.pubkey
                == phoenix_rise_ix::flight::get_flight_collateral_transfer_authority_address()
                    .unwrap()
        }));
    }

    #[test]
    fn test_position_authority_wrap_appends_tail_and_places_delegate_signer() {
        let root_authority = Pubkey::new_unique();
        let client = PhoenixFlightClient::from_exchange_store(
            Pubkey::new_unique(),
            0,
            0,
            exchange_store(root_authority),
        );
        let delegate = Pubkey::new_unique();
        let inner = build_sample_phoenix_ix(PhoenixInstruction::PlaceMarketOrder.discriminant());
        let inner_accounts = inner.accounts.len();

        let wrapped = client
            .try_wrap_order_instruction(inner, delegate, true)
            .unwrap();

        assert_eq!(wrapped.accounts.len(), 6 + inner_accounts + 2);
        assert_eq!(wrapped.accounts[5].pubkey, delegate);
        assert_eq!(
            wrapped.accounts[wrapped.accounts.len() - 2].pubkey,
            phoenix_rise_ix::flight::get_flight_collateral_transfer_authority_address().unwrap()
        );
        assert_eq!(
            wrapped.accounts[wrapped.accounts.len() - 1].pubkey,
            phoenix_rise_ix::flight::get_flight_authorized_collateral_transfer_permission_address(
                &root_authority
            )
            .unwrap()
        );
    }

    #[test]
    fn test_position_authority_wrap_requires_root_authority() {
        let client = PhoenixFlightClient::new(Pubkey::new_unique(), 0, 0);
        let inner = build_sample_phoenix_ix(PhoenixInstruction::PlaceMarketOrder.discriminant());

        let result = client.try_wrap_order_instruction(inner, Pubkey::new_unique(), true);

        assert!(matches!(result, Err(PhoenixIxError::MissingRootAuthority)));
    }

    #[test]
    fn test_position_authority_wrap_with_unpopulated_store_is_missing_root_authority() {
        let client = PhoenixFlightClient::from_exchange_store(
            Pubkey::new_unique(),
            0,
            0,
            SharedExchangeCacheStore::new_empty(),
        );
        let inner = build_sample_phoenix_ix(PhoenixInstruction::PlaceMarketOrder.discriminant());

        let result = client.try_wrap_order_instruction(inner, Pubkey::new_unique(), true);

        assert!(matches!(result, Err(PhoenixIxError::MissingRootAuthority)));
    }

    #[test]
    fn test_store_sourced_wrap_derives_permission_from_live_root_authority() {
        let initial_root = Pubkey::new_unique();
        let rotated_root = Pubkey::new_unique();
        let store = exchange_store(initial_root);
        let client =
            PhoenixFlightClient::from_exchange_store(Pubkey::new_unique(), 0, 0, store.clone());
        let position_authority = Pubkey::new_unique();
        let inner = build_sample_phoenix_ix(PhoenixInstruction::PlaceMarketOrder.discriminant());

        let wrapped_before = client
            .try_wrap_order_instruction(inner.clone(), position_authority, true)
            .unwrap();
        assert_eq!(
            permission_account(&wrapped_before),
            phoenix_rise_ix::flight::get_flight_authorized_collateral_transfer_permission_address(
                &initial_root
            )
            .unwrap()
        );

        // Rotate the root authority on-chain (observed through the store) and
        // wrap again: the client must derive the NEW permission PDA.
        rotate_store_root_authority(&store, rotated_root);

        let wrapped_after = client
            .try_wrap_order_instruction(inner, position_authority, true)
            .unwrap();
        assert_eq!(
            permission_account(&wrapped_after),
            phoenix_rise_ix::flight::get_flight_authorized_collateral_transfer_permission_address(
                &rotated_root
            )
            .unwrap()
        );
        assert_ne!(
            permission_account(&wrapped_before),
            permission_account(&wrapped_after)
        );
    }

    #[test]
    fn test_store_sourced_position_authority_wrap_tracks_rotation() {
        let initial_root = Pubkey::new_unique();
        let rotated_root = Pubkey::new_unique();
        let store = exchange_store(initial_root);
        let client =
            PhoenixFlightClient::from_exchange_store(Pubkey::new_unique(), 0, 0, store.clone());
        let position_authority = Pubkey::new_unique();
        let inner = build_sample_phoenix_ix(PhoenixInstruction::PlaceMarketOrder.discriminant());

        rotate_store_root_authority(&store, rotated_root);

        let wrapped = client
            .try_wrap_order_instruction(inner, position_authority, true)
            .unwrap();
        assert_eq!(
            permission_account(&wrapped),
            phoenix_rise_ix::flight::get_flight_authorized_collateral_transfer_permission_address(
                &rotated_root
            )
            .unwrap()
        );
    }

    #[test]
    fn test_store_sourced_wrap_with_invalid_root_authority_is_missing_field() {
        let store = exchange_store(Pubkey::new_unique());
        let mut snapshot = store.snapshot().expect("store should be populated");
        snapshot.exchange.current_authorities.root_authority = "not-a-pubkey".to_string();
        store.apply_snapshot(
            snapshot,
            crate::exchange_cache::ExchangeCacheSnapshotSource::Websocket,
        );
        let client = PhoenixFlightClient::from_exchange_store(Pubkey::new_unique(), 0, 0, store);
        let inner = build_sample_phoenix_ix(PhoenixInstruction::PlaceMarketOrder.discriminant());

        let result = client.try_wrap_order_instruction(inner, Pubkey::new_unique(), true);

        assert!(matches!(result, Err(PhoenixIxError::MissingRootAuthority)));
    }

    #[test]
    fn test_is_flight_routable_instruction() {
        assert!(is_flight_routable_instruction(&build_sample_limit_ix()));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceMarketOrder.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceMarketOrderDelegated.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceStopLoss.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlacePositionConditionalOrder.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceAttachedConditionalOrder.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceLimitOrderWithConditionals.discriminant()
        )));

        let fake = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![0u8; 8],
        };
        assert!(!is_flight_routable_instruction(&fake));
    }
}
