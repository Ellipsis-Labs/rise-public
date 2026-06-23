use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use base64::Engine;
use borsh::{BorshSerialize, to_vec};
use litesvm::LiteSVM;
use litesvm::types::{FailedTransactionMetadata, TransactionMetadata};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

use crate::types::{
    AuthoritySetView, ExchangeKeysView, ExchangeMarketConfig, ExchangeRiskFactors, ExchangeView,
    MarketStatus, PhoenixMetadata,
};

pub const DEFAULT_SDK_LOCALNET_FIXTURE_JSON: &str =
    include_str!("../../test-fixtures/default-localnet.json");
pub const REQUIRED_PROGRAM_ARTIFACT_ENV: &str = "RISE_SDK_LOCALNET_REQUIRE_PROGRAMS";
pub const PHOENIX_REPO_ROOT_ENV: &str = "PHOENIX_REPO_ROOT";
pub const ETERNAL_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_ETERNAL_SO";
pub const EMBER_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_EMBER_SO";
pub const HAWKEYE_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_HAWKEYE_SO";
pub const FLIGHT_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_FLIGHT_SO";
pub const PHOENIX_MAINNET_BPF_PROGRAMS_ENV: &str = "PHOENIX_MAINNET_BPF_PROGRAMS";
pub const PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR_ENV: &str = "PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR";
pub const PHOENIX_MAINNET_RPC_URL_ENV: &str = "PHOENIX_MAINNET_RPC_URL";
pub const PHOENIX_RPC_URL_ENV: &str = "PHOENIX_RPC_URL";
pub const DEFAULT_MAINNET_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    solana_pubkey::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
const MAINNET_BPF_CACHE_FINGERPRINT: &str = "mainnet-bpf-programs.fingerprint";
const MAINNET_BPF_CACHE_LOCK: &str = ".mainnet-bpf-programs.lock";
const MAINNET_BPF_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const MAINNET_BPF_CACHE_WAIT_TIMEOUT: Duration = Duration::from_secs(300);
const PROGRAMDATA_METADATA_LEN: usize = 45;
const SPL_TOKEN_MINT_TO_DISCRIMINANT: u8 = 7;
const FIXTURE_PRICE_EXPONENT: u8 = 3;
const FIXTURE_ORACLE_UPDATE_TIMESTAMP_BASE: u64 = 1_900_000_000;
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

impl FixtureMarket {
    pub fn price_usd_to_ticks(&self, price_usd: f64, quote_decimals: u8) -> u64 {
        self.price_to_ticks(FixturePrice::from_usd(price_usd), quote_decimals)
    }

    pub fn price_to_ticks(&self, price: FixturePrice, quote_decimals: u8) -> u64 {
        let scaled_value = if price.expo < quote_decimals {
            let factor = 10_u64
                .checked_pow((quote_decimals - price.expo) as u32)
                .expect("price scale factor should not overflow");
            price
                .value
                .checked_mul(factor)
                .expect("scaled price should not overflow")
        } else {
            let factor = 10_u64
                .checked_pow((price.expo - quote_decimals) as u32)
                .expect("price scale factor should not overflow");
            price.value / factor
        };

        if self.base_lot_decimals >= 0 {
            let base_lots_per_unit = 10_u64
                .checked_pow(self.base_lot_decimals as u32)
                .expect("base-lot scale should not overflow");
            scaled_value
                .checked_div(
                    self.tick_size_in_quote_lots_per_base_lot
                        .checked_mul(base_lots_per_unit)
                        .expect("tick denominator should not overflow"),
                )
                .expect("tick denominator should not be zero")
        } else {
            let base_units_per_lot = 10_u64
                .checked_pow((-self.base_lot_decimals) as u32)
                .expect("base-lot scale should not overflow");
            scaled_value
                .checked_mul(base_units_per_lot)
                .expect("tick numerator should not overflow")
                .checked_div(self.tick_size_in_quote_lots_per_base_lot)
                .expect("tick denominator should not be zero")
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshSerialize)]
pub struct FixturePrice {
    pub value: u64,
    pub expo: u8,
}

impl FixturePrice {
    pub fn new(value: u64, expo: u8) -> Self {
        assert!(value > 0, "fixture price must be positive");
        Self { value, expo }
    }

    pub fn from_usd(price_usd: f64) -> Self {
        assert!(
            price_usd.is_finite() && price_usd > 0.0,
            "fixture USD price must be a positive finite value"
        );
        let scale = 10_u64.pow(FIXTURE_PRICE_EXPONENT as u32) as f64;
        let value = (price_usd * scale).round();
        assert!(
            value > 0.0 && value <= u64::MAX as f64,
            "fixture USD price is out of range"
        );
        Self::new(value as u64, FIXTURE_PRICE_EXPONENT)
    }
}

#[derive(Clone, Debug)]
pub struct FixtureOraclePriceUpdate {
    pub symbol: String,
    pub new_exchange_perp_price: Option<FixturePrice>,
    pub new_exchange_spot_price: FixturePrice,
}

impl FixtureOraclePriceUpdate {
    pub fn new(
        symbol: impl Into<String>,
        new_exchange_perp_price: Option<FixturePrice>,
        new_exchange_spot_price: FixturePrice,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            new_exchange_perp_price,
            new_exchange_spot_price,
        }
    }

    pub fn mark_and_spot(symbol: impl Into<String>, price: FixturePrice) -> Self {
        Self::new(symbol, Some(price), price)
    }

    pub fn mark_and_spot_usd(symbol: impl Into<String>, price_usd: f64) -> Self {
        Self::mark_and_spot(symbol, FixturePrice::from_usd(price_usd))
    }
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
                    stats_snapshot: None,
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

#[derive(Clone, Debug)]
pub struct SdkLocalnetProgram {
    pub program_id: Pubkey,
    pub path: PathBuf,
}

impl SdkLocalnetProgram {
    pub fn new(program_id: Pubkey, path: impl Into<PathBuf>) -> Self {
        Self {
            program_id,
            path: path.into(),
        }
    }
}

pub struct SdkLocalnetContext {
    pub fixture: SdkLocalnetFixture,
    pub svm: LiteSVM,
    signers_by_seed: HashMap<String, Keypair>,
    signer_seed_by_pubkey: HashMap<Pubkey, String>,
    oracle_update_sequence: u64,
}

impl SdkLocalnetContext {
    pub fn new(fixture: SdkLocalnetFixture, program_paths: SdkLocalnetProgramPaths) -> Self {
        Self::new_with_programs(fixture, program_paths, [])
    }

    pub fn new_with_programs(
        fixture: SdkLocalnetFixture,
        program_paths: SdkLocalnetProgramPaths,
        extra_programs: impl IntoIterator<Item = SdkLocalnetProgram>,
    ) -> Self {
        let mut svm = LiteSVM::new();

        if should_load_mainnet_bpf_programs() && !running_in_ci() {
            load_mainnet_protocol_programs(&mut svm, &fixture);
        } else {
            load_program(
                &mut svm,
                &fixture.programs.phoenix_eternal,
                &program_paths.phoenix_eternal,
            );
            load_program(&mut svm, &fixture.programs.ember, &program_paths.ember);
            if let Some(hawkeye) = program_paths.hawkeye.as_ref() {
                load_program(
                    &mut svm,
                    &crate::phoenix_rise_ix::HAWKEYE_PROGRAM_ID.to_string(),
                    hawkeye,
                );
            }
            if let Some(flight) = program_paths.flight.as_ref() {
                load_program(
                    &mut svm,
                    &crate::phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID.to_string(),
                    flight,
                );
            }
        }

        for program in extra_programs {
            load_program(&mut svm, &program.program_id.to_string(), &program.path);
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
            oracle_update_sequence: 1,
        }
    }

    pub fn load_program_from_path(&mut self, program_id: &Pubkey, path: &Path) {
        let bytes = std::fs::read(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        self.svm
            .add_program(*program_id, &bytes)
            .unwrap_or_else(|error| panic!("failed to load {}: {error:?}", path.display()));
    }

    pub fn load_program(&mut self, program: SdkLocalnetProgram) {
        self.load_program_from_path(&program.program_id, &program.path);
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

    pub fn init_flight(&mut self, max_fee_cap_bps: u64) {
        let global_state = crate::phoenix_rise_ix::flight::get_flight_global_state_address();
        if self.svm.get_account(&global_state).is_some() {
            return;
        }
        self.send_instructions(
            vec![flight_init_ix(self.signer_pubkey("payer"), max_fee_cap_bps)],
            "payer",
            "flight-init",
        );
    }

    pub fn register_flight_builder(&mut self, builder: &FixtureActor, fee_bps: u64) {
        let ix = crate::phoenix_rise_ix::flight::create_register_builder_ix(
            crate::phoenix_rise_ix::flight::RegisterBuilderParams::builder()
                .trader_authority(parse_pubkey(&builder.pubkey).unwrap())
                .trader_account(parse_pubkey(&builder.trader_account).unwrap())
                .subaccount_index(builder.subaccount_index)
                .fee_bps(fee_bps)
                .build()
                .unwrap(),
        )
        .unwrap()
        .into();
        self.send_instructions(vec![ix], &builder.seed, "flight-register-builder");
    }

    pub fn setup_flight_builder(
        &mut self,
        actor_name: &str,
        max_fee_cap_bps: u64,
        fee_bps: u64,
    ) -> FixtureActor {
        self.init_flight(max_fee_cap_bps);
        let actor = self.actor(actor_name);
        self.register_flight_builder(&actor, fee_bps);
        actor
    }

    pub fn move_market_price_usd(&mut self, symbol: &str, price_usd: f64) {
        let price = FixturePrice::from_usd(price_usd);
        let quote_decimals = self.fixture_mint("fakeUsdc").decimals;
        let market = self.market(symbol);
        let price_ticks = market.price_to_ticks(price, quote_decimals);

        self.update_oracle_prices(&[FixtureOraclePriceUpdate::mark_and_spot(symbol, price)]);
        self.update_spline_price_ticks(symbol, price_ticks);
    }

    pub fn update_oracle_prices(&mut self, updates: &[FixtureOraclePriceUpdate]) {
        let update_timestamp = self.next_fixture_oracle_update_timestamp();
        self.update_oracle_prices_at(update_timestamp, updates);
    }

    pub fn update_oracle_prices_at(
        &mut self,
        update_timestamp: u64,
        updates: &[FixtureOraclePriceUpdate],
    ) {
        assert!(
            !updates.is_empty(),
            "fixture oracle price update requires at least one price"
        );
        let mut ix = self.fixture_instruction("oracleSetPrices", "updateOraclePrices");
        ix.data = oracle_price_update_data(update_timestamp, updates);
        self.send_instructions(vec![ix], "price-oracle", "oracle-update-prices");
    }

    pub fn update_spline_price_ticks(&mut self, symbol: &str, new_mid_price_ticks: u64) {
        assert!(
            new_mid_price_ticks > 0,
            "fixture spline price must be positive"
        );
        let instruction_name = format!("update{}SplinePrice", symbol.to_ascii_uppercase());
        let mut ix = self.fixture_instruction("splineUpdatePrices", &instruction_name);
        ix.data = spline_price_update_data(new_mid_price_ticks);
        self.send_instructions(
            vec![ix],
            "spline-trader",
            &format!("spline-update-{}-price", symbol.to_ascii_lowercase()),
        );
    }

    pub fn airdrop_usdc_to_actor(&mut self, actor_name: &str, amount_atoms: u64) {
        let actor = self.actor(actor_name);
        self.mint_fake_usdc_to_token_account(
            parse_pubkey(&actor.fake_usdc_token_account).unwrap(),
            amount_atoms,
        );
    }

    pub fn mint_fake_usdc_to_token_account(&mut self, token_account: Pubkey, amount_atoms: u64) {
        let mint = self.fixture_mint("fakeUsdc");
        let mint_pubkey = parse_pubkey(&mint.pubkey).unwrap();
        let authority = self.signer_pubkey("payer");
        let mut data = Vec::with_capacity(9);
        data.push(SPL_TOKEN_MINT_TO_DISCRIMINANT);
        data.extend_from_slice(&amount_atoms.to_le_bytes());

        self.send_instructions(
            vec![Instruction {
                program_id: crate::phoenix_rise_ix::SPL_TOKEN_PROGRAM_ID,
                accounts: vec![
                    AccountMeta::new(mint_pubkey, false),
                    AccountMeta::new(token_account, false),
                    AccountMeta::new_readonly(authority, true),
                ],
                data,
            }],
            "payer",
            "mint-fake-usdc",
        );
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

    fn fixture_instruction(&self, transaction_name: &str, instruction_name: &str) -> Instruction {
        let transaction = self
            .fixture
            .setup_transactions
            .iter()
            .chain(self.fixture.action_templates.iter())
            .find(|transaction| transaction.name == transaction_name)
            .unwrap_or_else(|| panic!("missing fixture transaction {transaction_name}"));
        let instruction = transaction
            .instructions
            .iter()
            .find(|instruction| instruction.name == instruction_name)
            .unwrap_or_else(|| {
                panic!("missing fixture instruction {transaction_name}:{instruction_name}")
            });
        decode_fixture_instruction(instruction).unwrap()
    }

    fn next_fixture_oracle_update_timestamp(&mut self) -> u64 {
        let timestamp = FIXTURE_ORACLE_UPDATE_TIMESTAMP_BASE + self.oracle_update_sequence;
        self.oracle_update_sequence += 1;
        timestamp
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

    pub fn fixture_mint(&self, name: &str) -> FixtureMint {
        self.fixture
            .mints
            .iter()
            .find(|mint| mint.name == name)
            .unwrap_or_else(|| panic!("missing fixture mint {name}"))
            .clone()
    }

    pub fn fake_usdc_mint(&self) -> Pubkey {
        parse_pubkey(&self.fixture_mint("fakeUsdc").pubkey).expect("fake USDC mint should parse")
    }

    pub fn phoenix_collateral_mint(&self) -> Pubkey {
        parse_pubkey(&self.fixture_mint("phoenixCollateral").pubkey)
            .expect("Phoenix collateral mint should parse")
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

    pub fn token_amount(&self, token_account: &Pubkey) -> u64 {
        let data = self.account_data(token_account);
        u64::from_le_bytes(data[64..72].try_into().unwrap())
    }
}

pub fn sdk_localnet_vm_required() -> bool {
    matches!(
        std::env::var(REQUIRED_PROGRAM_ARTIFACT_ENV).as_deref(),
        Ok("1") | Ok("true")
    )
}

pub fn mainnet_bpf_programs_enabled() -> bool {
    should_load_mainnet_bpf_programs() && !running_in_ci()
}

pub fn find_sdk_localnet_program_paths() -> Option<SdkLocalnetProgramPaths> {
    if mainnet_bpf_programs_enabled() {
        return Some(SdkLocalnetProgramPaths {
            phoenix_eternal: PathBuf::new(),
            ember: PathBuf::new(),
            hawkeye: None,
            flight: None,
        });
    }

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

fn should_load_mainnet_bpf_programs() -> bool {
    matches!(
        std::env::var(PHOENIX_MAINNET_BPF_PROGRAMS_ENV).as_deref(),
        Ok("1") | Ok("true")
    )
}

fn running_in_ci() -> bool {
    matches!(std::env::var("CI").as_deref(), Ok("1") | Ok("true"))
}

fn mainnet_rpc_url() -> String {
    std::env::var(PHOENIX_MAINNET_RPC_URL_ENV)
        .or_else(|_| std::env::var(PHOENIX_RPC_URL_ENV))
        .unwrap_or_else(|_| DEFAULT_MAINNET_RPC_URL.to_string())
}

fn load_mainnet_protocol_programs(svm: &mut LiteSVM, fixture: &SdkLocalnetFixture) {
    let specs = mainnet_protocol_programs(fixture);
    let cache_dir = mainnet_bpf_program_cache_dir();
    ensure_mainnet_program_cache(&cache_dir, &specs);

    for spec in specs {
        load_program_id(
            svm,
            spec.program_id,
            &mainnet_cached_program_path(&cache_dir, &spec),
        );
    }
}

#[derive(Clone, Copy, Debug)]
struct MainnetProgramSpec {
    name: &'static str,
    program_id: Pubkey,
    file_name: &'static str,
}

fn mainnet_protocol_programs(fixture: &SdkLocalnetFixture) -> Vec<MainnetProgramSpec> {
    vec![
        MainnetProgramSpec {
            name: "phoenix-eternal",
            program_id: parse_pubkey(&fixture.programs.phoenix_eternal)
                .expect("Phoenix program id should parse"),
            file_name: "phoenix_eternal-mainnet.so",
        },
        MainnetProgramSpec {
            name: "ember",
            program_id: parse_pubkey(&fixture.programs.ember)
                .expect("Ember program id should parse"),
            file_name: "phoenix_ember_program-mainnet.so",
        },
        MainnetProgramSpec {
            name: "hawkeye",
            program_id: crate::phoenix_rise_ix::HAWKEYE_PROGRAM_ID,
            file_name: "phoenix_hawkeye-mainnet.so",
        },
        MainnetProgramSpec {
            name: "flight",
            program_id: crate::phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID,
            file_name: "phoenix_flight-mainnet.so",
        },
    ]
}

pub fn mainnet_bpf_program_cache_dir() -> PathBuf {
    if let Ok(cache_dir) = std::env::var(PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR_ENV) {
        return PathBuf::from(cache_dir);
    }
    if let Ok(root) = std::env::var(PHOENIX_REPO_ROOT_ENV) {
        return PathBuf::from(root).join("target/deploy/.cache");
    }
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(target_dir).join("deploy/.cache");
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.join("../..");
    if repo_root.join("rise/rust/Cargo.toml").exists() {
        return repo_root.join("target/deploy/.cache");
    }

    if let Ok(current_dir) = std::env::current_dir() {
        if let Some(cargo_root) = find_cargo_target_root(&current_dir) {
            return cargo_root.join("target/deploy/.cache");
        }
    }
    if let Some(cargo_root) = find_cargo_target_root(&manifest_dir) {
        return cargo_root.join("target/deploy/.cache");
    }

    std::env::current_dir()
        .unwrap_or(manifest_dir)
        .join("target/deploy/.cache")
}

fn find_cargo_target_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    let mut first_cargo_manifest = None;

    loop {
        if current.join("Cargo.lock").exists() || current.join("target").exists() {
            return Some(current);
        }
        if current.join("Cargo.toml").exists() && first_cargo_manifest.is_none() {
            first_cargo_manifest = Some(current.clone());
        }
        if !current.pop() {
            break;
        }
    }

    first_cargo_manifest
}

fn ensure_mainnet_program_cache(cache_dir: &Path, specs: &[MainnetProgramSpec]) {
    fs::create_dir_all(cache_dir).unwrap_or_else(|error| {
        panic!("create mainnet BPF cache {}: {error}", cache_dir.display())
    });
    if mainnet_program_cache_is_fresh(cache_dir, specs) {
        return;
    }

    match try_acquire_mainnet_bpf_cache_lock(cache_dir) {
        Ok(Some(_guard)) => {
            if !mainnet_program_cache_is_fresh(cache_dir, specs) {
                fetch_mainnet_program_cache(cache_dir, specs);
            }
        }
        Ok(None) => wait_for_mainnet_program_cache(cache_dir, specs),
        Err(error) => panic!(
            "acquire mainnet BPF cache lock in {}: {error}",
            cache_dir.display()
        ),
    }
}

fn mainnet_program_cache_is_fresh(cache_dir: &Path, specs: &[MainnetProgramSpec]) -> bool {
    if specs
        .iter()
        .any(|spec| !mainnet_cached_program_path(cache_dir, spec).exists())
    {
        return false;
    }

    let fingerprint_path = cache_dir.join(MAINNET_BPF_CACHE_FINGERPRINT);
    let fingerprint = match fs::read_to_string(&fingerprint_path) {
        Ok(fingerprint) => fingerprint,
        Err(_) => return false,
    };
    if specs.iter().any(|spec| {
        !fingerprint.contains(spec.name) || !fingerprint.contains(&spec.program_id.to_string())
    }) {
        return false;
    }

    let modified = match fs::metadata(&fingerprint_path).and_then(|metadata| metadata.modified()) {
        Ok(modified) => modified,
        Err(_) => return false,
    };
    SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default()
        < MAINNET_BPF_CACHE_TTL
}

fn wait_for_mainnet_program_cache(cache_dir: &Path, specs: &[MainnetProgramSpec]) {
    let started = Instant::now();
    while started.elapsed() < MAINNET_BPF_CACHE_WAIT_TIMEOUT {
        if mainnet_program_cache_is_fresh(cache_dir, specs) {
            return;
        }
        let lock_path = cache_dir.join(MAINNET_BPF_CACHE_LOCK);
        if !lock_path.exists() || mainnet_bpf_cache_lock_is_stale(&lock_path) {
            match try_acquire_mainnet_bpf_cache_lock(cache_dir) {
                Ok(Some(_guard)) => {
                    if !mainnet_program_cache_is_fresh(cache_dir, specs) {
                        fetch_mainnet_program_cache(cache_dir, specs);
                    }
                    return;
                }
                Ok(None) => {}
                Err(error) => panic!(
                    "acquire mainnet BPF cache lock in {}: {error}",
                    cache_dir.display()
                ),
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    panic!(
        "timed out waiting for mainnet BPF cache in {}",
        cache_dir.display()
    );
}

struct MainnetBpfCacheLock {
    path: PathBuf,
}

impl Drop for MainnetBpfCacheLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn try_acquire_mainnet_bpf_cache_lock(
    cache_dir: &Path,
) -> std::io::Result<Option<MainnetBpfCacheLock>> {
    let lock_path = cache_dir.join(MAINNET_BPF_CACHE_LOCK);
    match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&lock_path)
    {
        Ok(mut file) => {
            let _ = writeln!(
                file,
                "pid={}\nstarted={}",
                std::process::id(),
                unix_timestamp()
            );
            Ok(Some(MainnetBpfCacheLock { path: lock_path }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if mainnet_bpf_cache_lock_is_stale(&lock_path) {
                let _ = fs::remove_file(&lock_path);
                return try_acquire_mainnet_bpf_cache_lock(cache_dir);
            }
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn mainnet_bpf_cache_lock_is_stale(lock_path: &Path) -> bool {
    fs::metadata(lock_path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age > MAINNET_BPF_CACHE_WAIT_TIMEOUT)
}

fn fetch_mainnet_program_cache(cache_dir: &Path, specs: &[MainnetProgramSpec]) {
    let cache_dir = cache_dir.to_path_buf();
    let specs = specs.to_vec();
    let handle = thread::Builder::new()
        .name("rise-mainnet-bpf-fetch".to_string())
        .spawn(move || fetch_mainnet_program_cache_inner(&cache_dir, &specs))
        .expect("spawn mainnet BPF fetch thread");

    if let Err(panic) = handle.join() {
        std::panic::resume_unwind(panic);
    }
}

fn fetch_mainnet_program_cache_inner(cache_dir: &Path, specs: &[MainnetProgramSpec]) {
    let rpc_url = mainnet_rpc_url();
    let client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());
    let mut fingerprint = format!(
        "fetched_at_unix_secs={}\nrpc_url={rpc_url}\n",
        unix_timestamp()
    );

    for spec in specs {
        let bytes = fetch_mainnet_upgradeable_program_bytes(&client, spec);
        write_atomic(&mainnet_cached_program_path(cache_dir, spec), &bytes);
        fingerprint.push_str(&format!(
            "program={} id={} file={} bytes={}\n",
            spec.name,
            spec.program_id,
            spec.file_name,
            bytes.len()
        ));
    }

    write_atomic(
        &cache_dir.join(MAINNET_BPF_CACHE_FINGERPRINT),
        fingerprint.as_bytes(),
    );
}

fn fetch_mainnet_upgradeable_program_bytes(
    client: &RpcClient,
    spec: &MainnetProgramSpec,
) -> Vec<u8> {
    let programdata_address =
        Pubkey::find_program_address(&[spec.program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID).0;
    let programdata_account = client
        .get_account(&programdata_address)
        .unwrap_or_else(|error| {
            panic!(
                "fetch mainnet programdata {} for {} ({}): {error}",
                programdata_address, spec.name, spec.program_id
            )
        });
    if programdata_account.data.len() <= PROGRAMDATA_METADATA_LEN {
        panic!(
            "mainnet programdata {} for {} is too small",
            programdata_address, spec.name
        );
    }
    programdata_account.data[PROGRAMDATA_METADATA_LEN..].to_vec()
}

fn mainnet_cached_program_path(cache_dir: &Path, spec: &MainnetProgramSpec) -> PathBuf {
    cache_dir.join(spec.file_name)
}

fn write_atomic(path: &Path, data: &[u8]) {
    let file_name = path
        .file_name()
        .unwrap_or_else(|| panic!("cache path should have a file name: {}", path.display()))
        .to_string_lossy();
    let tmp = path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()));
    fs::write(&tmp, data).unwrap_or_else(|error| panic!("write {}: {error}", tmp.display()));
    fs::rename(&tmp, path)
        .unwrap_or_else(|error| panic!("rename {} to {}: {error}", tmp.display(), path.display()));
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(BorshSerialize)]
struct FixtureSymbol {
    symbol_bytes: [u8; 16],
}

impl FixtureSymbol {
    fn new(symbol: &str) -> Self {
        let bytes = symbol.as_bytes();
        assert!(
            !bytes.is_empty() && bytes.len() <= 16,
            "fixture symbol must be between 1 and 16 bytes"
        );
        assert!(
            bytes.iter().all(|byte| byte.is_ascii() && *byte != 0),
            "fixture symbol must be non-null ASCII"
        );
        let mut symbol_bytes = [0_u8; 16];
        symbol_bytes[..bytes.len()].copy_from_slice(bytes);
        Self { symbol_bytes }
    }
}

#[derive(BorshSerialize)]
struct SerializedOraclePriceUpdate {
    perp_asset_id: FixtureSymbol,
    new_exchange_perp_price: Option<FixturePrice>,
    new_exchange_spot_price: FixturePrice,
}

#[derive(BorshSerialize)]
struct SerializedOraclePriceUpdateFlags {
    flags: u8,
}

#[derive(BorshSerialize)]
struct SerializedOraclePriceUpdateInstruction {
    update_timestamp: u64,
    updates: Vec<SerializedOraclePriceUpdate>,
    flags: SerializedOraclePriceUpdateFlags,
}

#[derive(BorshSerialize)]
struct SerializedUpdateSplinePriceParams {
    new_mid_price: u64,
    user_update_slot: Option<u64>,
    refresh_regions: bool,
}

fn oracle_price_update_data(
    update_timestamp: u64,
    updates: &[FixtureOraclePriceUpdate],
) -> Vec<u8> {
    let payload = SerializedOraclePriceUpdateInstruction {
        update_timestamp,
        updates: updates
            .iter()
            .map(|update| SerializedOraclePriceUpdate {
                perp_asset_id: FixtureSymbol::new(&update.symbol),
                new_exchange_perp_price: update.new_exchange_perp_price,
                new_exchange_spot_price: update.new_exchange_spot_price,
            })
            .collect(),
        flags: SerializedOraclePriceUpdateFlags { flags: 0 },
    };
    let mut data =
        crate::phoenix_rise_ix::compute_discriminant("global:update_oracle_prices_with_ordering")
            .to_vec();
    data.extend_from_slice(&to_vec(&payload).expect("oracle price update should serialize"));
    data
}

fn spline_price_update_data(new_mid_price_ticks: u64) -> Vec<u8> {
    let payload = SerializedUpdateSplinePriceParams {
        new_mid_price: new_mid_price_ticks,
        user_update_slot: None,
        refresh_regions: true,
    };
    let mut data =
        crate::phoenix_rise_ix::compute_discriminant("global:update_spline_price").to_vec();
    data.extend_from_slice(&to_vec(&payload).expect("spline price update should serialize"));
    data
}

fn flight_init_ix(payer: Pubkey, max_fee_cap_bps: u64) -> Instruction {
    let mut data = crate::phoenix_rise_ix::compute_discriminant("global:init").to_vec();
    data.extend_from_slice(&max_fee_cap_bps.to_le_bytes());
    Instruction {
        program_id: crate::phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(
                crate::phoenix_rise_ix::flight::get_flight_global_state_address(),
                false,
            ),
            AccountMeta::new_readonly(*crate::phoenix_rise_ix::PHOENIX_PROGRAM_ID, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(crate::phoenix_rise_ix::SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

fn load_program(svm: &mut LiteSVM, program_id: &str, path: &Path) {
    load_program_id(
        svm,
        parse_pubkey(program_id).expect("program id should parse"),
        path,
    );
}

fn load_program_id(svm: &mut LiteSVM, program_id: Pubkey, path: &Path) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    svm.add_program(program_id, &bytes)
        .unwrap_or_else(|error| panic!("failed to load {}: {error:?}", path.display()));
}
