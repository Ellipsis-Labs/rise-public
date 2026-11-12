#![cfg(feature = "events")]

use phoenix_rise_core::events::{
    ExecutedInstructionView, LOG_EVENT_LENGTHS_INSTRUCTION_TAG, LOG_INSTRUCTION_TAG,
    RpcInnerInstructionGroup, RpcInnerInstructionView, RpcInstructionView, advance_inner_path,
    parse_events_from_executed_instructions_with_errors,
    parse_events_from_rpc_instructions_with_errors,
};
use phoenix_rise_events::MarketEvent;
use phoenix_rise_events::fill_id::generate_fill_id_from_pnl;
use phoenix_rise_events::market_events::{PnLEvent, SlotContextEvent, Symbol};
use phoenix_rise_math::{SignedBaseLots, SignedQuoteLots};

const PHOENIX: &[u8] = &[7; 32];
const OTHER: &[u8] = &[9; 32];
const PLACE_LIMIT_ORDER_TAG: u64 = 0x1122_3344_5566_7788;
const PLACE_MARKET_ORDER_TAG: u64 = 0x8877_6655_4433_2211;

fn instruction_data(tag: u64, payload: &[u8]) -> Vec<u8> {
    let mut out = tag.to_le_bytes().to_vec();
    out.extend_from_slice(payload);
    out
}

fn event(timestamp: u64, slot: u64) -> Vec<u8> {
    borsh::to_vec(&MarketEvent::SlotContext(SlotContextEvent {
        timestamp,
        slot,
    }))
    .unwrap()
}

fn log_payload(batch_index: u32, events: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&batch_index.to_le_bytes());
    out.extend_from_slice(&(events.len() as u32).to_le_bytes());
    for event in events {
        out.extend_from_slice(event);
    }
    out
}

fn lengths_payload(batch_index: u32, lengths: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&batch_index.to_le_bytes());
    out.extend_from_slice(&(lengths.len() as u32).to_le_bytes());
    for length in lengths {
        out.extend_from_slice(&length.to_le_bytes());
    }
    out
}

#[test]
fn executed_instruction_parser_uses_stack_path_parent_for_logs() {
    let e1 = event(1, 10);
    let e2 = event(2, 20);
    let parent_a = instruction_data(PLACE_LIMIT_ORDER_TAG, &[]);
    let parent_b = instruction_data(PLACE_MARKET_ORDER_TAG, &[]);
    let lengths_a = instruction_data(
        LOG_EVENT_LENGTHS_INSTRUCTION_TAG,
        &lengths_payload(0, &[e1.len() as u16]),
    );
    let log_a = instruction_data(LOG_INSTRUCTION_TAG, &log_payload(0, &[&e1]));
    let lengths_b = instruction_data(
        LOG_EVENT_LENGTHS_INSTRUCTION_TAG,
        &lengths_payload(0, &[e2.len() as u16]),
    );
    let log_b = instruction_data(LOG_INSTRUCTION_TAG, &log_payload(0, &[&e2]));

    let parsed = parse_events_from_executed_instructions_with_errors(
        [
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &parent_a,
                stack_path: &[0, 0],
            },
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &lengths_a,
                stack_path: &[0, 0, 0],
            },
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &log_a,
                stack_path: &[0, 0, 1],
            },
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &parent_b,
                stack_path: &[0, 1],
            },
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &lengths_b,
                stack_path: &[0, 1, 0],
            },
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &log_b,
                stack_path: &[0, 1, 1],
            },
        ],
        PHOENIX,
    );

    assert!(parsed.orphan_logs.is_empty());
    assert_eq!(parsed.instructions.len(), 2);
    assert_eq!(parsed.instructions[0].stack_path, vec![0, 0]);
    assert_eq!(parsed.instructions[1].stack_path, vec![0, 1]);
    assert!(matches!(
        parsed.instructions[0].events[0],
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 1,
            slot: 10
        })
    ));
    assert!(matches!(
        parsed.instructions[1].events[0],
        MarketEvent::SlotContext(SlotContextEvent {
            timestamp: 2,
            slot: 20
        })
    ));
}

#[test]
fn rpc_instruction_parser_reconstructs_inner_stack_paths() {
    let e1 = event(1, 10);
    let parent = instruction_data(PLACE_LIMIT_ORDER_TAG, &[]);
    let other = instruction_data(0xaa, &[]);
    let lengths = instruction_data(
        LOG_EVENT_LENGTHS_INSTRUCTION_TAG,
        &lengths_payload(0, &[e1.len() as u16]),
    );
    let log = instruction_data(LOG_INSTRUCTION_TAG, &log_payload(0, &[&e1]));

    let inner = [
        RpcInnerInstructionView {
            program_id: PHOENIX,
            data: &parent,
            stack_height: Some(2),
        },
        RpcInnerInstructionView {
            program_id: PHOENIX,
            data: &lengths,
            stack_height: Some(3),
        },
        RpcInnerInstructionView {
            program_id: PHOENIX,
            data: &log,
            stack_height: Some(3),
        },
        RpcInnerInstructionView {
            program_id: OTHER,
            data: &other,
            stack_height: Some(2),
        },
    ];
    let groups = [RpcInnerInstructionGroup {
        index: 0,
        instructions: &inner,
    }];
    let top = [RpcInstructionView {
        program_id: OTHER,
        data: &other,
    }];
    let parsed = parse_events_from_rpc_instructions_with_errors(&top, &groups, PHOENIX);

    assert_eq!(parsed.instructions.len(), 1);
    assert_eq!(parsed.instructions[0].stack_path, vec![0, 0]);
    assert_eq!(parsed.instructions[0].events.len(), 1);
}

#[test]
fn missing_rpc_stack_height_falls_back_to_previous_phoenix_instruction() {
    let e1 = event(1, 10);
    let parent = instruction_data(PLACE_LIMIT_ORDER_TAG, &[]);
    let lengths = instruction_data(
        LOG_EVENT_LENGTHS_INSTRUCTION_TAG,
        &lengths_payload(0, &[e1.len() as u16]),
    );
    let log = instruction_data(LOG_INSTRUCTION_TAG, &log_payload(0, &[&e1]));
    let top = [RpcInstructionView {
        program_id: OTHER,
        data: &[],
    }];
    let inner = [
        RpcInnerInstructionView {
            program_id: PHOENIX,
            data: &parent,
            stack_height: None,
        },
        RpcInnerInstructionView {
            program_id: PHOENIX,
            data: &lengths,
            stack_height: None,
        },
        RpcInnerInstructionView {
            program_id: PHOENIX,
            data: &log,
            stack_height: None,
        },
    ];
    let groups = [RpcInnerInstructionGroup {
        index: 0,
        instructions: &inner,
    }];

    let parsed = parse_events_from_rpc_instructions_with_errors(&top, &groups, PHOENIX);
    assert_eq!(parsed.instructions.len(), 1);
    assert_eq!(parsed.instructions[0].events.len(), 1);
}

#[test]
fn advance_inner_path_matches_rpc_preorder_transitions() {
    let mut path = Vec::new();
    let mut out = Vec::new();
    for stack_height in [Some(2), Some(3), Some(3), Some(2), None] {
        advance_inner_path(&mut path, stack_height);
        out.push(path.clone());
    }

    assert_eq!(out, vec![vec![0], vec![0, 0], vec![0, 1], vec![1], vec![2]]);
}

#[test]
fn nested_and_sibling_phoenix_invocations_preserve_fill_id_ordinals() {
    let mut signature = [0; 64];
    signature[0] = 7;
    signature[63] = 8;
    let signature = bs58::encode(signature).into_string();
    let mut trader = [0; 32];
    trader[0] = 11;
    trader[31] = 12;
    let pnl = |before, after| {
        borsh::to_vec(&MarketEvent::PnL(PnLEvent {
            trader,
            asset_id: 42,
            asset_symbol: Symbol::new("SOL").unwrap(),
            realized_pnl: SignedQuoteLots::new(-25),
            funding_payment: SignedQuoteLots::ZERO,
            base_lots_before: SignedBaseLots::new(before),
            base_lots_after: SignedBaseLots::new(after),
            virtual_quote_lots_before: SignedQuoteLots::ZERO,
            virtual_quote_lots_after: SignedQuoteLots::ZERO,
        }))
        .unwrap()
    };
    let first = pnl(100, 150);
    let second = pnl(150, 175);
    let lengths = |event: &[u8], batch| {
        instruction_data(
            LOG_EVENT_LENGTHS_INSTRUCTION_TAG,
            &lengths_payload(batch, &[u16::try_from(event.len()).unwrap()]),
        )
    };
    let log =
        |event: &[u8], batch| instruction_data(LOG_INSTRUCTION_TAG, &log_payload(batch, &[event]));
    let instructions = [
        // Eventless Phoenix calls still consume an ordinal.
        (
            PHOENIX,
            instruction_data(PLACE_LIMIT_ORDER_TAG, &[]),
            vec![0],
        ),
        (OTHER, vec![], vec![1]),
        (
            PHOENIX,
            instruction_data(PLACE_MARKET_ORDER_TAG, &[]),
            vec![1, 0],
        ),
        (PHOENIX, lengths(&first, 0), vec![1, 0, 0]),
        (PHOENIX, log(&first, 0), vec![1, 0, 1]),
        (OTHER, vec![], vec![1, 0, 2]),
        (
            PHOENIX,
            instruction_data(PLACE_MARKET_ORDER_TAG, &[]),
            vec![1, 0, 2, 0],
        ),
        (PHOENIX, lengths(&first, 0), vec![1, 0, 2, 0, 0]),
        (PHOENIX, log(&first, 0), vec![1, 0, 2, 0, 1]),
        // The outer producer resumes after the nested invocation returns.
        (PHOENIX, lengths(&second, 1), vec![1, 0, 3]),
        (PHOENIX, log(&second, 1), vec![1, 0, 4]),
        (
            PHOENIX,
            instruction_data(PLACE_MARKET_ORDER_TAG, &[]),
            vec![1, 1],
        ),
        (PHOENIX, lengths(&first, 0), vec![1, 1, 0]),
        (PHOENIX, log(&first, 0), vec![1, 1, 1]),
    ];
    let executed = parse_events_from_executed_instructions_with_errors(
        instructions
            .iter()
            .map(|(program_id, data, stack_path)| ExecutedInstructionView {
                program_id,
                data,
                stack_path,
            }),
        PHOENIX,
    );
    let top = instructions
        .iter()
        .filter(|(_, _, path)| path.len() == 1)
        .map(|(program_id, data, _)| RpcInstructionView { program_id, data })
        .collect::<Vec<_>>();
    let inner = instructions
        .iter()
        .filter(|(_, _, path)| path.len() > 1)
        .map(|(program_id, data, path)| RpcInnerInstructionView {
            program_id,
            data,
            stack_height: Some(u32::try_from(path.len()).unwrap()),
        })
        .collect::<Vec<_>>();
    let groups = [RpcInnerInstructionGroup {
        index: 1,
        instructions: &inner,
    }];
    let rpc = parse_events_from_rpc_instructions_with_errors(&top, &groups, PHOENIX);
    for parsed in [executed, rpc] {
        assert!(parsed.orphan_logs.is_empty());
        assert_eq!(parsed.instructions.len(), 4);
        assert_eq!(parsed.instructions[0].phoenix_instruction_index, 0);
        assert!(parsed.instructions[0].events.is_empty());
        let mut ids = Vec::new();
        for (instruction, (ordinal, path)) in parsed.instructions.iter().skip(1).zip([
            (1, vec![1, 0]),
            (2, vec![1, 0, 2, 0]),
            (3, vec![1, 1]),
        ]) {
            assert_eq!(instruction.phoenix_instruction_index, ordinal);
            assert_eq!(instruction.stack_path, path);
            assert!(
                instruction.failures.is_empty(),
                "{:?}",
                instruction.failures
            );
            for event in &instruction.events {
                let MarketEvent::PnL(event) = event else {
                    panic!("expected PnL");
                };
                ids.push(
                    generate_fill_id_from_pnl(&signature, i32::try_from(ordinal).unwrap(), event)
                        .unwrap()
                        .to_string(),
                );
            }
        }
        assert_eq!(
            ids,
            [
                "cfa704c6-ffce-3b7d-8eb4-7a35ccf62d36",
                "cd2e7836-9e21-3d4d-8c2e-1f4c9457f703",
                "e7f270ef-baae-30e8-a37f-624e50e8ad66",
                "7258cf00-6e98-3d3b-836f-220f47716e14",
            ]
        );
    }
}

#[test]
fn logger_with_known_missing_parent_is_not_attributed_to_an_unrelated_fill() {
    let parent = instruction_data(PLACE_MARKET_ORDER_TAG, &[]);
    let bytes = event(1, 10);
    let lengths = instruction_data(
        LOG_EVENT_LENGTHS_INSTRUCTION_TAG,
        &lengths_payload(0, &[bytes.len() as u16]),
    );
    let log = instruction_data(LOG_INSTRUCTION_TAG, &log_payload(0, &[&bytes]));
    let parsed = parse_events_from_executed_instructions_with_errors(
        [
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &parent,
                stack_path: &[0],
            },
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &lengths,
                stack_path: &[1, 0],
            },
            ExecutedInstructionView {
                program_id: PHOENIX,
                data: &log,
                stack_path: &[1, 1],
            },
        ],
        PHOENIX,
    );
    assert_eq!(parsed.orphan_logs.len(), 2);
    assert!(parsed.instructions[0].events.is_empty());
}
