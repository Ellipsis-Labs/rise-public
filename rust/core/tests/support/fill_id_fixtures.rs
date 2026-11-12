use phoenix_rise_core::events::{ExecutedInstructionView, advance_inner_path};
use serde::Deserialize;
use solana_pubkey::Pubkey;

#[derive(Deserialize)]
pub struct RecordedInstruction {
    pub instruction_index: u8,
    pub stack_height: Option<u32>,
    pub program_id: String,
    pub data_base58: String,
}

#[derive(Deserialize)]
pub struct RecordedTransaction {
    pub signature: String,
    pub slot: u64,
    #[serde(default)]
    pub top_level_instructions: Vec<RecordedInstruction>,
    pub inner_instructions: Vec<RecordedInstruction>,
}

pub struct ExecutedInstruction {
    program_id: Pubkey,
    data: Vec<u8>,
    stack_path: Vec<u8>,
}

impl ExecutedInstruction {
    fn decode(instruction: &RecordedInstruction, stack_path: Vec<u8>) -> Self {
        Self {
            program_id: instruction.program_id.parse().unwrap(),
            data: bs58::decode(&instruction.data_base58).into_vec().unwrap(),
            stack_path,
        }
    }

    pub fn view(&self) -> ExecutedInstructionView<'_> {
        ExecutedInstructionView {
            program_id: self.program_id.as_ref(),
            data: &self.data,
            stack_path: &self.stack_path,
        }
    }
}

impl RecordedTransaction {
    pub fn execution_order(&self) -> Vec<ExecutedInstruction> {
        let mut instructions = Vec::new();
        for top in &self.top_level_instructions {
            instructions.push(ExecutedInstruction::decode(
                top,
                vec![top.instruction_index],
            ));
            let mut path = Vec::new();
            for inner in &self.inner_instructions {
                if inner.instruction_index != top.instruction_index {
                    continue;
                }
                advance_inner_path(&mut path, inner.stack_height);
                let mut full_path = vec![top.instruction_index];
                full_path.extend_from_slice(&path);
                instructions.push(ExecutedInstruction::decode(inner, full_path));
            }
        }
        instructions
    }
}
