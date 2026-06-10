#![allow(dead_code)]

use std::str::FromStr;

use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

pub const DEFAULT_SDK_LOCALNET_FIXTURE_JSON: &str =
    include_str!("../../../test-fixtures/default-localnet.json");

#[derive(Debug, thiserror::Error)]
pub enum SdkLocalnetFixtureError {
    #[error("invalid pubkey `{value}`: {reason}")]
    InvalidPubkey { value: String, reason: String },
    #[error("invalid base64 data for instruction `{instruction}`: {source}")]
    InvalidInstructionData {
        instruction: String,
        source: base64::DecodeError,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkLocalnetFixture {
    pub schema_version: u32,
    pub name: String,
    pub keypair_base_seed: String,
    pub keypair_derivation: String,
    pub programs: ProgramAddresses,
    pub addresses: FixtureAddresses,
    pub mints: Vec<FixtureMint>,
    pub signers: Vec<FixtureSigner>,
    pub markets: Vec<FixtureMarket>,
    pub actors: Vec<FixtureActor>,
    pub setup_transactions: Vec<FixtureTransaction>,
    pub action_templates: Vec<FixtureTransaction>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramAddresses {
    pub phoenix_eternal: String,
    pub ember: String,
    pub spl_token: String,
    pub associated_token: String,
    pub system: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureAddresses {
    pub global_config: String,
    pub log_authority: String,
    pub perp_asset_map: String,
    pub withdraw_queue: String,
    pub global_vault: String,
    pub global_trader_index: Vec<String>,
    pub active_trader_buffer: Vec<String>,
    pub ember_state: String,
    pub ember_vault: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureMint {
    pub name: String,
    pub seed: String,
    pub pubkey: String,
    pub decimals: u8,
    pub role: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureSigner {
    pub name: String,
    pub role: String,
    pub seed: String,
    pub pubkey: String,
    pub initial_lamports: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureMarket {
    pub symbol: String,
    pub orderbook: String,
    pub spline: String,
    pub signer_seed: String,
    pub base_lot_decimals: i8,
    pub tick_size_in_quote_lots_per_base_lot: u64,
    pub initial_mark_price_atoms: u64,
    pub initial_spot_price_atoms: u64,
    pub liquidity: FixtureLiquidity,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureLiquidity {
    pub bids: Vec<FixtureLiquidityLevel>,
    pub asks: Vec<FixtureLiquidityLevel>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureLiquidityLevel {
    pub price_usd: String,
    pub base_quantity: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureActor {
    pub name: String,
    pub signer: String,
    pub seed: String,
    pub pubkey: String,
    pub trader_account: String,
    pub fake_usdc_token_account: String,
    pub phoenix_token_account: String,
    pub subaccount_index: u8,
    pub initial_usdc_atoms: u64,
    pub phoenix_deposit_atoms: u64,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureTransaction {
    pub name: String,
    pub description: String,
    pub signer_seeds: Vec<String>,
    pub instructions: Vec<SerializedInstruction>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedInstruction {
    pub name: String,
    pub program_id: String,
    pub accounts: Vec<SerializedAccountMeta>,
    pub data_base64: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

pub fn default_sdk_localnet_fixture() -> Result<SdkLocalnetFixture, serde_json::Error> {
    serde_json::from_str(DEFAULT_SDK_LOCALNET_FIXTURE_JSON)
}

pub fn sdk_localnet_seed_bytes(keypair_base_seed: &str, seed: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(format!("{keypair_base_seed}-{seed}"));
    hasher.finalize().into()
}

pub fn default_sdk_localnet_seed_bytes(seed: &str) -> [u8; 32] {
    sdk_localnet_seed_bytes("rise-sdk-localnet-v1", seed)
}

pub fn decode_fixture_instruction(
    instruction: &SerializedInstruction,
) -> Result<Instruction, SdkLocalnetFixtureError> {
    let program_id = parse_pubkey(&instruction.program_id)?;
    let accounts = instruction
        .accounts
        .iter()
        .map(|account| {
            Ok(AccountMeta {
                pubkey: parse_pubkey(&account.pubkey)?,
                is_signer: account.is_signer,
                is_writable: account.is_writable,
            })
        })
        .collect::<Result<Vec<_>, SdkLocalnetFixtureError>>()?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(&instruction.data_base64)
        .map_err(|source| SdkLocalnetFixtureError::InvalidInstructionData {
            instruction: instruction.name.clone(),
            source,
        })?;

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

fn parse_pubkey(value: &str) -> Result<Pubkey, SdkLocalnetFixtureError> {
    Pubkey::from_str(value).map_err(|source| SdkLocalnetFixtureError::InvalidPubkey {
        value: value.to_string(),
        reason: source.to_string(),
    })
}
