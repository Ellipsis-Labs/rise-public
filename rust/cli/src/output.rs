use std::error::Error;
use std::fmt::Write as _;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use serde::Serialize;
use sha2::{Digest, Sha256};
use solana_instruction::Instruction;

#[derive(Debug, Clone, Copy)]
pub struct OutputOptions {
    pub json: bool,
    pub pretty: bool,
}

impl OutputOptions {
    pub fn compact_json(self) -> bool {
        self.json || !self.pretty
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountMetaOutput {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionOutput {
    pub program_id: String,
    pub accounts: Vec<AccountMetaOutput>,
    pub data: Vec<u8>,
    pub data_base64: String,
    pub data_sha256: String,
}

impl From<&Instruction> for InstructionOutput {
    fn from(ix: &Instruction) -> Self {
        Self {
            program_id: ix.program_id.to_string(),
            accounts: ix
                .accounts
                .iter()
                .map(|meta| AccountMetaOutput {
                    pubkey: meta.pubkey.to_string(),
                    is_signer: meta.is_signer,
                    is_writable: meta.is_writable,
                })
                .collect(),
            data: ix.data.clone(),
            data_base64: BASE64_STANDARD.encode(&ix.data),
            data_sha256: sha256_hex(&ix.data),
        }
    }
}

pub fn print_json<T: Serialize>(value: &T, output: OutputOptions) -> Result<(), Box<dyn Error>> {
    if output.compact_json() {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

pub fn print_human_json<T: Serialize>(
    value: &T,
    output: OutputOptions,
) -> Result<(), Box<dyn Error>> {
    if output.json {
        print_json(value, output)
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
        Ok(())
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}
