use phoenix_rise_events::market_events::SlotContextEvent;
use phoenix_rise_events::{
    LOG_EVENT_LENGTHS_INSTRUCTION_TAG, LOG_HEADER_LEN, LOG_INSTRUCTION_TAG, LogParseError,
    MarketEvent, MarketEventType, PhoenixLogInstruction, PhoenixLogInstructionKind, parse,
    parse_with_errors,
};

const LOG: PhoenixLogInstructionKind = PhoenixLogInstructionKind::Log;
const LENGTHS: PhoenixLogInstructionKind = PhoenixLogInstructionKind::LogEventLengths;

fn slot_context_event(timestamp: u64, slot: u64) -> MarketEvent {
    MarketEvent::SlotContext(SlotContextEvent { timestamp, slot })
}

fn serialized_slot_context(timestamp: u64, slot: u64) -> Vec<u8> {
    borsh::to_vec(&slot_context_event(timestamp, slot)).unwrap()
}

fn log_payload(batch_index: u32, event_count: u32, events_bytes: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&batch_index.to_le_bytes());
    out.extend_from_slice(&event_count.to_le_bytes());
    for bytes in events_bytes {
        out.extend_from_slice(bytes);
    }
    out
}

fn lengths_payload(batch_index: u32, lengths: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&batch_index.to_le_bytes());
    out.extend_from_slice(&(lengths.len() as u32).to_le_bytes());
    for len in lengths {
        out.extend_from_slice(&len.to_le_bytes());
    }
    out
}

#[test]
fn discriminants_match_eternal_instruction_tags() {
    assert_eq!(LOG_INSTRUCTION_TAG, 0xaacfd109f2d6e68d);
    assert_eq!(LOG_EVENT_LENGTHS_INSTRUCTION_TAG, 0x479947b5cb8607f7);
    assert_eq!(LOG.discriminant(), LOG_INSTRUCTION_TAG.to_le_bytes());
    assert_eq!(
        LENGTHS.discriminant(),
        LOG_EVENT_LENGTHS_INSTRUCTION_TAG.to_le_bytes()
    );
}

#[test]
fn from_instruction_data_splits_discriminant_and_payload() {
    let payload = lengths_payload(7, &[1, 2]);
    let mut data = LOG_EVENT_LENGTHS_INSTRUCTION_TAG.to_le_bytes().to_vec();
    data.extend_from_slice(&payload);

    let parsed = PhoenixLogInstruction::from_instruction_data(&data).unwrap();
    assert_eq!(parsed.kind, LENGTHS);
    assert_eq!(parsed.data, payload);
}

#[test]
fn market_event_type_displays_stable_variant_name() {
    let event = slot_context_event(1, 2);
    let event_type = event.event_type();

    assert_eq!(event_type, MarketEventType::SlotContext);
    assert_eq!(MarketEventType::from(&event), MarketEventType::SlotContext);
    assert_eq!(event_type.to_string(), "SlotContext");
}

#[test]
fn parses_length_guided_log_batches() {
    let e1 = serialized_slot_context(10, 20);
    let e2 = serialized_slot_context(30, 40);
    let lengths = lengths_payload(0, &[e1.len() as u16, e2.len() as u16]);
    let log = log_payload(0, 2, &[&e1, &e2]);

    let out = parse([
        PhoenixLogInstruction::new(LENGTHS, &lengths),
        PhoenixLogInstruction::new(LOG, &log),
    ]);

    assert_eq!(out.len(), 2);
    assert!(matches!(
        out[0],
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 10,
            slot: 20
        })
    ));
    assert!(matches!(
        out[1],
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 30,
            slot: 40
        })
    ));
}

#[test]
fn queues_duplicate_batch_indices_in_flat_streams() {
    let first = serialized_slot_context(1, 10);
    let second = serialized_slot_context(2, 20);
    let first_lengths = lengths_payload(0, &[first.len() as u16]);
    let first_log = log_payload(0, 1, &[&first]);
    let second_lengths = lengths_payload(0, &[second.len() as u16]);
    let second_log = log_payload(0, 1, &[&second]);

    let out = parse([
        PhoenixLogInstruction::new(LENGTHS, &first_lengths),
        PhoenixLogInstruction::new(LENGTHS, &second_lengths),
        PhoenixLogInstruction::new(LOG, &first_log),
        PhoenixLogInstruction::new(LOG, &second_log),
    ]);

    assert_eq!(out.len(), 2);
    assert!(matches!(
        out[0],
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 1,
            slot: 10
        })
    ));
    assert!(matches!(
        out[1],
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 2,
            slot: 20
        })
    ));
}

#[test]
fn parse_with_errors_returns_skipped_event_bytes() {
    let valid = serialized_slot_context(1, 2);
    let invalid = [0xff, 0x00, 0x00, 0x00];
    let lengths = lengths_payload(0, &[valid.len() as u16, invalid.len() as u16]);
    let log = log_payload(0, 2, &[&valid, &invalid]);

    let out = parse_with_errors([
        PhoenixLogInstruction::new(LENGTHS, &lengths),
        PhoenixLogInstruction::new(LOG, &log),
    ]);

    assert_eq!(out.events.len(), 1);
    assert_eq!(out.skipped_event_bytes, vec![invalid.as_slice()]);
    assert!(out.failures.iter().any(|failure| {
        matches!(
            failure.error,
            LogParseError::EventDecodeFailed {
                batch_index: 0,
                event_index: 1,
                ..
            }
        )
    }));
}

#[test]
fn malformed_event_length_stops_at_valid_prefix() {
    let valid = serialized_slot_context(1, 2);
    let lengths = lengths_payload(0, &[valid.len() as u16, 9999]);
    let log = log_payload(0, 2, &[&valid]);

    let out = parse_with_errors([
        PhoenixLogInstruction::new(LENGTHS, &lengths),
        PhoenixLogInstruction::new(LOG, &log),
    ]);

    assert_eq!(out.events.len(), 1);
    assert!(matches!(
        out.failures.last().map(|failure| &failure.error),
        Some(LogParseError::EventOutOfBounds {
            batch_index: 0,
            event_index: 1,
            ..
        })
    ));
}

#[test]
fn falls_back_to_legacy_off_chain_event_layout_when_lengths_absent() {
    let legacy = phoenix_rise_events::OffChainMarketEvent {
        batch_index: 0,
        events: vec![slot_context_event(3, 4)],
    };
    let log = borsh::to_vec(&legacy).unwrap();

    let out = parse([PhoenixLogInstruction::new(LOG, &log)]);

    assert_eq!(out.len(), 1);
    assert!(matches!(
        out[0],
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 3,
            slot: 4
        })
    ));
}

#[test]
fn logs_without_matching_lengths_are_skipped_when_any_lengths_exist() {
    let event = serialized_slot_context(1, 2);
    let lengths = lengths_payload(7, &[event.len() as u16]);
    let log = log_payload(0, 1, &[&event]);

    let out = parse_with_errors([
        PhoenixLogInstruction::new(LENGTHS, &lengths),
        PhoenixLogInstruction::new(LOG, &log),
    ]);

    assert!(out.events.is_empty());
    assert!(matches!(
        out.failures.last().map(|failure| &failure.error),
        Some(LogParseError::MissingLengths { batch_index: 0 })
    ));
}

#[test]
fn invalid_lengths_payload_is_reported() {
    let data = vec![0u8; LOG_HEADER_LEN + 1];
    let out = parse_with_errors([PhoenixLogInstruction::new(LENGTHS, &data)]);

    assert!(out.events.is_empty());
    assert!(matches!(
        out.failures.first().map(|failure| &failure.error),
        Some(LogParseError::InvalidLengthsByteCount { .. })
    ));
}
