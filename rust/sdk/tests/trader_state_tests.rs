//! Tests for the Trader state container.

use phoenix_rise::types::{
    CapabilityAccess, CooldownStatus, TraderCapabilitiesView, TraderStateCapabilities,
    TraderStateDelta, TraderStatePayload, TraderStatePositionDelta, TraderStatePositionRow,
    TraderStatePositionSnapshot, TraderStateRowChangeKind, TraderStateServerMessage,
    TraderStateSnapshot, TraderStateSubaccountDelta, TraderStateSubaccountSnapshot,
};
use phoenix_rise::{Trader, TraderKey};
use rust_decimal::Decimal;
use solana_pubkey::Pubkey;

fn make_capabilities() -> TraderStateCapabilities {
    TraderStateCapabilities {
        flags: 63,
        state: "active".to_string(),
        capabilities: TraderCapabilitiesView {
            place_limit_order: CapabilityAccess {
                immediate: true,
                via_cold_activation: true,
            },
            place_market_order: CapabilityAccess {
                immediate: true,
                via_cold_activation: true,
            },
            risk_increasing_trade: CapabilityAccess {
                immediate: true,
                via_cold_activation: true,
            },
            risk_reducing_trade: CapabilityAccess {
                immediate: true,
                via_cold_activation: true,
            },
            deposit_collateral: CapabilityAccess {
                immediate: true,
                via_cold_activation: true,
            },
            withdraw_collateral: CapabilityAccess {
                immediate: true,
                via_cold_activation: true,
            },
        },
    }
}

fn make_position_row(base_lots: i64, entry_price: &str) -> TraderStatePositionRow {
    TraderStatePositionRow {
        position_sequence_number: "1".to_string(),
        base_position_lots: base_lots.to_string(),
        entry_price_ticks: "1000".to_string(),
        entry_price_usd: entry_price.to_string(),
        virtual_quote_position_lots: "0".to_string(),
        unsettled_funding_quote_lots: "0".to_string(),
        accumulated_funding_quote_lots: "0".to_string(),
        take_profit_triggers: vec![],
        stop_loss_triggers: vec![],
    }
}

#[test]
fn test_apply_snapshot() {
    let key = TraderKey::new(Pubkey::new_unique());
    let mut trader = Trader::new(key.clone());

    // Create a snapshot message
    let snapshot = TraderStateSnapshot {
        version: 1,
        capabilities: make_capabilities(),
        maker_fee_override_multiplier: 0.9,
        taker_fee_override_multiplier: 1.1,
        subaccounts: vec![TraderStateSubaccountSnapshot {
            subaccount_index: 0,
            sequence: 100,
            collateral: "1000".to_string(),
            capabilities: Some(make_capabilities()),
            cooldown_status: None,
            positions: vec![TraderStatePositionSnapshot {
                symbol: "SOL".to_string(),
                position: make_position_row(100, "150.25"),
            }],
            orders: vec![],
            splines: vec![],
        }],
    };

    let msg = TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12345,
        content: TraderStatePayload::Snapshot(snapshot),
    };

    // Apply the snapshot
    trader.apply_update(&msg);

    // Verify state
    assert_eq!(trader.last_slot, 12345);
    assert_eq!(trader.maker_fee_override_multiplier, 0.9);
    assert_eq!(trader.taker_fee_override_multiplier, 1.1);

    let collateral = trader.total_collateral();
    assert_eq!(collateral, 1000);

    let positions = trader.all_positions();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].symbol, "SOL");
    assert_eq!(positions[0].base_position_lots, 100);

    let subaccount = trader.primary_subaccount().unwrap();
    assert_eq!(subaccount.sequence, 100);
}

#[test]
fn test_apply_delta_updates_position() {
    let key = TraderKey::new(Pubkey::new_unique());
    let mut trader = Trader::new(key.clone());

    // First apply a snapshot
    let snapshot = TraderStateSnapshot {
        version: 1,
        capabilities: make_capabilities(),
        maker_fee_override_multiplier: 1.0,
        taker_fee_override_multiplier: 1.0,
        subaccounts: vec![TraderStateSubaccountSnapshot {
            subaccount_index: 0,
            sequence: 100,
            collateral: "1000".to_string(),
            capabilities: Some(make_capabilities()),
            cooldown_status: None,
            positions: vec![TraderStatePositionSnapshot {
                symbol: "SOL".to_string(),
                position: make_position_row(100, "150.00"),
            }],
            orders: vec![],
            splines: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12345,
        content: TraderStatePayload::Snapshot(snapshot),
    });

    // Now apply a delta that updates the position
    let delta = TraderStateDelta {
        deltas: vec![TraderStateSubaccountDelta {
            subaccount_index: 0,
            sequence: 101,
            collateral: "1050".to_string(),
            capabilities: None,
            cooldown_status: None,
            positions: vec![TraderStatePositionDelta {
                symbol: "SOL".to_string(),
                change: TraderStateRowChangeKind::Updated,
                position: Some(make_position_row(200, "155.00")),
            }],
            orders: vec![],
            splines: vec![],
            trade_history: vec![],
            order_history: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12346,
        content: TraderStatePayload::Delta(delta),
    });

    // Verify updates
    let subaccount = trader.primary_subaccount().unwrap();
    assert_eq!(subaccount.sequence, 101);
    assert_eq!(subaccount.collateral, 1050);

    let positions = trader.all_positions();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].base_position_lots, 200);
    assert_eq!(positions[0].entry_price_usd, Decimal::new(15500, 2)); // 155.00
}

#[test]
fn test_apply_delta_closes_position() {
    let key = TraderKey::new(Pubkey::new_unique());
    let mut trader = Trader::new(key.clone());

    // First apply a snapshot with a position
    let snapshot = TraderStateSnapshot {
        version: 1,
        capabilities: make_capabilities(),
        maker_fee_override_multiplier: 1.0,
        taker_fee_override_multiplier: 1.0,
        subaccounts: vec![TraderStateSubaccountSnapshot {
            subaccount_index: 0,
            sequence: 100,
            collateral: "1000".to_string(),
            capabilities: Some(make_capabilities()),
            cooldown_status: None,
            positions: vec![TraderStatePositionSnapshot {
                symbol: "SOL".to_string(),
                position: make_position_row(100, "150.00"),
            }],
            orders: vec![],
            splines: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12345,
        content: TraderStatePayload::Snapshot(snapshot),
    });

    assert_eq!(trader.all_positions().len(), 1);

    // Apply delta that closes the position
    let delta = TraderStateDelta {
        deltas: vec![TraderStateSubaccountDelta {
            subaccount_index: 0,
            sequence: 101,
            collateral: "1100".to_string(),
            capabilities: None,
            cooldown_status: None,
            positions: vec![TraderStatePositionDelta {
                symbol: "SOL".to_string(),
                change: TraderStateRowChangeKind::Closed,
                position: None,
            }],
            orders: vec![],
            splines: vec![],
            trade_history: vec![],
            order_history: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12346,
        content: TraderStatePayload::Delta(delta),
    });

    // Position should be removed
    assert_eq!(trader.all_positions().len(), 0);
    assert_eq!(trader.total_collateral(), 1100);
}

#[test]
fn test_stale_delta_ignored() {
    let key = TraderKey::new(Pubkey::new_unique());
    let mut trader = Trader::new(key.clone());

    // Apply snapshot with sequence 100
    let snapshot = TraderStateSnapshot {
        version: 1,
        capabilities: make_capabilities(),
        maker_fee_override_multiplier: 1.0,
        taker_fee_override_multiplier: 1.0,
        subaccounts: vec![TraderStateSubaccountSnapshot {
            subaccount_index: 0,
            sequence: 100,
            collateral: "1000".to_string(),
            capabilities: Some(make_capabilities()),
            cooldown_status: None,
            positions: vec![],
            orders: vec![],
            splines: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12345,
        content: TraderStatePayload::Snapshot(snapshot),
    });

    // Try to apply a stale delta with sequence 99
    let delta = TraderStateDelta {
        deltas: vec![TraderStateSubaccountDelta {
            subaccount_index: 0,
            sequence: 99, // Stale!
            collateral: "999".to_string(),
            capabilities: None,
            cooldown_status: None,
            positions: vec![],
            orders: vec![],
            splines: vec![],
            trade_history: vec![],
            order_history: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12346,
        content: TraderStatePayload::Delta(delta),
    });

    // Collateral should NOT be updated since delta was stale
    let subaccount = trader.primary_subaccount().unwrap();
    assert_eq!(subaccount.sequence, 100);
    assert_eq!(subaccount.collateral, 1000);
}

#[test]
fn test_multiple_subaccounts() {
    let key = TraderKey::new(Pubkey::new_unique());
    let mut trader = Trader::new(key.clone());

    // Create snapshot with multiple subaccounts
    let snapshot = TraderStateSnapshot {
        version: 1,
        capabilities: make_capabilities(),
        maker_fee_override_multiplier: 1.0,
        taker_fee_override_multiplier: 1.0,
        subaccounts: vec![
            TraderStateSubaccountSnapshot {
                subaccount_index: 0,
                sequence: 100,
                collateral: "1000".to_string(),
                capabilities: Some(make_capabilities()),
                cooldown_status: None,
                positions: vec![],
                orders: vec![],
                splines: vec![],
            },
            TraderStateSubaccountSnapshot {
                subaccount_index: 1,
                sequence: 50,
                collateral: "500".to_string(),
                capabilities: Some(make_capabilities()),
                cooldown_status: None,
                positions: vec![],
                orders: vec![],
                splines: vec![],
            },
        ],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12345,
        content: TraderStatePayload::Snapshot(snapshot),
    });

    // Verify both subaccounts
    assert_eq!(trader.subaccounts.len(), 2);
    assert_eq!(trader.total_collateral(), 1500);

    let sub0 = trader.subaccount(0).unwrap();
    assert_eq!(sub0.collateral, 1000);

    let sub1 = trader.subaccount(1).unwrap();
    assert_eq!(sub1.collateral, 500);
}

#[test]
fn test_cooldown_status_snapshot_and_delta() {
    let key = TraderKey::new(Pubkey::new_unique());
    let mut trader = Trader::new(key.clone());

    let snapshot = TraderStateSnapshot {
        version: 1,
        capabilities: make_capabilities(),
        maker_fee_override_multiplier: 1.0,
        taker_fee_override_multiplier: 1.0,
        subaccounts: vec![TraderStateSubaccountSnapshot {
            subaccount_index: 0,
            sequence: 100,
            collateral: "1000".to_string(),
            capabilities: Some(make_capabilities()),
            cooldown_status: Some(CooldownStatus {
                last_deposit_slot: 1_000,
                cooldown_period_in_slots: 200,
            }),
            positions: vec![],
            orders: vec![],
            splines: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12345,
        content: TraderStatePayload::Snapshot(snapshot),
    });

    let sub = trader.subaccount(0).unwrap();
    let initial = sub.cooldown_status.as_ref().unwrap();
    assert_eq!(initial.last_deposit_slot, 1_000);
    assert_eq!(initial.cooldown_period_in_slots, 200);

    // Missing cooldown_status in delta should preserve existing value.
    let delta_preserve = TraderStateDelta {
        deltas: vec![TraderStateSubaccountDelta {
            subaccount_index: 0,
            sequence: 101,
            collateral: "1100".to_string(),
            capabilities: None,
            cooldown_status: None,
            positions: vec![],
            orders: vec![],
            splines: vec![],
            trade_history: vec![],
            order_history: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12346,
        content: TraderStatePayload::Delta(delta_preserve),
    });

    let sub = trader.subaccount(0).unwrap();
    let preserved = sub.cooldown_status.as_ref().unwrap();
    assert_eq!(preserved.last_deposit_slot, 1_000);

    // Present cooldown_status in delta should replace existing value.
    let delta_update = TraderStateDelta {
        deltas: vec![TraderStateSubaccountDelta {
            subaccount_index: 0,
            sequence: 102,
            collateral: "1200".to_string(),
            capabilities: None,
            cooldown_status: Some(CooldownStatus {
                last_deposit_slot: 1_500,
                cooldown_period_in_slots: 300,
            }),
            positions: vec![],
            orders: vec![],
            splines: vec![],
            trade_history: vec![],
            order_history: vec![],
        }],
    };

    trader.apply_update(&TraderStateServerMessage {
        authority: key.authority_string(),
        trader_pda_index: 0,
        slot: 12347,
        content: TraderStatePayload::Delta(delta_update),
    });

    let sub = trader.subaccount(0).unwrap();
    let updated = sub.cooldown_status.as_ref().unwrap();
    assert_eq!(updated.last_deposit_slot, 1_500);
    assert_eq!(updated.cooldown_period_in_slots, 300);
}
