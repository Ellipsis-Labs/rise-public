#![cfg(feature = "events")]

#[path = "support/fill_id_fixtures.rs"]
mod fixture;

use fixture::{RecordedInstruction, RecordedTransaction};
use phoenix_rise_core::events::parse_events_from_executed_instructions_with_errors;
use phoenix_rise_events::fill_id::{EventParserCtx, collect_parsed_pnls_with_fill_id};
use phoenix_rise_ix::discriminants::PhoenixInstruction;
use solana_pubkey::Pubkey;

#[test]
fn fill_id_matches_api_for_recorded_sol_taker_pnl() {
    let mut fixture: RecordedTransaction = serde_json::from_str(include_str!(
        "fixtures/fill_id_4ckFfh3mecHTh6ugpusMdFKoTD4ETq4QCosfTtvYdTbgw5YYJcvsiiFoiH7nC2dyFm6UR9xepEBdPUuWmidDEE8W.json"
    )).unwrap();
    assert_eq!(fixture.slot, 414988276);
    let program: Pubkey = "EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih"
        .parse()
        .unwrap();
    // This reduced recording contains only self-CPIs. Reconstruct its known
    // top-level producer so the logger pair has the original Phoenix ordinal.
    fixture.top_level_instructions.push(RecordedInstruction {
        instruction_index: 2,
        stack_height: Some(1),
        program_id: program.to_string(),
        data_base58: bs58::encode(PhoenixInstruction::PlaceMarketOrder.discriminant())
            .into_string(),
    });
    let instructions = fixture.execution_order();
    let parsed = parse_events_from_executed_instructions_with_errors(
        instructions.iter().map(|instruction| instruction.view()),
        program.as_ref(),
    );
    assert!(parsed.orphan_logs.is_empty());
    assert_eq!(parsed.instructions.len(), 1);
    let instruction = &parsed.instructions[0];
    assert!(
        instruction.failures.is_empty(),
        "{:?}",
        instruction.failures
    );
    assert_eq!(instruction.stack_path, [2]);
    assert_eq!(instruction.phoenix_instruction_index, 0);
    let context = EventParserCtx {
        signature: fixture.signature,
        top_level_instruction_index: 2,
        stack_height: Some(2),
        phoenix_instruction_index: i32::try_from(instruction.phoenix_instruction_index).unwrap(),
        instruction_type: "PlaceMarketOrder".to_owned(),
    };
    let pnls = collect_parsed_pnls_with_fill_id(&context, &instruction.events).unwrap();
    let matching = pnls
        .iter()
        .filter(|pnl| pnl.fill_id.to_string() == "8ba681db-23e9-3787-93e1-8b5886adc608")
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1);
    let pnl = &matching[0].event;
    assert_eq!(pnl.asset_symbol.to_string(), "SOL");
    assert_eq!(pnl.asset_id, 0);
    assert_eq!(pnl.base_lots_before.as_inner(), -100);
    assert_eq!(pnl.base_lots_after.as_inner(), -300);
    assert_eq!(pnl.realized_pnl.as_inner(), 0);
}
