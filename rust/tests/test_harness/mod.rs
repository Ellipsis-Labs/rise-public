#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use base64::Engine;
use litesvm::LiteSVM;
use litesvm::types::{FailedTransactionMetadata, TransactionMetadata};
use phoenix_rise::types::{
    AuthoritySetView, ExchangeKeysView, ExchangeMarketConfig, ExchangeRiskFactors, ExchangeView,
    MarketStatus, PhoenixMetadata,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub const DEFAULT_SDK_LOCALNET_FIXTURE_JSON: &str =
    include_str!("../../../test-fixtures/default-localnet.json");
pub const REQUIRED_PROGRAM_ARTIFACT_ENV: &str = "RISE_SDK_LOCALNET_REQUIRE_PROGRAMS";
pub const PHOENIX_REPO_ROOT_ENV: &str = "PHOENIX_REPO_ROOT";
pub const ETERNAL_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_ETERNAL_SO";
pub const EMBER_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_EMBER_SO";
pub const HAWKEYE_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_HAWKEYE_SO";
pub const FLIGHT_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_FLIGHT_SO";

const MICRO_FEE_DENOMINATOR: f64 = 1_000_000.0;

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
    #[serde(default)]
    pub default_taker_fee_micro: u32,
    #[serde(default)]
    pub default_maker_fee_micro: i32,
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

pub fn parse_pubkey(value: &str) -> Result<Pubkey, SdkLocalnetFixtureError> {
    Pubkey::from_str(value).map_err(|source| SdkLocalnetFixtureError::InvalidPubkey {
        value: value.to_string(),
        reason: source.to_string(),
    })
}

pub fn parse_pubkeys(values: &[String]) -> Vec<Pubkey> {
    values
        .iter()
        .map(|value| parse_pubkey(value).expect("fixture pubkey should parse"))
        .collect()
}

pub fn phoenix_metadata_from_fixture(fixture: &SdkLocalnetFixture) -> PhoenixMetadata {
    let payer = fixture
        .signers
        .iter()
        .find(|signer| signer.seed == "payer")
        .expect("payer signer should exist")
        .pubkey
        .clone();
    let authorities = AuthoritySetView {
        root_authority: payer.clone(),
        risk_authority: payer.clone(),
        market_authority: payer.clone(),
        oracle_authority: payer.clone(),
    };
    let keys = ExchangeKeysView {
        program_id: None,
        global_config: fixture.addresses.global_config.clone(),
        current_authorities: authorities.clone(),
        pending_authorities: authorities,
        canonical_mint: fixture
            .mints
            .iter()
            .find(|mint| mint.name == "phoenixCollateral")
            .map(|mint| mint.pubkey.clone())
            .unwrap_or_else(|| fixture.addresses.global_vault.clone()),
        global_vault: fixture.addresses.global_vault.clone(),
        perp_asset_map: fixture.addresses.perp_asset_map.clone(),
        global_trader_index: fixture.addresses.global_trader_index.clone(),
        active_trader_buffer: fixture.addresses.active_trader_buffer.clone(),
        withdraw_queue: fixture.addresses.withdraw_queue.clone(),
    };
    let markets = fixture
        .markets
        .iter()
        .enumerate()
        .map(|(asset_id, market)| {
            (
                market.symbol.clone(),
                ExchangeMarketConfig {
                    symbol: market.symbol.clone(),
                    asset_id: asset_id as u32,
                    market_status: MarketStatus::Active,
                    metadata: None,
                    market_pubkey: market.orderbook.clone(),
                    spline_pubkey: market.spline.clone(),
                    tick_size: market.tick_size_in_quote_lots_per_base_lot,
                    base_lots_decimals: market.base_lot_decimals,
                    taker_fee: micro_fee_to_rate(market.default_taker_fee_micro),
                    maker_fee: signed_micro_fee_to_rate(market.default_maker_fee_micro),
                    leverage_tiers: Vec::new(),
                    risk_factors: ExchangeRiskFactors::default(),
                    funding_interval_seconds: 1,
                    funding_period_seconds: 1,
                    max_funding_rate_per_interval: 0.0,
                    open_interest_cap_base_lots: 0_u64.into(),
                    max_liquidation_size_base_lots: 0_u64.into(),
                    isolated_only: false,
                },
            )
        })
        .collect();

    PhoenixMetadata::new(ExchangeView { keys, markets })
}

fn micro_fee_to_rate(value: u32) -> f64 {
    value as f64 / MICRO_FEE_DENOMINATOR
}

fn signed_micro_fee_to_rate(value: i32) -> f64 {
    value as f64 / MICRO_FEE_DENOMINATOR
}

#[derive(Clone, Debug)]
pub struct SdkLocalnetProgramPaths {
    pub phoenix_eternal: PathBuf,
    pub ember: PathBuf,
    pub hawkeye: Option<PathBuf>,
    pub flight: Option<PathBuf>,
}

pub struct SdkLocalnetContext {
    pub fixture: SdkLocalnetFixture,
    pub svm: LiteSVM,
    signers_by_seed: HashMap<String, Keypair>,
    signer_seed_by_pubkey: HashMap<Pubkey, String>,
}

impl SdkLocalnetContext {
    pub fn new(fixture: SdkLocalnetFixture, program_paths: SdkLocalnetProgramPaths) -> Self {
        let mut svm = LiteSVM::new();
        load_program(
            &mut svm,
            &fixture.programs.phoenix_eternal,
            &program_paths.phoenix_eternal,
        );
        load_program(&mut svm, &fixture.programs.ember, &program_paths.ember);
        if let Some(hawkeye) = program_paths.hawkeye.as_ref() {
            load_program(
                &mut svm,
                &phoenix_rise::HAWKEYE_PROGRAM_ID.to_string(),
                hawkeye,
            );
        }
        if let Some(flight) = program_paths.flight.as_ref() {
            load_program(
                &mut svm,
                &phoenix_rise::phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID.to_string(),
                flight,
            );
        }

        let mut signers_by_seed = HashMap::new();
        let mut signer_seed_by_pubkey = HashMap::new();
        for signer_config in &fixture.signers {
            let keypair = Keypair::new_from_array(sdk_localnet_seed_bytes(
                &fixture.keypair_base_seed,
                &signer_config.seed,
            ));
            assert_eq!(
                keypair.pubkey(),
                parse_pubkey(&signer_config.pubkey).expect("fixture pubkey should parse"),
                "derived signer {} should match fixture pubkey",
                signer_config.name
            );
            signer_seed_by_pubkey.insert(keypair.pubkey(), signer_config.seed.clone());
            signers_by_seed.insert(signer_config.seed.clone(), keypair);
        }

        for signer_config in &fixture.signers {
            if signer_config.initial_lamports == 0 {
                continue;
            }
            let signer = signers_by_seed
                .get(&signer_config.seed)
                .expect("signer should exist");
            svm.airdrop(&signer.pubkey(), signer_config.initial_lamports)
                .unwrap_or_else(|error| panic!("airdrop:{} failed: {error:?}", signer_config.name));
        }

        Self {
            fixture,
            svm,
            signers_by_seed,
            signer_seed_by_pubkey,
        }
    }

    pub fn load_program_from_path(&mut self, program_id: &Pubkey, path: &Path) {
        let bytes = std::fs::read(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        self.svm
            .add_program(*program_id, &bytes)
            .unwrap_or_else(|error| panic!("failed to load {}: {error:?}", path.display()));
    }

    pub fn execute_setup(&mut self) {
        for transaction in self.fixture.setup_transactions.clone() {
            self.send_fixture_transaction(&transaction.name);
        }
    }

    pub fn add_signer(&mut self, seed: &str, initial_lamports: u64) -> Pubkey {
        let keypair = Keypair::new_from_array(sdk_localnet_seed_bytes(
            &self.fixture.keypair_base_seed,
            seed,
        ));
        let pubkey = keypair.pubkey();
        self.signer_seed_by_pubkey.insert(pubkey, seed.to_string());
        self.signers_by_seed.insert(seed.to_string(), keypair);
        if initial_lamports > 0 {
            self.svm
                .airdrop(&pubkey, initial_lamports)
                .unwrap_or_else(|error| panic!("airdrop:{seed} failed: {error:?}"));
        }
        pubkey
    }

    pub fn send_fixture_transaction(&mut self, name: &str) {
        let transaction = self
            .fixture
            .setup_transactions
            .iter()
            .chain(self.fixture.action_templates.iter())
            .find(|transaction| transaction.name == name)
            .unwrap_or_else(|| panic!("missing fixture transaction {name}"))
            .clone();
        let fee_payer_seed = transaction
            .signer_seeds
            .iter()
            .find(|seed| seed.as_str() == "payer")
            .or_else(|| transaction.signer_seeds.first())
            .expect("fixture transaction should list signer seeds")
            .clone();

        for instruction in transaction.instructions {
            self.send_instructions(
                vec![decode_fixture_instruction(&instruction).unwrap()],
                &fee_payer_seed,
                &format!("{}:{}", transaction.name, instruction.name),
            );
        }
    }

    pub fn send_instructions(
        &mut self,
        instructions: Vec<Instruction>,
        fee_payer_seed: &str,
        label: &str,
    ) {
        self.send_instructions_with_metadata(instructions, fee_payer_seed, label);
    }

    pub fn send_instructions_with_metadata(
        &mut self,
        instructions: Vec<Instruction>,
        fee_payer_seed: &str,
        label: &str,
    ) -> TransactionMetadata {
        self.try_send_instructions_with_metadata(instructions, fee_payer_seed)
            .unwrap_or_else(|error| {
                panic!(
                    "LiteSVM transaction failed for {label}: {:?}\n{}",
                    error.err,
                    error.meta.logs.join("\n")
                )
            })
    }

    pub fn try_send_instructions_with_metadata(
        &mut self,
        instructions: Vec<Instruction>,
        fee_payer_seed: &str,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let fee_payer = self
            .signers_by_seed
            .get(fee_payer_seed)
            .unwrap_or_else(|| panic!("missing fee payer signer {fee_payer_seed}"));
        let mut signer_pubkeys = HashSet::new();
        let mut signers = Vec::new();
        signer_pubkeys.insert(fee_payer.pubkey());
        signers.push(fee_payer);

        for instruction in &instructions {
            for account in &instruction.accounts {
                if !account.is_signer || signer_pubkeys.contains(&account.pubkey) {
                    continue;
                }
                let seed = self
                    .signer_seed_by_pubkey
                    .get(&account.pubkey)
                    .unwrap_or_else(|| panic!("missing signer for {}", account.pubkey));
                let signer = self
                    .signers_by_seed
                    .get(seed)
                    .unwrap_or_else(|| panic!("missing signer seed {seed}"));
                signer_pubkeys.insert(account.pubkey);
                signers.push(signer);
            }
        }

        self.svm.expire_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&fee_payer.pubkey()),
            &signers,
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx)
    }

    pub fn signer_pubkey(&self, seed: &str) -> Pubkey {
        self.signers_by_seed
            .get(seed)
            .unwrap_or_else(|| panic!("missing signer {seed}"))
            .pubkey()
    }

    pub fn actor(&self, name: &str) -> FixtureActor {
        self.fixture
            .actors
            .iter()
            .find(|actor| actor.name == name)
            .unwrap_or_else(|| panic!("missing actor {name}"))
            .clone()
    }

    pub fn actor_pubkey(&self, name: &str) -> Pubkey {
        parse_pubkey(&self.actor(name).pubkey).expect("actor pubkey should parse")
    }

    pub fn actor_trader(&self, name: &str) -> Pubkey {
        parse_pubkey(&self.actor(name).trader_account).expect("actor trader should parse")
    }

    pub fn market(&self, symbol: &str) -> FixtureMarket {
        self.fixture
            .markets
            .iter()
            .find(|market| market.symbol == symbol)
            .unwrap_or_else(|| panic!("missing market {symbol}"))
            .clone()
    }

    pub fn account_data(&self, address: &Pubkey) -> Vec<u8> {
        self.svm
            .get_account(address)
            .unwrap_or_else(|| panic!("missing account {address}"))
            .data
    }
}

pub fn sdk_localnet_vm_required() -> bool {
    matches!(
        std::env::var(REQUIRED_PROGRAM_ARTIFACT_ENV).as_deref(),
        Ok("1") | Ok("true")
    )
}

pub fn find_sdk_localnet_program_paths() -> Option<SdkLocalnetProgramPaths> {
    let explicit = match (
        std::env::var(ETERNAL_PROGRAM_ENV),
        std::env::var(EMBER_PROGRAM_ENV),
        std::env::var(HAWKEYE_PROGRAM_ENV),
        std::env::var(FLIGHT_PROGRAM_ENV),
    ) {
        (Ok(phoenix_eternal), Ok(ember), hawkeye, flight) => Some(SdkLocalnetProgramPaths {
            phoenix_eternal: PathBuf::from(phoenix_eternal),
            ember: PathBuf::from(ember),
            hawkeye: hawkeye.ok().map(PathBuf::from),
            flight: flight.ok().map(PathBuf::from),
        }),
        _ => None,
    };
    if explicit.as_ref().is_some_and(program_paths_exist) {
        return explicit;
    }

    for root in default_program_roots() {
        for program_paths in [
            SdkLocalnetProgramPaths {
                phoenix_eternal: root.join("programs/target/deploy/phoenix_eternal.so"),
                ember: root.join("programs/target/deploy/phoenix_ember_program.so"),
                hawkeye: optional_program_path(
                    root.join("programs/target/deploy/phoenix_hawkeye.so"),
                ),
                flight: optional_program_path(
                    root.join("programs/target/deploy/phoenix_flight.so"),
                ),
            },
            SdkLocalnetProgramPaths {
                phoenix_eternal: root.join("target/deploy/phoenix_eternal.so"),
                ember: root.join("target/deploy/phoenix_ember_program.so"),
                hawkeye: optional_program_path(root.join("target/deploy/phoenix_hawkeye.so")),
                flight: optional_program_path(root.join("target/deploy/phoenix_flight.so")),
            },
            SdkLocalnetProgramPaths {
                phoenix_eternal: root.join("programs/eternal/target/deploy/phoenix_eternal.so"),
                ember: root.join("programs/ember/target/deploy/phoenix_ember_program.so"),
                hawkeye: optional_program_path(
                    root.join("programs/phoenix-hawkeye/target/deploy/phoenix_hawkeye.so"),
                ),
                flight: optional_program_path(
                    root.join("programs/flight/target/deploy/phoenix_flight.so"),
                ),
            },
        ] {
            if program_paths_exist(&program_paths) {
                return Some(program_paths);
            }
        }
    }

    None
}

fn default_program_roots() -> Vec<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![
        manifest_dir.join("../.."),
        std::env::current_dir().unwrap_or_else(|_| manifest_dir.clone()),
    ];
    if let Ok(root) = std::env::var(PHOENIX_REPO_ROOT_ENV) {
        roots.push(PathBuf::from(root));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn program_paths_exist(paths: &SdkLocalnetProgramPaths) -> bool {
    paths.phoenix_eternal.exists()
        && paths.ember.exists()
        && match paths.hawkeye.as_ref() {
            Some(hawkeye) => hawkeye.exists(),
            None => true,
        }
        && match paths.flight.as_ref() {
            Some(flight) => flight.exists(),
            None => true,
        }
}

fn optional_program_path(path: PathBuf) -> Option<PathBuf> {
    path.exists().then_some(path)
}

fn load_program(svm: &mut LiteSVM, program_id: &str, path: &Path) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    svm.add_program(
        parse_pubkey(program_id).expect("program id should parse"),
        &bytes,
    )
    .unwrap_or_else(|error| panic!("failed to load {}: {error:?}", path.display()));
}
