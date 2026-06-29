//! Fixture schema and deterministic localnet configuration.
//!
//! The default fixture is embedded from `test-fixtures/default-localnet.json`.
//! It models a compact Phoenix exchange:
//!
//! - `fakeUsdc` is a 6-decimal SPL mint controlled by the payer and used as the
//!   Ember input mint.
//! - `phoenixCollateral` is a 6-decimal Ember output mint used as Phoenix
//!   collateral.
//! - `priceOracle` is delegated to update Phoenix oracle prices.
//! - `orderbookTrader` owns the deterministic maker levels.
//! - `splineTrader` owns each market spline and signs spline price updates.
//! - `taker0`, `taker1`, and `stopLossTaker` are pre-funded traders with
//!   deposit, withdraw, limit, market, and risk-increase capabilities.
//!
//! Prices in the fixture use 3 decimal places. For example, BTC's
//! `initialMarkPriceAtoms` value of `100_000_000` represents 100,000.000 USD.
//! [`FixturePrice::from_usd`] uses the same scale for runtime price moves.

use std::str::FromStr;

use base64::Engine;
use borsh::BorshSerialize;
use phoenix_rise_api::PhoenixMetadata;
use phoenix_rise_api::types::prelude::{
    AuthoritySetView, ExchangeKeysView, ExchangeMarketConfig, ExchangeRiskFactors, ExchangeView,
    MarketStatus,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Embedded default localnet fixture JSON.
pub const DEFAULT_SDK_LOCALNET_FIXTURE_JSON: &str =
    include_str!("../test-fixtures/default-localnet.json");
const FIXTURE_PRICE_EXPONENT: u8 = 3;
const MICRO_FEE_DENOMINATOR: f64 = 1_000_000.0;

#[derive(Debug, thiserror::Error)]
/// Errors returned while decoding fixture addresses or serialized
/// instructions.
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
/// Complete deterministic localnet fixture used by [`SdkLocalnetContext`].
///
/// [`SdkLocalnetContext`]: crate::SdkLocalnetContext
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
/// Program ids used by the fixture.
pub struct ProgramAddresses {
    pub phoenix_eternal: String,
    pub ember: String,
    pub spl_token: String,
    pub associated_token: String,
    pub system: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Core Phoenix and Ember account addresses created by the fixture.
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
/// SPL mint configured by the fixture.
pub struct FixtureMint {
    pub name: String,
    pub seed: String,
    pub pubkey: String,
    pub decimals: u8,
    pub role: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Deterministic signer generated from the fixture's base seed.
pub struct FixtureSigner {
    pub name: String,
    pub role: String,
    pub seed: String,
    pub pubkey: String,
    pub initial_lamports: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Per-market localnet configuration.
///
/// The default fixture has three active markets:
///
/// - BTC: 100,000.000 USD mark and spot, 4 base-lot decimals, tick size 100,
///   maker bids at 99,500/99,000, and maker asks at 100,500/101,000.
/// - ETH: 3,500.000 USD mark and spot, 3 base-lot decimals, tick size 10, maker
///   bids at 3,485/3,475, and maker asks at 3,515/3,525.
/// - SOL: 150.000 USD mark and spot, 2 base-lot decimals, tick size 10, maker
///   bids at 149.5/149, and maker asks at 150.5/151.
///
/// Use [`FixtureMarket::price_usd_to_ticks`] to convert a test price into the
/// tick value expected by [`SdkLocalnetContext::update_spline_price_ticks`].
///
/// [`SdkLocalnetContext::update_spline_price_ticks`]: crate::SdkLocalnetContext::update_spline_price_ticks
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
/// Maker levels inserted by the `orderbookPlaceLevels` fixture transaction.
pub struct FixtureLiquidity {
    pub bids: Vec<FixtureLiquidityLevel>,
    pub asks: Vec<FixtureLiquidityLevel>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// One deterministic maker orderbook level.
pub struct FixtureLiquidityLevel {
    pub price_usd: String,
    pub base_quantity: String,
}

impl FixtureMarket {
    /// Convert a USD price into this market's Phoenix tick value.
    pub fn price_usd_to_ticks(&self, price_usd: f64, quote_decimals: u8) -> u64 {
        self.price_to_ticks(FixturePrice::from_usd(price_usd), quote_decimals)
    }

    /// Convert a fixture price into this market's Phoenix tick value.
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
/// Oracle or spline price in fixture atoms.
///
/// The default helper scale is 3 decimal places, so
/// `FixturePrice::from_usd(99_500.0)` serializes as `99_500_000`.
pub struct FixturePrice {
    pub value: u64,
    pub expo: u8,
}

impl FixturePrice {
    /// Construct a raw fixture price with an explicit decimal exponent.
    pub fn new(value: u64, expo: u8) -> Self {
        assert!(value > 0, "fixture price must be positive");
        Self { value, expo }
    }

    /// Construct a 3-decimal fixture price from a USD float.
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
/// Price update sent through the delegated fixture oracle.
///
/// Set `new_exchange_perp_price` to `None` when a test needs to update spot
/// only. Use [`FixtureOraclePriceUpdate::mark_and_spot_usd`] for the common
/// case where mark and spot move together.
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
/// Funded trader actor in the default fixture.
///
/// Each default actor starts with 25,000,000 fake USDC atoms and deposits
/// 10,000,000 Phoenix collateral atoms during setup. `orderbookTrader` is the
/// deterministic maker, `splineTrader` owns the market splines, and `taker0`,
/// `taker1`, and `stopLossTaker` are available for program scenarios.
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
/// Named fixture transaction that can be replayed by
/// [`SdkLocalnetContext::send_fixture_transaction`].
///
/// [`SdkLocalnetContext::send_fixture_transaction`]: crate::SdkLocalnetContext::send_fixture_transaction
pub struct FixtureTransaction {
    pub name: String,
    pub description: String,
    pub signer_seeds: Vec<String>,
    pub instructions: Vec<SerializedInstruction>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Serialized instruction in a fixture transaction.
pub struct SerializedInstruction {
    pub name: String,
    pub program_id: String,
    pub accounts: Vec<SerializedAccountMeta>,
    pub data_base64: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Serialized account metadata for a fixture instruction.
pub struct SerializedAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

/// Deserialize the embedded default localnet fixture.
pub fn default_sdk_localnet_fixture() -> Result<SdkLocalnetFixture, serde_json::Error> {
    serde_json::from_str(DEFAULT_SDK_LOCALNET_FIXTURE_JSON)
}

/// Derive a deterministic 32-byte seed from a fixture base seed and signer
/// seed.
pub fn sdk_localnet_seed_bytes(keypair_base_seed: &str, seed: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(format!("{keypair_base_seed}-{seed}"));
    hasher.finalize().into()
}

/// Derive a deterministic signer seed using the default fixture base seed.
pub fn default_sdk_localnet_seed_bytes(seed: &str) -> [u8; 32] {
    sdk_localnet_seed_bytes("rise-sdk-localnet-v1", seed)
}

/// Decode a fixture instruction into a Solana instruction.
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

/// Parse a fixture pubkey string.
pub fn parse_pubkey(value: &str) -> Result<Pubkey, SdkLocalnetFixtureError> {
    Pubkey::from_str(value).map_err(|source| SdkLocalnetFixtureError::InvalidPubkey {
        value: value.to_string(),
        reason: source.to_string(),
    })
}

/// Parse a list of fixture pubkey strings.
pub fn parse_pubkeys(values: &[String]) -> Vec<Pubkey> {
    values
        .iter()
        .map(|value| parse_pubkey(value).expect("fixture pubkey should parse"))
        .collect()
}

/// Build Rise API metadata from the localnet fixture.
///
/// This is useful for SDK/client tests that need exchange metadata without
/// querying the API service.
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
