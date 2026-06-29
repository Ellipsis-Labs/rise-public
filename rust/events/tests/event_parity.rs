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
