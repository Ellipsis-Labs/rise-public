use phoenix_rise_core::transaction_errors::parse_transaction_error;
use serde::Deserialize;
use serde_json::{Value, json};
use solana_instruction::Instruction;
use solana_transaction_error::TransactionError;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    name: String,
    instructions: Vec<FixtureInstruction>,
    error: TransactionError,
    logs: Option<Vec<String>>,
    instruction_name: Option<String>,
    failure: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureInstruction {
    program_address: String,
    data: Vec<u8>,
}

#[test]
fn failure_context_uses_the_rejecting_program_and_survives_missing_evidence() {
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../test-fixtures/transaction-errors.json"
    ))
    .expect("transaction failure fixtures should parse");
    for case in cases {
        let instructions: Vec<_> = case
            .instructions
            .into_iter()
            .map(|ix| Instruction {
                program_id: ix
                    .program_address
                    .parse()
                    .expect("fixture program address is valid"),
                accounts: Vec::new(),
                data: ix.data,
            })
            .collect();
        let parsed = parse_transaction_error(&instructions, &case.error, case.logs.as_deref())
            .unwrap_or_else(|| panic!("{} lost its instruction error", case.name));
        if let TransactionError::InstructionError(index, expected) = &case.error {
            assert_eq!(parsed.instruction_index, *index, "{}", case.name);
            assert_eq!(&parsed.error, expected, "{}", case.name);
        }
        assert_eq!(
            parsed
                .instruction
                .and_then(|instruction| instruction.instruction_name),
            case.instruction_name.as_deref(),
            "{}",
            case.name,
        );
        let failure = parsed
            .failure
            .map(|failure| {
                json!({
                    "programAddress": failure.instruction.program_id.to_string(),
                    "programName": failure.instruction.program_name,
                    "instructionName": failure.instruction.instruction_name,
                    "errorCode": failure.error_code,
                    "errorName": failure.error_name,
                    "message": failure.message,
                    "hint": failure.hint.map(|hint| hint.to_string()),
                })
            })
            .unwrap_or(Value::Null);
        let mut expected = case.failure;
        if let Some(expected) = expected.as_object_mut() {
            expected.entry("hint").or_insert(Value::Null);
        }
        assert_eq!(failure, expected, "{}", case.name);
    }
    assert_eq!(
        parse_transaction_error(&[], &TransactionError::BlockhashNotFound, None),
        None,
    );
}
