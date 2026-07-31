use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use base64::Engine;
use phoenix_rise::accounts::PhoenixAccount;
use phoenix_rise::accounts::conditional_orders::{ConditionalOrderHeader, ConditionalOrders};
use phoenix_rise::accounts::escrow::{Escrow, EscrowActionKind, EscrowHeader};
use phoenix_rise::accounts::global_config::GlobalConfig;
use phoenix_rise::accounts::multi_arena::MultiArenaHeader;
use phoenix_rise::accounts::perp_asset_map::PerpAssetMap;
use phoenix_rise::accounts::stop_losses::{
    StopLossTriggerOrderKind, StopLosses, TradeSide, TriggerDirection,
};
use phoenix_rise::accounts::trader::capabilities::{
    REQUIRED_TRADER_CAPABILITIES, TRADER_CAPABILITY_CAN_WITHDRAW, is_trader_ready,
};
use phoenix_rise::accounts::trader::{Trader, TraderHeader, TraderPositions};
use phoenix_rise::accounts::withdraw_queue::{
    WithdrawQueue, WithdrawRequestState, WithdrawTransitionReason,
};
use phoenix_rise::ix::PhoenixInstruction;
use phoenix_rise::ix::constants::compute_discriminant;
use serde::Deserialize;
use solana_pubkey::Pubkey;

#[derive(Debug, Deserialize)]
struct FixtureFile {
    account: FixtureAccount,
}

#[derive(Debug, Deserialize)]
struct FixtureAccount {
    data: [String; 2],
}

fn mock_bytes(file_name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ts/tests/mocks")
        .join(file_name);
    let fixture: FixtureFile =
        serde_json::from_str(&fs::read_to_string(path).expect("fixture should be readable"))
            .expect("fixture should parse");
    base64::engine::general_purpose::STANDARD
        .decode(&fixture.account.data[0])
        .expect("fixture should contain base64 account bytes")
}

#[test]
fn borrowed_trader_reads_header_flags_and_position_capacity() {
    let data = mock_bytes("trader.json");
    let header = TraderHeader::try_from_account_bytes(&data).expect("header should decode");

    assert_eq!(header.flags(), 62);
    assert!(header.is_ready());
    assert!(header.is_cold());
    assert!(!header.is_hot());
    assert!(!header.is_reduce_only());
    assert!(!header.is_frozen());
    assert_eq!(header.max_positions(), 128);
    assert_eq!(header.trader_preference_bits(), 0);
    assert!(!header.disable_collateral_sweep());
    assert_eq!(header.trader_preference_flags().to_string(), "NONE");

    let trader = Trader::try_from_account_bytes(&data).expect("trader should decode");
    assert_eq!(trader.raw_len(), 1);
    assert_eq!(trader.capacity(), 128);
    assert!(trader.has_position_capacity());
}

#[test]
fn borrowed_trader_positions_iterate_without_allocating() {
    let data = mock_bytes("trader.json");
    let positions =
        TraderPositions::try_from_account_bytes(&data).expect("positions should decode");
    let entries = positions.iter().collect::<Vec<_>>();

    assert_eq!(positions.len(), 1);
    assert_eq!(positions.capacity(), 128);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].asset_id, 11);
    assert_eq!(entries[0].position.base_lot_position.as_inner(), 312);
    assert_eq!(
        entries[0].position.virtual_quote_lot_position.as_inner(),
        -100_027_200
    );
    assert_eq!(
        entries[0].position.cumulative_funding_snapshot.as_inner(),
        -7
    );
    assert_eq!(entries[0].position.position_sequence_number, 139);
    assert_eq!(
        entries[0]
            .position
            .accumulated_funding_for_active_position
            .as_inner(),
        0
    );
}

#[test]
fn borrowed_global_config_reads_dynamic_account_headers() {
    let data = mock_bytes("global_config.json");
    let config = GlobalConfig::try_from_account_bytes(&data).expect("global config should decode");

    assert_eq!(
        Pubkey::new_from_array(config.perp_asset_map_key()).to_string(),
        "2nHGAaEw3D5dd4hVueaUNoygkQFmoeKqRQWnSPqSMFUC"
    );
    assert_eq!(
        Pubkey::new_from_array(config.global_trader_index_header_key()).to_string(),
        "HCrPXLByGqRh2szQi3gj7oRdRVBNi1gccAyn4CQCT3HK"
    );
    assert_eq!(
        Pubkey::new_from_array(config.active_trader_buffer_header_key()).to_string(),
        "2U32rSzzrQS3eVmGHsnbw5kcqKF3wQXpHGd3hMq5YJok"
    );
}

#[test]
fn borrowed_multi_arena_header_reads_superblock_count() {
    let mut data = vec![0_u8; 80];
    data[..8].copy_from_slice(&PhoenixAccount::GlobalTraderIndexHeader.discriminant());
    data[8..16].copy_from_slice(&42_u64.to_le_bytes());
    data[16..24].copy_from_slice(&99_u64.to_le_bytes());
    data[24..28].copy_from_slice(&7_u32.to_le_bytes());
    data[48..52].copy_from_slice(&128_u32.to_le_bytes());
    data[52..54].copy_from_slice(&3_u16.to_le_bytes());
    data[54..56].copy_from_slice(&2_u16.to_le_bytes());
    data[56..60].copy_from_slice(&64_u32.to_le_bytes());

    let header = MultiArenaHeader::try_from_account_bytes(
        "GlobalTraderIndex",
        &data,
        PhoenixAccount::GlobalTraderIndexHeader.discriminant(),
    )
    .expect("multi-arena header should decode");

    assert_eq!(header.sequence_number().sequence_number, 42);
    assert_eq!(header.sequence_number().last_update_slot, 99);
    assert_eq!(header.num_additional_nodes(), 7);
    assert_eq!(header.superblock().size(), 128);
    assert_eq!(header.num_arenas(), 3);
    assert_eq!(header.account_count(), 3);
    assert_eq!(header.superblock().num_active_arenas(), 2);
    assert_eq!(header.superblock().num_nodes_per_arena(), 64);
}

#[test]
fn borrowed_conditional_order_account_reads_dynamic_capacity() {
    let trader = Pubkey::new_unique();
    let funding = Pubkey::new_unique();
    let capacity = 3_u8;
    let mut data = vec![0_u8; ConditionalOrderHeader::required_account_size(capacity)];
    data[..8].copy_from_slice(&PhoenixAccount::ConditionalOrderHeader.discriminant());
    data[8..40].copy_from_slice(trader.as_ref());
    data[40..72].copy_from_slice(funding.as_ref());
    data[72..80].copy_from_slice(&10_u64.to_le_bytes());
    data[80..88].copy_from_slice(&20_u64.to_le_bytes());
    data[88] = 1;
    data[89] = capacity;

    let order_offset = 96;
    data[order_offset..order_offset + 8].copy_from_slice(&42_u64.to_le_bytes());
    data[order_offset + 8..order_offset + 16].copy_from_slice(&100_u64.to_le_bytes());
    data[order_offset + 16..order_offset + 24].copy_from_slice(&101_u64.to_le_bytes());
    data[order_offset + 24..order_offset + 32].copy_from_slice(&1_000_u64.to_le_bytes());
    data[order_offset + 32..order_offset + 40].copy_from_slice(&750_u64.to_le_bytes());
    data[order_offset + 40..order_offset + 48].copy_from_slice(&250_u64.to_le_bytes());
    data[order_offset + 48..order_offset + 56].copy_from_slice(&99_u64.to_le_bytes());
    data[order_offset + 56..order_offset + 60].copy_from_slice(&7_u32.to_le_bytes());
    data[order_offset + 60] = 1;
    data[order_offset + 61] = 25;
    data[order_offset + 64..order_offset + 72].copy_from_slice(&20_000_u64.to_le_bytes());
    data[order_offset + 72..order_offset + 80].copy_from_slice(&19_900_u64.to_le_bytes());
    data[order_offset + 80] = 9;
    data[order_offset + 81] = 1 | (1 << 1) | (1 << 2) | (1 << 3);

    let header =
        ConditionalOrderHeader::try_from_account_bytes(&data).expect("header should decode");
    assert_eq!(header.trader_key(), trader.to_bytes());
    assert_eq!(header.funding_key(), funding.to_bytes());
    assert_eq!(header.sequence_number().sequence_number, 10);
    assert_eq!(header.sequence_number().last_update_slot, 20);
    assert_eq!(header.len(), 1);
    assert_eq!(header.capacity(), capacity);
    assert!(header.can_fit_raw_index(1));
    assert!(!header.can_fit_raw_index(capacity));

    let orders = ConditionalOrders::try_from_account_bytes(&data).expect("orders should decode");
    let decoded_orders = orders
        .orders()
        .collect::<Result<Vec<_>, _>>()
        .expect("orders should decode");
    let order = &decoded_orders[0];
    assert_eq!(decoded_orders.len(), capacity as usize);
    assert_eq!(order.sequence_number(), 42);
    assert_eq!(order.order_id().price_in_ticks(), 100);
    assert_eq!(order.order_id().order_sequence_number(), 101);
    assert_eq!(order.max_size(), 1_000);
    assert_eq!(order.fillable_size(), 750);
    assert_eq!(order.filled_size(), 250);
    assert_eq!(order.slot(), 99);
    assert_eq!(order.asset_id(), 7);
    assert!(order.flags().uses_percent());
    assert_eq!(order.percent(), 25);
    assert!(order.is_active());
    assert_eq!(order.greater_trigger_order().trigger_price(), 20_000);
    assert_eq!(order.greater_trigger_order().execution_price(), 19_900);
    assert_eq!(order.greater_trigger_order().position_sequence_number(), 9);
    assert!(
        order
            .greater_trigger_order()
            .flags()
            .is_greater_than_trigger()
    );
    assert!(order.greater_trigger_order().flags().is_bid());
    assert!(order.greater_trigger_order().flags().is_limit_order());
}

#[test]
fn borrowed_stop_losses_account_reads_fixed_slots() {
    let trader = Pubkey::new_unique();
    let funding = Pubkey::new_unique();
    let mut data = vec![0_u8; 328];
    data[..8].copy_from_slice(&PhoenixAccount::StopLosses.discriminant());

    let stop_loss_offset = 8;
    data[stop_loss_offset..stop_loss_offset + 8].copy_from_slice(&11_u64.to_le_bytes());
    data[stop_loss_offset + 8..stop_loss_offset + 16].copy_from_slice(&25_000_u64.to_le_bytes());
    data[stop_loss_offset + 16..stop_loss_offset + 24].copy_from_slice(&24_900_u64.to_le_bytes());
    data[stop_loss_offset + 24..stop_loss_offset + 32].copy_from_slice(&5_u64.to_le_bytes());
    data[stop_loss_offset + 32..stop_loss_offset + 40].copy_from_slice(&77_u64.to_le_bytes());
    data[stop_loss_offset + 40] = 3;
    data[stop_loss_offset + 41] = 1 | (1 << 1) | (1 << 2) | (1 << 3);

    data[168..176].copy_from_slice(&1_u64.to_le_bytes());
    data[176..184].copy_from_slice(&12_u64.to_le_bytes());
    data[184..192].copy_from_slice(&13_u64.to_le_bytes());
    data[192..224].copy_from_slice(funding.as_ref());
    data[224..256].copy_from_slice(trader.as_ref());
    data[256..260].copy_from_slice(&8_u32.to_le_bytes());

    let stop_losses = StopLosses::try_from_account_bytes(&data).expect("stop losses should decode");
    let greater = stop_losses.greater_than_stop_loss();
    assert!(stop_losses.is_initialized());
    assert_eq!(stop_losses.sequence_number().sequence_number, 12);
    assert_eq!(stop_losses.sequence_number().last_update_slot, 13);
    assert_eq!(stop_losses.funding_key(), funding.to_bytes());
    assert_eq!(stop_losses.trader_key(), trader.to_bytes());
    assert_eq!(stop_losses.asset_id(), 8);
    assert!(stop_losses.validate(&trader.to_bytes(), Some(&funding.to_bytes()), Some(8)));
    assert_eq!(greater.sequence_number(), 11);
    assert_eq!(greater.trigger_price(), 25_000);
    assert_eq!(greater.execution_price(), 24_900);
    assert_eq!(greater.trade_size(), 5);
    assert_eq!(greater.slot(), 77);
    assert_eq!(greater.position_sequence_number(), 3);
    assert!(greater.is_active());
    assert_eq!(greater.trigger_direction(), TriggerDirection::GreaterThan);
    assert_eq!(greater.trade_side(), TradeSide::Bid);
    assert_eq!(greater.order_kind(), StopLossTriggerOrderKind::Limit);
}

#[test]
fn borrowed_escrow_account_reads_requests_and_actions() {
    let authority = Pubkey::new_unique();
    let funder = Pubkey::new_unique();
    let sender = Pubkey::new_unique();
    let capacity = 2_u64;
    let mut data = vec![0_u8; EscrowHeader::required_account_size(capacity).unwrap()];
    data[..8].copy_from_slice(&PhoenixAccount::EscrowHeader.discriminant());
    data[8..40].copy_from_slice(authority.as_ref());
    data[40..72].copy_from_slice(funder.as_ref());
    data[72..80].copy_from_slice(&3_u64.to_le_bytes());
    data[80..88].copy_from_slice(&4_u64.to_le_bytes());
    data[88..96].copy_from_slice(&1_u64.to_le_bytes());
    data[96..104].copy_from_slice(&capacity.to_le_bytes());

    let request_offset = 104;
    data[request_offset..request_offset + 32].copy_from_slice(sender.as_ref());
    data[request_offset + 32] = 0;
    data[request_offset + 33] = 0;
    data[request_offset + 34] = 1;
    data[request_offset + 35] = 2;
    data[request_offset + 36..request_offset + 40].copy_from_slice(&50_u32.to_le_bytes());
    data[request_offset + 40..request_offset + 48].copy_from_slice(&55_u64.to_le_bytes());
    data[request_offset + 48..request_offset + 56].copy_from_slice(&100_u64.to_le_bytes());

    let action_offset = request_offset + 192;
    data[action_offset] = 1;
    data[action_offset + 8..action_offset + 16].copy_from_slice(&1_500_u64.to_le_bytes());

    let escrow = Escrow::try_from_account_bytes(&data).expect("escrow should decode");
    let header = escrow.header();
    assert_eq!(header.authority(), authority.to_bytes());
    assert_eq!(header.funder_key(), funder.to_bytes());
    assert_eq!(header.sequence_number().sequence_number, 3);
    assert_eq!(header.sequence_number().last_update_slot, 4);
    assert_eq!(header.len(), 1);
    assert_eq!(header.capacity(), capacity);
    assert_eq!(escrow.requests().len(), capacity as usize);

    let request = escrow
        .find_request(55)
        .expect("request lookup should decode")
        .expect("request should exist");
    assert_eq!(request.sender_authority(), sender.to_bytes());
    assert_eq!(request.sender_pda_index(), 0);
    assert_eq!(request.sender_subaccount_index(), 0);
    assert_eq!(request.receiver_pda_index(), 1);
    assert_eq!(request.receiver_subaccount_index(), 2);
    assert_eq!(request.expiration_offset(), Some(50));
    assert_eq!(request.sequence_number(), Some(55));
    assert_eq!(request.initial_slot(), 100);
    assert!(request.is_active(150));
    assert!(request.is_expired(151));
    assert_eq!(request.actions()[0].kind(), EscrowActionKind::Cash);
    assert_eq!(request.actions()[0].cash_amount(), Some(1_500));
}

#[test]
fn borrowed_withdraw_queue_account_reads_header_and_iterates_nodes() {
    let trader = Pubkey::new_unique();
    let wallet = Pubkey::new_unique();
    let mut data = vec![0_u8; 213_136];
    data[..8].copy_from_slice(&PhoenixAccount::WithdrawQueueHeader.discriminant());
    data[8..16].copy_from_slice(&1_u64.to_le_bytes());
    data[16..24].copy_from_slice(&2_u64.to_le_bytes());
    data[24..32].copy_from_slice(&10_000_u64.to_le_bytes());
    data[32..40].copy_from_slice(&9_000_u64.to_le_bytes());
    data[40..48].copy_from_slice(&100_u64.to_le_bytes());
    data[48..56].copy_from_slice(&7_u64.to_le_bytes());
    data[56..64].copy_from_slice(&1_234_u64.to_le_bytes());
    data[64..72].copy_from_slice(&5_u64.to_le_bytes());
    data[72..80].copy_from_slice(&6_u64.to_le_bytes());

    data[120..124].copy_from_slice(&1_u32.to_le_bytes());
    data[124..128].copy_from_slice(&1_u32.to_le_bytes());
    data[128..136].copy_from_slice(&1_u64.to_le_bytes());
    data[136..140].copy_from_slice(&2_u32.to_le_bytes());

    let node_offset = 144;
    data[node_offset..node_offset + 4].copy_from_slice(&0_u32.to_le_bytes());
    data[node_offset + 4..node_offset + 8].copy_from_slice(&0_u32.to_le_bytes());
    let request_offset = node_offset + 8;
    data[request_offset..request_offset + 32].copy_from_slice(trader.as_ref());
    data[request_offset + 32..request_offset + 64].copy_from_slice(wallet.as_ref());
    data[request_offset + 64..request_offset + 72].copy_from_slice(&1_234_u64.to_le_bytes());
    data[request_offset + 72..request_offset + 80].copy_from_slice(&10_u64.to_le_bytes());
    data[request_offset + 80..request_offset + 88].copy_from_slice(&11_u64.to_le_bytes());
    data[request_offset + 88..request_offset + 90].copy_from_slice(&2_u16.to_le_bytes());
    data[request_offset + 90] = WithdrawRequestState::Queued as u8;
    data[request_offset + 91] = WithdrawTransitionReason::QueueOrdering as u8;

    let queue = WithdrawQueue::try_from_account_bytes(&data).expect("queue should decode");
    assert_eq!(queue.header().sequence_number().sequence_number, 1);
    assert_eq!(queue.header().sequence_number().last_update_slot, 2);
    assert_eq!(queue.header().withdraw_throttle().max_budget(), 10_000);
    assert_eq!(queue.header().withdraw_throttle().remaining_budget(), 9_000);
    assert_eq!(queue.header().withdraw_throttle().last_update_slot(), 7);
    assert_eq!(queue.header().total_queued_amount(), 1_234);
    assert_eq!(queue.header().withdrawal_fee(), 5);
    assert_eq!(queue.header().enqueueing_fee(), 6);
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.head_node_index(), Some(1));
    assert_eq!(queue.tail_node_index(), Some(1));
    assert_eq!(queue.bump_index(), 2);

    let entries = queue
        .iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("queue entries should decode");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].node_index, 1);
    let request = entries[0].request;
    assert_eq!(request.trader_key(), trader.to_bytes());
    assert_eq!(request.wallet_key(), wallet.to_bytes());
    assert_eq!(request.amount(), 1_234);
    assert_eq!(request.submission_slot(), 10);
    assert_eq!(request.last_update_slot(), 11);
    assert_eq!(request.transition_count(), 2);
    assert_eq!(request.state(), Some(WithdrawRequestState::Queued));
    assert_eq!(
        request.last_transition_reason(),
        Some(WithdrawTransitionReason::QueueOrdering)
    );
    assert!(request.is_active());
    assert!(!request.is_terminal());
}

#[test]
fn borrowed_perp_asset_map_iterates_active_metadata() {
    let data = mock_bytes("perp_asset_map.json");
    let map = PerpAssetMap::try_from_account_bytes(&data).expect("perp asset map should decode");
    let entries = map
        .iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("entries should decode");

    assert_eq!(map.num_assets(), 12);
    assert_eq!(entries.len(), 12);

    let sol = map
        .find_by_symbol("SOL")
        .expect("symbol lookup should decode")
        .expect("SOL should be present");
    assert_eq!(sol.symbol.as_str(), "SOL");
    assert_eq!(sol.metadata.static_market_params().base_lot_decimals, 2);
    assert_eq!(sol.metadata.asset_flags().bits, 0);
    assert!(!sol.metadata.asset_flags().is_commodity);
    assert!(!sol.metadata.asset_flags().is_commodities_reopen);
    assert!(!sol.metadata.asset_flags().is_commodities_after_hours);
}

#[test]
fn trader_ready_helper_requires_market_deposit_and_withdraw() {
    assert!(is_trader_ready(REQUIRED_TRADER_CAPABILITIES));
    assert!(!is_trader_ready(
        REQUIRED_TRADER_CAPABILITIES & !TRADER_CAPABILITY_CAN_WITHDRAW
    ));
}

#[test]
fn phoenix_account_discriminants_round_trip() {
    let mut seen = HashSet::new();

    for account in PhoenixAccount::ALL {
        assert!(
            seen.insert(account.tag()),
            "duplicate account tag for {account}"
        );
        assert_eq!(
            PhoenixAccount::from_discriminant(account.discriminant()),
            Some(account)
        );
        assert_eq!(PhoenixAccount::from_tag(account.tag()), Some(account));
        assert_eq!(
            PhoenixAccount::from_account_name(account.account_name()),
            Some(account)
        );
        assert_eq!(
            PhoenixAccount::from_snake_case_account_name(account.snake_case_account_name()),
            Some(account)
        );
    }

    assert_eq!(PhoenixAccount::Orderbook, PhoenixAccount::OrderbookHeader);
    assert_eq!(
        PhoenixAccount::ConditionalOrderCollection,
        PhoenixAccount::ConditionalOrderHeader
    );
}

#[test]
fn phoenix_instruction_discriminants_round_trip() {
    let mut seen = HashSet::new();

    for instruction in PhoenixInstruction::ALL {
        assert!(
            seen.insert(instruction.tag()),
            "duplicate instruction tag for {instruction}"
        );
        assert_eq!(
            PhoenixInstruction::from_discriminant(instruction.discriminant()),
            Some(instruction)
        );
        assert_eq!(
            PhoenixInstruction::from_instruction_name(instruction.instruction_name()),
            Some(instruction)
        );
        assert_eq!(
            PhoenixInstruction::from_snake_case_instruction_name(
                instruction.snake_case_instruction_name()
            ),
            Some(instruction)
        );
    }

    assert_eq!(PhoenixInstruction::ALL.len(), 96);
    assert_eq!(
        PhoenixInstruction::SetMultiArenaAdditionalNodesWatermark.discriminant(),
        compute_discriminant("global:set_multi_arena_additional_nodes_watermark")
    );
    assert_eq!(
        PhoenixInstruction::SetMultiArenaNumNodesPerArena.discriminant(),
        compute_discriminant("global:set_multi_arena_num_nodes_per_arena")
    );
    assert_eq!(
        PhoenixInstruction::from_instruction_name("UpdateSplinePriceWithOrdering"),
        Some(PhoenixInstruction::UpdateSplinePrice)
    );
}
