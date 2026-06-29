use std::path::Path;

use phoenix_rise_events::market_events as rise;

const SUPPORT_TYPES_IN_MARKET_EVENTS: &[&str] = &[
    "OrderRejectionReason",
    "OrderResidualDiscardReason",
    "OrderModificationReason",
    "PositionSizeLimits",
    "PositionSizeLimit",
    "ConditionalOrderPingStateSnapshot",
    "ConditionalOrderPingSnapshot",
    "AdminParameterUpdateKind",
    "MarkPriceConfig",
    "WithdrawConfig",
    "WithdrawRateLimitConfig",
    "FundingConfig",
    "CommodityMarketStateConfig",
    "MarketFeeConfig",
    "EscrowRequestCancelReason",
];

#[test]
fn market_event_variant_order_matches_phoenix_exchange() {
    let Some(exchange) = exchange_market_events_source() else {
        return;
    };
    let rise = rise_market_events_source();
    let rise_variants = extract_market_event_variants(&rise);
    let exchange_variants = extract_market_event_variants(&exchange);

    assert!(
        rise_variants.len() > 60,
        "expected broad MarketEvent coverage, got {} variants",
        rise_variants.len()
    );
    assert_eq!(rise_variants, exchange_variants);
}

#[test]
fn market_event_payload_shapes_match_phoenix_exchange() {
    let Some(exchange) = exchange_market_events_source() else {
        return;
    };
    let rise = rise_market_events_source();

    let mut checked = Vec::new();
    let variants = extract_market_event_variants(&exchange);
    for (_, payload) in &variants {
        assert_normalized_type_eq(&rise, &exchange, &payload);
        checked.push(payload.clone());
    }
    for support_type in SUPPORT_TYPES_IN_MARKET_EVENTS {
        assert_normalized_type_eq(&rise, &exchange, support_type);
        checked.push((*support_type).to_string());
    }

    checked.sort();
    checked.dedup();
    assert!(
        checked.len() >= variants.len(),
        "expected at least one payload shape per MarketEvent variant"
    );
}

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

fn exchange_market_events_source() -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("program-core/exchange/src/events/market_events.rs");
    if !path.exists() {
        eprintln!(
            "skipping phoenix-exchange parity check because source is absent: {}",
            path.display()
        );
        return None;
    }

    Some(
        std::fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("read phoenix-exchange market events source {}: {err}", path.display())
        }),
    )
}

fn rise_market_events_source() -> String {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = vec![
        std::fs::read_to_string(source_root.join("market_events.rs"))
            .expect("read phoenix-rise-events market events source"),
    ];

    let mut module_files: Vec<_> = std::fs::read_dir(source_root.join("market_events"))
        .expect("read phoenix-rise-events market_events module dir")
        .map(|entry| entry.expect("read market_events module entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .collect();
    module_files.sort();

    for path in module_files {
        sources.push(std::fs::read_to_string(&path).expect("read market_events module source"));
    }

    sources.join("\n")
}

fn assert_normalized_type_eq(rise: &str, exchange: &str, type_name: &str) {
    assert_eq!(
        normalize_type_block(extract_type_block(rise, type_name)),
        normalize_type_block(extract_type_block(exchange, type_name)),
        "{type_name} diverged from phoenix-exchange"
    );
}

fn extract_market_event_variants(source: &str) -> Vec<(String, String)> {
    let block = extract_type_block(source, "MarketEvent");
    block
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("#[") || line.starts_with("//") || line.is_empty() {
                return None;
            }
            let (variant, rest) = line.split_once('(')?;
            let payload = rest.split_once(')')?.0.trim();
            let variant = variant.trim();
            if variant
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            {
                Some((variant.to_string(), payload.to_string()))
            } else {
                None
            }
        })
        .collect()
}

fn extract_type_block<'a>(source: &'a str, type_name: &str) -> &'a str {
    let struct_needle = format!("pub struct {type_name}");
    let enum_needle = format!("pub enum {type_name}");
    let start = find_exact_type(source, &struct_needle)
        .or_else(|| find_exact_type(source, &enum_needle))
        .unwrap_or_else(|| panic!("{type_name} not found"));
    let source = &source[start..];
    let first_brace = source.find('{');
    let first_semicolon = source.find(';');

    if let Some(semicolon) = first_semicolon
        && first_brace.is_none_or(|brace| semicolon < brace)
    {
        return &source[..=semicolon];
    }

    let first_brace = first_brace.expect("braced type block");
    let mut depth = 0usize;
    for (offset, byte) in source[first_brace..].bytes().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let end = first_brace + offset + 1;
                    return &source[..end];
                }
            }
            _ => {}
        }
    }
    panic!("{type_name} block did not terminate");
}

fn find_exact_type(source: &str, needle: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(relative_start) = source[offset..].find(needle) {
        let start = offset + relative_start;
        let next = source[start + needle.len()..].chars().next();
        if !next.is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
            return Some(start);
        }
        offset = start + needle.len();
    }
    None
}

fn normalize_type_block(block: &str) -> String {
    let mut normalized: String = block
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("#[")
                && !line.starts_with("//")
                && !line.starts_with("///")
        })
        .flat_map(|line| line.chars())
        .filter(|ch| !ch.is_whitespace())
        .collect();
    normalized = normalized.replace(",}", "}").replace(",)", ")");
    normalized
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
