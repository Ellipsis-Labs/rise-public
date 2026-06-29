#![cfg(feature = "events")]

use phoenix_rise_core::events::{
    ExecutedInstructionView, LOG_EVENT_LENGTHS_INSTRUCTION_TAG, LOG_INSTRUCTION_TAG,
    RpcInnerInstructionGroup, RpcInnerInstructionView, RpcInstructionView, advance_inner_path,
    parse_events_from_executed_instructions_with_errors,
    parse_events_from_rpc_instructions_with_errors,
};
use phoenix_rise_events::MarketEvent;
use phoenix_rise_events::market_events::SlotContextEvent;

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
