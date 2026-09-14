use std::path::Path;

use phoenix_rise_events::market_events as rise;

#[test]
fn representative_event_serialization_matches_phoenix_exchange() {
    assert_eq!(
        borsh::to_vec(&rise::MarketEvent::SlotContext(rise::SlotContextEvent {
            timestamp: 123,
            slot: 456,
        }))
        .unwrap(),
        expected_slot_context_bytes(123, 456)
    );

    assert_eq!(
        borsh::to_vec(&rise::MarketEvent::MarketTombstoned(
            rise::MarketTombstonedEvent {
                previous_market_status: rise::MarketStatus::Closed,
                final_sequence_number: 10,
                final_trade_sequence_number: 11,
                final_order_sequence_number: 12,
            },
        ))
        .unwrap(),
        expected_market_tombstoned_bytes()
    );

    assert_eq!(
        borsh::to_vec(&rise::MarketEvent::AdminParameterUpdated(
            rise::AdminParameterUpdatedEvent {
                authority: [7; 32],
                asset_symbol: Some(rise::Symbol::new("SOL").unwrap()),
                asset_id: Some(1),
                update_kind: rise::AdminParameterUpdateKind::FeatureSet {
                    previous: rise::FeatureSet::new(0),
                    new: rise::FeatureSet::new(1),
                },
            },
        ))
        .unwrap(),
        expected_admin_feature_set_bytes()
    );
}

/// The `SpotCollateralConfig` admin payload embeds the account struct
/// verbatim, padding included. If this mirror is short, every enclosing event
/// decodes misaligned from the `new` field onward.
#[test]
fn spot_collateral_metadata_borsh_size_matches_onchain_layout() {
    assert_eq!(
        borsh::to_vec(&spot_collateral_metadata()).unwrap().len(),
        320
    );
}

/// Borsh enum tags are positional, so the three spot-collateral events must
/// sit at exactly 67/68/69. A drift here silently misreads every event after
/// `OrderResidualDiscarded`.
#[test]
fn spot_collateral_events_use_the_expected_borsh_tags() {
    let deposited = borsh::to_vec(&rise::MarketEvent::SpotCollateralDeposited(
        spot_collateral_deposited_event(),
    ))
    .unwrap();
    assert_eq!(deposited[0], 67);
    assert_eq!(deposited, expected_spot_collateral_deposited_bytes());

    let withdrawn = borsh::to_vec(&rise::MarketEvent::SpotCollateralWithdrawn(
        rise::SpotCollateralWithdrawnEvent {
            asset_index: 0xFFFF_0000,
            trader: [1; 32],
            authority: [2; 32],
            destination: [3; 32],
            amount: 5,
            excess: 6,
            withdraw_excess: true,
            is_self_cpi: false,
            flow: rise::SpotCollateralFlow::Withdraw,
            new_collateral_balance: 7,
            post_global_collateral: 8,
            trader_sequence_number: 9,
            prev_sequence_number_slot: 10,
        },
    ))
    .unwrap();
    assert_eq!(withdrawn[0], 68);

    let liquidated = borsh::to_vec(&rise::MarketEvent::SpotCollateralLiquidated(
        rise::SpotCollateralLiquidatedEvent {
            liquidator: [4; 32],
            liquidated_trader: [5; 32],
            asset_index: 0xFFFF_0000,
            liquidation_size: 11,
            over_cap_excess: 12,
            quote_lots_deposited: rise::QuoteLots::new(13),
            oracle_notional: rise::QuoteLots::new(14),
            liquidation_discount: rise::BasisPoints::new(500),
        },
    ))
    .unwrap();
    assert_eq!(liquidated[0], 69);
}

/// Regression: before the spot-collateral variants existed, an unknown trailing
/// Borsh tag failed the whole batch rather than one event, so a single spot
/// collateral event took down every event in the transaction.
#[test]
fn a_batch_containing_a_spot_collateral_event_decodes_completely() {
    let events = vec![
        rise::MarketEvent::SlotContext(rise::SlotContextEvent {
            timestamp: 1,
            slot: 2,
        }),
        rise::MarketEvent::SpotCollateralDeposited(spot_collateral_deposited_event()),
        rise::MarketEvent::SlotContext(rise::SlotContextEvent {
            timestamp: 3,
            slot: 4,
        }),
    ];

    let batch = rise::OffChainMarketEvent {
        batch_index: 0,
        events: events.clone(),
    };
    let decoded: rise::OffChainMarketEvent =
        borsh::from_slice(&borsh::to_vec(&batch).unwrap()).unwrap();

    assert_eq!(decoded.events.len(), events.len());
    assert_eq!(
        decoded
            .events
            .iter()
            .map(rise::MarketEventType::from)
            .collect::<Vec<_>>(),
        vec![
            rise::MarketEventType::SlotContext,
            rise::MarketEventType::SpotCollateralDeposited,
            rise::MarketEventType::SlotContext,
        ]
    );
}

fn spot_collateral_metadata() -> rise::SpotCollateralMetadata {
    rise::SpotCollateralMetadata {
        mint_address: [0; 32],
        decimals: 9,
        perp_asset_index: 1,
        max_per_trader_balance: 1_000_000_000_000,
        max_global_balance: 100_000_000_000_000,
        curr_global_balance: 0,
        min_margin_discount: 500,
        max_margin_discount: 1_000,
        max_liquidation_discount: 500,
        min_liquidation_slippage: 5,
        max_liquidation_size: 100_000_000_000,
        post_liquidation_buffer: rise::QuoteLots::new(10_000_000),
        quote_lot_collateral_shortfall_buffer: rise::QuoteLots::new(10_000_000_000),
        flags: 0b011,
        _padding_flags: [0; 7],
        padding: [0; 26],
    }
}

fn spot_collateral_deposited_event() -> rise::SpotCollateralDepositedEvent {
    rise::SpotCollateralDepositedEvent {
        asset_index: 0xFFFF_0000,
        trader: [1; 32],
        authority: [2; 32],
        amount: 100,
        requested_amount: 150,
        binding_cap: Some(rise::SpotCollateralCap::PerTrader),
        flow: rise::SpotCollateralFlow::Sync,
        new_collateral_balance: 100,
        post_global_collateral: 200,
        trader_sequence_number: 3,
        prev_sequence_number_slot: 4,
    }
}

fn expected_spot_collateral_deposited_bytes() -> Vec<u8> {
    let mut out = vec![67];
    out.extend_from_slice(&0xFFFF_0000u32.to_le_bytes());
    out.extend_from_slice(&[1; 32]);
    out.extend_from_slice(&[2; 32]);
    out.extend_from_slice(&100u64.to_le_bytes());
    out.extend_from_slice(&150u64.to_le_bytes());
    // Option<SpotCollateralCap>::Some(PerTrader)
    out.push(1);
    out.push(0);
    // SpotCollateralFlow::Sync
    out.push(0);
    out.extend_from_slice(&100u64.to_le_bytes());
    out.extend_from_slice(&200u64.to_le_bytes());
    out.extend_from_slice(&3u64.to_le_bytes());
    out.extend_from_slice(&4u64.to_le_bytes());
    out
}

#[test]
fn rise_runtime_sources_do_not_import_phoenix_exchange() {
    let rise_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("events crate has a rise/rust parent");
    let mut violations = Vec::new();

    visit_files(rise_root, &mut |path| {
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            return;
        }
        if path.components().any(|component| {
            let value = component.as_os_str().to_string_lossy();
            value == "target" || value == "tests"
        }) {
            return;
        }

        let contents = std::fs::read_to_string(path).expect("read source file");
        if contents.contains("phoenix_exchange") || contents.contains("phoenix-exchange") {
            violations.push(path.strip_prefix(rise_root).unwrap().display().to_string());
        }
    });

    assert!(
        violations.is_empty(),
        "Rise runtime sources must not import phoenix-exchange: {violations:?}"
    );
}

fn expected_slot_context_bytes(timestamp: u64, slot: u64) -> Vec<u8> {
    let mut out = vec![0];
    out.extend_from_slice(&timestamp.to_le_bytes());
    out.extend_from_slice(&slot.to_le_bytes());
    out
}

fn expected_market_tombstoned_bytes() -> Vec<u8> {
    let mut out = vec![64, 4];
    out.extend_from_slice(&10u64.to_le_bytes());
    out.extend_from_slice(&11u64.to_le_bytes());
    out.extend_from_slice(&12u64.to_le_bytes());
    out
}

fn expected_admin_feature_set_bytes() -> Vec<u8> {
    let mut out = vec![48];
    out.extend_from_slice(&[7; 32]);
    out.push(1);
    let mut symbol = [0u8; 16];
    symbol[..3].copy_from_slice(b"SOL");
    out.extend_from_slice(&symbol);
    out.push(1);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.push(15);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out
}

fn visit_files(root: &Path, f: &mut impl FnMut(&Path)) {
    for entry in std::fs::read_dir(root).expect("read directory") {
        let entry = entry.expect("read directory entry");
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, f);
        } else {
            f(&path);
        }
    }
}
