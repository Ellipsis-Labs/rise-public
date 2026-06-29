//! LiteSVM test fixtures for Rise program integration tests.
//!
//! This crate provides the deterministic localnet fixture used by the Rise
//! example programs and SDK integration tests. It loads Phoenix protocol SBF
//! programs into [`LiteSVM`](litesvm::LiteSVM), creates a fake USDC collateral
//! flow through Ember, initializes Phoenix markets, and exposes helpers for
//! moving oracle and spline prices during tests.
//!
//! The default fixture creates BTC, ETH, and SOL perpetual markets. Each market
//! starts with matching mark and spot oracle prices, has a registered spline
//! owner, and has deterministic maker orderbook levels owned by
//! `orderbookTrader`.
//!
//! Default market setup:
//!
//! - BTC starts at 100,000.000 USD with maker bids at 99,500 and 99,000, and
//!   maker asks at 100,500 and 101,000.
//! - ETH starts at 3,500.000 USD with maker bids at 3,485 and 3,475, and maker
//!   asks at 3,515 and 3,525.
//! - SOL starts at 150.000 USD with maker bids at 149.5 and 149, and maker asks
//!   at 150.5 and 151.
//!
//! Use [`SdkLocalnetContext::move_market_price_usd`] to move both the oracle
//! and spline price for a market, [`SdkLocalnetContext::update_oracle_prices`]
//! when a test needs oracle-only control, and
//! [`SdkLocalnetContext::update_spline_price_ticks`] when a test needs to move
//! spline state separately.

pub mod context;
pub mod fixture;
pub mod programs;

mod instructions;

pub use context::SdkLocalnetContext;
pub use fixture::{
    DEFAULT_SDK_LOCALNET_FIXTURE_JSON, FixtureActor, FixtureAddresses, FixtureLiquidity,
    FixtureLiquidityLevel, FixtureMarket, FixtureMint, FixtureOraclePriceUpdate, FixturePrice,
    FixtureSigner, FixtureTransaction, ProgramAddresses, SdkLocalnetFixture,
    SdkLocalnetFixtureError, SerializedAccountMeta, SerializedInstruction,
    decode_fixture_instruction, default_sdk_localnet_fixture, default_sdk_localnet_seed_bytes,
    parse_pubkey, parse_pubkeys, phoenix_metadata_from_fixture, sdk_localnet_seed_bytes,
};
pub use programs::{
    DEFAULT_MAINNET_RPC_URL, EMBER_PROGRAM_ENV, ETERNAL_PROGRAM_ENV, FLIGHT_PROGRAM_ENV,
    HAWKEYE_PROGRAM_ENV, PHOENIX_MAINNET_BPF_PROGRAM_CACHE_DIR_ENV,
    PHOENIX_MAINNET_BPF_PROGRAMS_ENV, PHOENIX_MAINNET_RPC_URL_ENV, PHOENIX_REPO_ROOT_ENV,
    PHOENIX_RPC_URL_ENV, REQUIRED_PROGRAM_ARTIFACT_ENV, SdkLocalnetProgram,
    SdkLocalnetProgramPaths, find_sdk_localnet_program_paths, mainnet_bpf_program_cache_dir,
    mainnet_bpf_programs_enabled, sdk_localnet_vm_required,
};
