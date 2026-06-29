#![cfg(feature = "events")]

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use phoenix_rise_core::events::{
    ExecutedInstructionView, parse_events_from_executed_instructions_with_errors,
};
use serde::Deserialize;

const FIXTURE_JSON: &str = include_str!("fixtures/phoenix-event-transactions.json");

#[test]
fn helius_reduced_transaction_fixtures_parse_without_errors() {
    let fixture: EventTransactionFixtureFile =
        serde_json::from_str(FIXTURE_JSON).expect("event transaction fixture should parse");
    assert_eq!(fixture.schema_version, 1);
    assert!(
        !fixture.transactions.is_empty(),
        "fixture should contain at least one transaction"
    );

    let program_id = decode_pubkey(&fixture.source.program_id);
    let fixture_signatures = fixture
        .transactions
        .iter()
        .map(|transaction| transaction.signature.as_str())
        .collect::<BTreeSet<_>>();
    for included_signature in &fixture.source.included_signatures {
        assert!(
            fixture_signatures.contains(included_signature.as_str()),
            "explicitly included fixture signature {included_signature} is missing"
        );
    }
    let mut covered_variants = BTreeSet::new();

    for transaction in &fixture.transactions {
        let decoded_instructions = transaction
            .instructions
            .iter()
            .map(decode_instruction)
            .collect::<Vec<_>>();
        let views = decoded_instructions
            .iter()
            .map(|instruction| ExecutedInstructionView {
                program_id: &instruction.program_id,
                data: &instruction.data,
                stack_path: &instruction.stack_path,
            });

        let parsed = parse_events_from_executed_instructions_with_errors(views, &program_id);
        assert!(
            parsed.orphan_logs.is_empty(),
            "{} should not leave orphan log instructions: {:?}",
            transaction.signature,
            parsed.orphan_logs
        );

        let mut actual_event_count = 0usize;
        let mut actual_variants = BTreeSet::new();
        let mut actual_expected = Vec::new();

        for instruction in parsed.instructions {
            assert!(
                instruction.failures.is_empty(),
                "{} instruction 0x{:016x} at {:?} had parse failures: {:?}",
                transaction.signature,
                instruction.instruction_tag,
                instruction.stack_path,
                instruction.failures
            );
            assert!(
                instruction.skipped_event_bytes.is_empty(),
                "{} instruction 0x{:016x} at {:?} skipped event bytes",
                transaction.signature,
                instruction.instruction_tag,
                instruction.stack_path
            );
            if instruction.events.is_empty() {
                continue;
            }

            let event_variants = instruction
                .events
                .iter()
                .map(|event| event.event_type().to_string())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            actual_event_count += instruction.events.len();
            actual_variants.extend(event_variants.iter().cloned());
            actual_expected.push(FixtureExpectedInstruction {
                instruction_tag_hex: format!("0x{:016x}", instruction.instruction_tag),
                stack_path: instruction.stack_path,
                events_count: instruction.events.len(),
                event_variants,
            });
        }

        covered_variants.extend(actual_variants.iter().cloned());
        assert_eq!(
            actual_event_count, transaction.events_count,
            "{} event count changed",
            transaction.signature
        );
        assert_eq!(
            actual_variants.into_iter().collect::<Vec<_>>(),
            transaction.event_variants,
            "{} event variant set changed",
            transaction.signature
        );
        assert_eq!(
            actual_expected, transaction.expected.instructions,
            "{} per-instruction expected events changed",
            transaction.signature
        );
    }

    let observed_variants = fixture
        .analysis
        .by_event_variant
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        covered_variants, observed_variants,
        "reduced fixtures should cover every event variant observed in the 10k transaction sample"
    );
}

fn decode_instruction(instruction: &FixtureInstruction) -> DecodedFixtureInstruction {
    DecodedFixtureInstruction {
        program_id: decode_pubkey(&instruction.program_id),
        stack_path: instruction.stack_path.clone(),
        data: BASE64
            .decode(&instruction.data_base64)
            .expect("fixture instruction data should be base64"),
    }
}

fn decode_pubkey(value: &str) -> Vec<u8> {
    let bytes = bs58::decode(value)
        .into_vec()
        .expect("fixture pubkey should be base58");
    assert_eq!(bytes.len(), 32, "fixture pubkey should be 32 bytes");
    bytes
}

struct DecodedFixtureInstruction {
    program_id: Vec<u8>,
    stack_path: Vec<u8>,
    data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventTransactionFixtureFile {
    schema_version: u32,
    source: EventFixtureSource,
    analysis: EventFixtureAnalysis,
    transactions: Vec<EventTransactionFixture>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventFixtureSource {
    program_id: String,
    #[serde(default)]
    included_signatures: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventFixtureAnalysis {
    by_event_variant: BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventTransactionFixture {
    signature: String,
    events_count: usize,
    event_variants: Vec<String>,
    instructions: Vec<FixtureInstruction>,
    expected: FixtureExpected,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureInstruction {
    program_id: String,
    stack_path: Vec<u8>,
    data_base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureExpected {
    instructions: Vec<FixtureExpectedInstruction>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct FixtureExpectedInstruction {
    instruction_tag_hex: String,
    stack_path: Vec<u8>,
    events_count: usize,
    event_variants: Vec<String>,
}
