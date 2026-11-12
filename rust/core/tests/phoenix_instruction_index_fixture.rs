#![cfg(feature = "events")]

#[path = "support/fill_id_fixtures.rs"]
mod fixture;

use fixture::RecordedTransaction;
use phoenix_rise_core::events::parse_events_from_executed_instructions_with_errors;
use phoenix_rise_ix::discriminants::PhoenixInstruction;
use solana_pubkey::Pubkey;

#[test]
fn recorded_phoenix_ordinals_ignore_non_phoenix_instructions_and_log_cpis() {
    let fixture: RecordedTransaction = serde_json::from_str(include_str!(
        "fixtures/phoenix_instruction_index_hZEjEjgXmruKapZybvau7Jyer6L66yK5NbUAxnCCtfhYihb4sj6qZdVEbKyAHMSZS9PxdmXC77RoT2hbPPYQPve.json"
    )).unwrap();
    assert_eq!(
        fixture.signature,
        "hZEjEjgXmruKapZybvau7Jyer6L66yK5NbUAxnCCtfhYihb4sj6qZdVEbKyAHMSZS9PxdmXC77RoT2hbPPYQPve"
    );
    assert_eq!(fixture.slot, 414726931);
    let instructions = fixture.execution_order();
    let program: Pubkey = "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"
        .parse()
        .unwrap();
    let parsed = parse_events_from_executed_instructions_with_errors(
        instructions.iter().map(|instruction| instruction.view()),
        program.as_ref(),
    );
    assert!(parsed.orphan_logs.is_empty());
    assert_eq!(parsed.instructions.len(), 2);
    for (instruction, (ordinal, top_index, kind)) in parsed.instructions.iter().zip([
        (0, 1, PhoenixInstruction::CancelAll),
        (1, 2, PhoenixInstruction::PlaceMultiLimitOrder),
    ]) {
        assert_eq!(instruction.phoenix_instruction_index, ordinal);
        assert_eq!(instruction.stack_path, [top_index]);
        assert_eq!(
            instruction.instruction_tag.to_le_bytes(),
            kind.discriminant()
        );
        assert!(
            instruction.failures.is_empty(),
            "{:?}",
            instruction.failures
        );
        assert!(!instruction.events.is_empty());
    }
}
