//! IDL-backed debugging hints for program errors and instruction
//! discriminators.

use solana_pubkey::Pubkey;

use crate::constants::{BETA_PHOENIX_PROGRAM_ID, PROD_PHOENIX_PROGRAM_ID};

/// A custom error defined by a program's IDL. Messages retain `<field>`
/// placeholders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgramErrorInfo {
    pub code: u32,
    pub name: &'static str,
    pub message: &'static str,
}

/// The identity of an instruction, without decoding its arguments.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstructionInfo {
    pub program_id: Pubkey,
    pub program_name: Option<&'static str>,
    pub instruction_name: Option<&'static str>,
}

/// Debugging context for an instruction that returned an error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstructionFailure {
    pub instruction: InstructionInfo,
    /// `None` for Solana built-in instruction errors.
    pub error_code: Option<u32>,
    pub error_name: Option<&'static str>,
    pub message: String,
    /// Additional context established by the failing invocation's own logs.
    pub hint: Option<InstructionFailureHint>,
}

/// A diagnostic hint; this does not replace the program's error code or name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InstructionFailureHint {
    ConditionalOrdersAccountUninitialized,
}

impl std::fmt::Display for InstructionFailureHint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConditionalOrdersAccountUninitialized => {
                f.write_str("conditionalOrdersAccountUninitialized")
            }
        }
    }
}

/// Pass the runtime reason and logs belonging only to the returning invocation.
pub fn identify_instruction_failure_hint<'a>(
    program_id: &Pubkey,
    reason: &str,
    logs: impl IntoIterator<Item = &'a str>,
) -> Option<InstructionFailureHint> {
    if metadata(program_id)?.name != "Eternal" || reason != "invalid account data for instruction" {
        return None;
    }
    logs.into_iter().find_map(|line| {
        let detail = line.strip_prefix("Program log: ")?;
        let detail = if let Some(quoted) = detail.strip_prefix('"') {
            quoted.split_once("\"\n")?.0
        } else {
            detail
        };
        let detail = detail.strip_prefix("Invalid account data length for ")?;
        let (account_type, sizes) = detail.split_once(". Size: ")?;
        if account_type.rsplit("::").next()? != "ConditionalOrderHeader" {
            return None;
        }
        let expected = sizes.strip_prefix("0, Expected: ")?.parse::<usize>().ok()?;
        (expected > 0).then_some(InstructionFailureHint::ConditionalOrdersAccountUninitialized)
    })
}

struct ProgramMetadata {
    id: Pubkey,
    name: &'static str,
    instructions: &'static [(&'static [u8], &'static str)],
    errors: &'static [(u32, &'static str, &'static str)],
}

macro_rules! program_catalog {
    ($($id:literal => $name:literal {
        instructions { $($discriminator:expr => $instruction:literal,)* }
        errors { $($code:literal => ($error:literal, $message:literal),)* }
    })*) => {
        const PROGRAMS: &[ProgramMetadata] = &[$(ProgramMetadata {
            id: solana_pubkey::pubkey!($id),
            name: $name,
            instructions: &[$((&$discriminator, $instruction)),*],
            errors: &[$(($code, $error, $message)),*],
        }),*];
    };
}

include!("catalog.rs");

fn metadata(program_id: &Pubkey) -> Option<&'static ProgramMetadata> {
    let id = if *program_id == BETA_PHOENIX_PROGRAM_ID {
        &PROD_PHOENIX_PROGRAM_ID
    } else {
        program_id
    };
    PROGRAMS.iter().find(|program| program.id == *id)
}

/// Looks up a custom error only in the IDL of the program that returned it.
/// Unknown programs and codes return `None`; no log prose is classified.
pub fn decode_program_error(program_id: &Pubkey, code: u32) -> Option<ProgramErrorInfo> {
    let &(_, name, message) = metadata(program_id)?
        .errors
        .iter()
        .find(|(candidate, ..)| *candidate == code)?;
    Some(ProgramErrorInfo {
        code,
        name,
        message,
    })
}

/// Names an instruction from its program ID and its IDL discriminator bytes.
/// Unknown programs, short data, and unknown discriminators keep their identity
/// but have no decoded name.
pub fn identify_instruction(program_id: &Pubkey, data: &[u8]) -> InstructionInfo {
    let program = metadata(program_id);
    let instruction_name = program.and_then(|program| {
        program
            .instructions
            .iter()
            .filter(|(candidate, _)| data.starts_with(candidate))
            .max_by_key(|(candidate, _)| candidate.len())
            .map(|(_, name)| *name)
    });
    InstructionInfo {
        program_id: *program_id,
        program_name: program.map(|program| program.name),
        instruction_name,
    }
}

/// Decodes a custom error for its *originating* instruction. If a caller
/// propagated a CPI error, pass the CPI instruction rather than the caller.
/// Unknown codes retain their number and a generic message.
pub fn decode_instruction_error(program_id: &Pubkey, data: &[u8], code: u32) -> InstructionFailure {
    let error = decode_program_error(program_id, code);
    InstructionFailure {
        instruction: identify_instruction(program_id, data),
        error_code: Some(code),
        error_name: error.map(|error| error.name),
        message: error
            .map(|error| error.message.to_string())
            .unwrap_or_else(|| format!("Custom program error: 0x{code:x}")),
        hint: None,
    }
}

/// Decodes a failure already attributed by an invocation parser. `logs` must
/// belong only to that invocation; this function does not validate a trace.
pub fn decode_runtime_failure<'a>(
    program_id: &Pubkey,
    data: &[u8],
    reason: &str,
    logs: impl IntoIterator<Item = &'a str>,
) -> Option<InstructionFailure> {
    let mut failure = if let Some(code) = reason.strip_prefix("custom program error: 0x") {
        decode_instruction_error(program_id, data, u32::from_str_radix(code, 16).ok()?)
    } else {
        InstructionFailure {
            instruction: identify_instruction(program_id, data),
            error_code: None,
            error_name: None,
            message: reason.to_string(),
            hint: None,
        }
    };
    let prefix = failure
        .error_name
        .map(|name| format!("Program log: {name}: "));
    for line in logs {
        if let Some(prefix) = prefix.as_deref()
            && let Some(message) = line.strip_prefix(prefix)
        {
            failure.message = message.to_string();
        }
        if failure.hint.is_none() {
            failure.hint = identify_instruction_failure_hint(program_id, reason, [line]);
        }
    }
    Some(failure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::EMBER_PROGRAM_ID;
    use crate::discriminants::PhoenixInstruction;

    #[test]
    fn margin_errors_are_not_self_trade_errors_or_errors_from_another_program() {
        let data = PhoenixInstruction::PlaceMarketOrder.discriminant();
        let failure = decode_instruction_error(&PROD_PHOENIX_PROGRAM_ID, &data, 0x5001);
        assert_eq!(
            failure.instruction.instruction_name,
            Some("PlaceMarketOrder")
        );
        assert_eq!(failure.instruction.program_name, Some("Eternal"));
        assert_eq!(failure.error_name, Some("MarginInsufficient"));
        assert_eq!(failure.message, "Insufficient margin");
        assert_eq!(decode_program_error(&EMBER_PROGRAM_ID, 0x5001), None);
        assert_eq!(decode_program_error(&Pubkey::new_unique(), 0x5001), None);
        assert_eq!(
            decode_program_error(&BETA_PHOENIX_PROGRAM_ID, 0x5001),
            decode_program_error(&PROD_PHOENIX_PROGRAM_ID, 0x5001),
        );

        let unknown = decode_instruction_error(&PROD_PHOENIX_PROGRAM_ID, &[1, 2], u32::MAX);
        assert_eq!(unknown.instruction.instruction_name, None);
        assert_eq!(unknown.error_code, Some(u32::MAX));
        assert_eq!(unknown.error_name, None);
    }

    #[test]
    fn sidecar_errors_and_short_discriminators_keep_their_program_identity() {
        let flicker = solana_pubkey::pubkey!("FLickryJ5sBSyBRj7sSzMnsP93jvPxEqNKjvG49q9CEU");
        assert_eq!(
            decode_program_error(&flicker, 8003).unwrap().name,
            "BboUnavailableRetry"
        );
        assert_eq!(decode_program_error(&PROD_PHOENIX_PROGRAM_ID, 8003), None);
        let hawkeye = solana_pubkey::pubkey!("RiSeVw3ZjNfsaXPRb4mgaqYaEEt41pNNJoDvVh7pgQj");
        assert_eq!(
            decode_program_error(&hawkeye, 0x2102).unwrap().name,
            "MarketInvalidAssetId"
        );
        assert_eq!(
            decode_program_error(&hawkeye, 0x3001).unwrap().name,
            "OracleMarkPriceUnavailable"
        );
        assert_eq!(
            decode_program_error(&EMBER_PROGRAM_ID, 6000).unwrap().name,
            "Unauthorized"
        );
        let punchcard = solana_pubkey::pubkey!("pcWKVSdcdDUKabPz4pVfaQ2jMod1kWv3LqeQivjKXiF");
        assert_eq!(
            identify_instruction(&punchcard, &[0]).instruction_name,
            Some("Create")
        );
        assert_eq!(identify_instruction(&punchcard, &[]).instruction_name, None);
    }
}
