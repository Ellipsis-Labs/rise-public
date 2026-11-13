//! Best-effort debugging context for failed transactions and simulations.

use phoenix_rise_ix::diagnostics::{
    InstructionFailure, InstructionInfo, decode_runtime_failure, identify_instruction,
};
use solana_instruction::Instruction;
use solana_instruction::error::InstructionError;
use solana_pubkey::Pubkey;
use solana_transaction_error::TransactionError;

/// The rejected top-level instruction and, when logs align, the program
/// failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionFailure {
    pub instruction_index: u8,
    pub instruction: Option<InstructionInfo>,
    /// The original Solana error is preserved even when hints cannot be
    /// decoded.
    pub error: InstructionError,
    /// A nested failure is a propagation hint: a caller can catch and later
    /// return the same error. Its instruction name is absent without CPI data.
    pub failure: Option<InstructionFailure>,
}

/// Parses an RPC/simulation error using resolved top-level instructions.
///
/// Returns `None` for transaction-wide errors such as an expired blockhash.
/// Missing, truncated, or mismatched logs leave `failure` absent;
/// custom codes are never attributed to a caller without invocation evidence.
pub fn parse_transaction_error(
    instructions: &[Instruction],
    error: &TransactionError,
    logs: Option<&[String]>,
) -> Option<TransactionFailure> {
    let TransactionError::InstructionError(index, error) = error else {
        return None;
    };
    let instruction = instructions.get(usize::from(*index));
    let failure = logs.and_then(|logs| {
        let instruction = instruction?;
        let roots = frames(logs)?;
        if roots.len() != usize::from(*index) + 1
            || roots
                .iter()
                .zip(instructions)
                .any(|(frame, instruction)| frame.program_id != instruction.program_id)
            || roots[..usize::from(*index)]
                .iter()
                .any(|frame| frame.reason.is_some())
        {
            return None;
        }
        let mut frame = roots.last()?;
        let reason = frame.reason?;
        let code = custom_code(reason);
        if match error {
            InstructionError::Custom(expected) => code != Some(*expected),
            _ => code.is_some() || reason != error.to_string(),
        } {
            return None;
        }
        let mut nested = false;
        while let Some(child) = frame
            .children
            .iter()
            .rev()
            .find(|child| child.reason == frame.reason)
        {
            frame = child;
            nested = true;
        }
        let data = if nested { &[][..] } else { &instruction.data };
        decode_runtime_failure(&frame.program_id, data, reason, frame.logs.iter().copied())
    });
    Some(TransactionFailure {
        instruction_index: *index,
        instruction: instruction.map(|ix| identify_instruction(&ix.program_id, &ix.data)),
        error: error.clone(),
        failure,
    })
}

fn custom_code(reason: &str) -> Option<u32> {
    u32::from_str_radix(reason.strip_prefix("custom program error: 0x")?, 16).ok()
}

struct Frame<'a> {
    program_id: Pubkey,
    reason: Option<&'a str>,
    logs: Vec<&'a str>,
    children: Vec<Frame<'a>>,
}

/// A complete balanced trace is required before any frame is used as evidence.
fn frames(logs: &[String]) -> Option<Vec<Frame<'_>>> {
    let mut roots = Vec::new();
    let mut stack: Vec<Frame<'_>> = Vec::new();
    for line in logs {
        if line.starts_with("Program log:") || line.starts_with("Program data:") {
            stack.last_mut()?.logs.push(line);
            continue;
        }
        if let Some(rest) = line.strip_prefix("Program ") {
            if let Some((id, depth)) = rest.split_once(" invoke [") {
                let depth: usize = depth.strip_suffix(']')?.parse().ok()?;
                if depth != stack.len() + 1 || depth > 64 {
                    return None;
                }
                stack.push(Frame {
                    program_id: id.parse().ok()?,
                    reason: None,
                    logs: Vec::new(),
                    children: Vec::new(),
                });
                continue;
            }
            let exit = rest
                .strip_suffix(" success")
                .map(|id| (id, None))
                .or_else(|| {
                    rest.split_once(" failed: ")
                        .map(|(id, reason)| (id, Some(reason)))
                });
            if let Some((id, reason)) = exit {
                let mut frame = stack.pop()?;
                if id.parse::<Pubkey>().ok()? != frame.program_id || reason == Some("") {
                    return None;
                }
                frame.reason = reason;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(frame);
                } else {
                    roots.push(frame);
                }
                continue;
            }
            if let Some((id, _)) = rest.split_once(" consumed ") {
                if id.parse::<Pubkey>().ok()? != stack.last()?.program_id {
                    return None;
                }
                continue;
            }
        }
        stack.last_mut()?.logs.push(line);
    }
    stack.is_empty().then_some(roots)
}
