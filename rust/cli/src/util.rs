use std::error::Error;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use phoenix_rise::api::{ApiInstructionResponse, default_solana_keypair_path};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{Keypair, read_keypair_file};
use solana_pubkey::Pubkey;
use solana_transaction::versioned::VersionedTransaction;

pub const ENV_AUTH_KEYPAIR_PATH: [&str; 2] = ["PHOENIX_AUTH_KEYPAIR_PATH", "SOLANA_KEYPAIR_PATH"];

pub fn parse_pubkey(input: &str) -> Result<Pubkey, Box<dyn Error>> {
    Pubkey::from_str(input).map_err(|e| format!("invalid pubkey '{input}': {e}").into())
}

pub fn read_wallet_keypair(path: Option<PathBuf>) -> Result<Keypair, Box<dyn Error>> {
    let keypair_path =
        path.or_else(|| read_first_trimmed_env(&ENV_AUTH_KEYPAIR_PATH).map(PathBuf::from));
    let path = match keypair_path {
        Some(path) => expand_home_path(path)?,
        None => default_solana_keypair_path()?,
    };

    read_keypair_file(&path).map_err(|error| {
        std::io::Error::other(format!(
            "failed to read Solana keypair at {}: {error}",
            path.display()
        ))
        .into()
    })
}

pub fn api_instruction_to_solana(
    api_instruction: &ApiInstructionResponse,
) -> Result<Instruction, Box<dyn Error>> {
    Ok(Instruction {
        program_id: Pubkey::from_str(&api_instruction.program_id)?,
        accounts: api_instruction
            .keys
            .iter()
            .map(|account| {
                Ok(AccountMeta {
                    pubkey: Pubkey::from_str(&account.pubkey)?,
                    is_signer: account.is_signer,
                    is_writable: account.is_writable,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
        data: api_instruction.data.clone(),
    })
}

pub fn encode_transaction(transaction: &VersionedTransaction) -> Result<String, Box<dyn Error>> {
    Ok(BASE64_STANDARD.encode(bincode::serialize(transaction)?))
}

pub fn expand_home_path(path: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    Ok(expand_home_path_with_home(path, &home_dir()?))
}

pub fn expand_home_path_with_home(path: PathBuf, home: &Path) -> PathBuf {
    let raw = path.as_os_str().to_string_lossy();
    if raw == "~" {
        return home.to_path_buf();
    }
    if let Some(stripped) = raw.strip_prefix("~/") {
        return home.join(stripped);
    }
    path
}

pub fn home_dir() -> Result<PathBuf, Box<dyn Error>> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("HOME is not set").into())
}

fn read_first_trimmed_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_tilde_in_paths() {
        let expanded =
            expand_home_path_with_home(PathBuf::from("~/keys/id.json"), Path::new("/tmp/home"));

        assert_eq!(expanded, PathBuf::from("/tmp/home/keys/id.json"));
    }
}
