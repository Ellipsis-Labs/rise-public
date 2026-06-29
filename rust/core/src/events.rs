//! Helpers for attaching Phoenix log events to executed transaction
//! instructions.

pub use phoenix_rise_events::{
    LOG_EVENT_LENGTHS_INSTRUCTION_TAG, LOG_HEADER_LEN, LOG_INSTRUCTION_TAG, LogParseError,
    LogParseFailure, PhoenixLogInstructionKind as LogInstructionKind,
};
use phoenix_rise_events::{
    MarketEvent, ParsedLogEvents, PhoenixLogInstruction, PhoenixLogInstructionKind,
    parse as parse_log_events, parse_with_errors as parse_log_events_with_errors,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutedInstructionView<'a> {
    pub program_id: &'a [u8],
    pub data: &'a [u8],
    /// Full execution path. Top-level instructions are `[top_level_index]`.
    /// Inner instructions append sibling indices at each stack depth.
    pub stack_path: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RpcInstructionView<'a> {
    pub program_id: &'a [u8],
    pub data: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RpcInnerInstructionView<'a> {
    pub program_id: &'a [u8],
    pub data: &'a [u8],
    /// Solana RPC `stackHeight`: 1 for top-level, 2 for first CPI depth, etc.
    pub stack_height: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RpcInnerInstructionGroup<'a> {
    pub index: u8,
    pub instructions: &'a [RpcInnerInstructionView<'a>],
}

#[derive(Clone, Debug)]
pub struct ParsedInstructionEvents<'a> {
    pub instruction_tag: u64,
    pub instruction_data: &'a [u8],
    pub stack_path: Vec<u8>,
    pub events: Vec<MarketEvent>,
    pub skipped_event_bytes: Vec<&'a [u8]>,
    pub failures: Vec<LogParseFailure<'a>>,
}

#[derive(Clone, Debug)]
pub struct OrphanLogInstruction<'a> {
    pub stack_path: Vec<u8>,
    pub instruction: PhoenixLogInstruction<'a>,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedTransactionEvents<'a> {
    pub instructions: Vec<ParsedInstructionEvents<'a>>,
    pub orphan_logs: Vec<OrphanLogInstruction<'a>>,
}

struct WorkItem<'a> {
    instruction_tag: u64,
    instruction_data: &'a [u8],
    stack_path: Vec<u8>,
    logs: Vec<PhoenixLogInstruction<'a>>,
}

struct EventParser<'a, 'program> {
    phoenix_program_id: &'program [u8],
    work_items: Vec<WorkItem<'a>>,
    orphan_logs: Vec<OrphanLogInstruction<'a>>,
}

impl<'a, 'program> EventParser<'a, 'program> {
    fn new(phoenix_program_id: &'program [u8]) -> Self {
        Self {
            phoenix_program_id,
            work_items: Vec::new(),
            orphan_logs: Vec::new(),
        }
    }

    fn push_instruction(&mut self, program_id: &'a [u8], data: &'a [u8], stack_path: &[u8]) {
        if program_id != self.phoenix_program_id {
            return;
        }

        let Some((tag_bytes, after_tag)) = data.split_first_chunk::<8>() else {
            return;
        };
        let tag = u64::from_le_bytes(*tag_bytes);

        if let Some(kind) = PhoenixLogInstructionKind::from_tag(tag) {
            self.push_log(PhoenixLogInstruction::new(kind, after_tag), stack_path);
        } else {
            self.work_items.push(WorkItem {
                instruction_tag: tag,
                instruction_data: data,
                stack_path: stack_path.to_vec(),
                logs: Vec::new(),
            });
        }
    }

    fn push_log(&mut self, instruction: PhoenixLogInstruction<'a>, stack_path: &[u8]) {
        if let Some(parent_path) = stack_path.split_last().map(|(_, parent)| parent)
            && let Some(item) = self.find_work_item_by_path(parent_path)
        {
            item.logs.push(instruction);
            return;
        }

        if let Some(item) = self.work_items.last_mut() {
            item.logs.push(instruction);
        } else {
            self.orphan_logs.push(OrphanLogInstruction {
                stack_path: stack_path.to_vec(),
                instruction,
            });
        }
    }

    fn find_work_item_by_path(&mut self, stack_path: &[u8]) -> Option<&mut WorkItem<'a>> {
        self.work_items
            .iter_mut()
            .rev()
            .find(|item| item.stack_path == stack_path)
    }

    fn finish(self, include_errors: bool) -> ParsedTransactionEvents<'a> {
        let instructions = self
            .work_items
            .into_iter()
            .map(|item| {
                let ParsedLogEvents {
                    events,
                    skipped_event_bytes,
                    failures,
                } = if include_errors {
                    parse_log_events_with_errors(item.logs)
                } else {
                    ParsedLogEvents {
                        events: parse_log_events(item.logs),
                        skipped_event_bytes: Vec::new(),
                        failures: Vec::new(),
                    }
                };

                ParsedInstructionEvents {
                    instruction_tag: item.instruction_tag,
                    instruction_data: item.instruction_data,
                    stack_path: item.stack_path,
                    events,
                    skipped_event_bytes,
                    failures,
                }
            })
            .collect();

        ParsedTransactionEvents {
            instructions,
            orphan_logs: self.orphan_logs,
        }
    }
}

pub fn parse_events_from_executed_instructions<'a, I>(
    instructions: I,
    phoenix_program_id: &[u8],
) -> Vec<ParsedInstructionEvents<'a>>
where
    I: IntoIterator<Item = ExecutedInstructionView<'a>>,
{
    let mut parser = EventParser::new(phoenix_program_id);
    for instruction in instructions {
        parser.push_instruction(
            instruction.program_id,
            instruction.data,
            instruction.stack_path,
        );
    }
    parser.finish(false).instructions
}

pub fn parse_events_from_executed_instructions_with_errors<'a, I>(
    instructions: I,
    phoenix_program_id: &[u8],
) -> ParsedTransactionEvents<'a>
where
    I: IntoIterator<Item = ExecutedInstructionView<'a>>,
{
    let mut parser = EventParser::new(phoenix_program_id);
    for instruction in instructions {
        parser.push_instruction(
            instruction.program_id,
            instruction.data,
            instruction.stack_path,
        );
    }
    parser.finish(true)
}

pub fn parse_events_from_rpc_instructions<'a>(
    top_level_instructions: &[RpcInstructionView<'a>],
    inner_instruction_groups: &[RpcInnerInstructionGroup<'a>],
    phoenix_program_id: &[u8],
) -> Vec<ParsedInstructionEvents<'a>> {
    let mut parser = EventParser::new(phoenix_program_id);
    push_rpc_instructions(
        &mut parser,
        top_level_instructions,
        inner_instruction_groups,
    );
    parser.finish(false).instructions
}

pub fn parse_events_from_rpc_instructions_with_errors<'a>(
    top_level_instructions: &[RpcInstructionView<'a>],
    inner_instruction_groups: &[RpcInnerInstructionGroup<'a>],
    phoenix_program_id: &[u8],
) -> ParsedTransactionEvents<'a> {
    let mut parser = EventParser::new(phoenix_program_id);
    push_rpc_instructions(
        &mut parser,
        top_level_instructions,
        inner_instruction_groups,
    );
    parser.finish(true)
}

fn push_rpc_instructions<'a>(
    parser: &mut EventParser<'a, '_>,
    top_level_instructions: &[RpcInstructionView<'a>],
    inner_instruction_groups: &[RpcInnerInstructionGroup<'a>],
) {
    for (top_level_index, instruction) in top_level_instructions.iter().enumerate() {
        let top_level_path = [top_level_index as u8];
        parser.push_instruction(instruction.program_id, instruction.data, &top_level_path);

        for group in inner_instruction_groups
            .iter()
            .filter(|group| group.index as usize == top_level_index)
        {
            let mut current_path = Vec::new();
            for inner_instruction in group.instructions {
                advance_inner_path(&mut current_path, inner_instruction.stack_height);
                let mut stack_path = Vec::with_capacity(current_path.len() + 1);
                stack_path.push(group.index);
                stack_path.extend_from_slice(&current_path);
                parser.push_instruction(
                    inner_instruction.program_id,
                    inner_instruction.data,
                    &stack_path,
                );
            }
        }
    }
}

/// Reconstructs the sibling path of the next RPC inner instruction at its
/// reported stack depth.
pub fn advance_inner_path(current_path: &mut Vec<u8>, stack_height: Option<u32>) {
    let target_path_len = match stack_height {
        Some(height) if height > 1 => (height - 1) as usize,
        _ => 1,
    };

    if target_path_len > current_path.len() {
        while current_path.len() < target_path_len {
            current_path.push(0);
        }
    } else {
        current_path.truncate(target_path_len);
        if let Some(last) = current_path.last_mut() {
            *last = last.saturating_add(1);
        }
    }
}
