//! LiteSVM runtime context for the Rise localnet fixture.
//!
//! [`SdkLocalnetContext`] owns the [`LiteSVM`] instance, deterministic signers,
//! and the loaded fixture. Construct it with
//! [`default_sdk_localnet_fixture`](crate::default_sdk_localnet_fixture) and
//! [`find_sdk_localnet_program_paths`](crate::find_sdk_localnet_program_paths),
//! then call [`SdkLocalnetContext::execute_setup`] before sending test
//! instructions.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use litesvm::LiteSVM;
use litesvm::types::{FailedTransactionMetadata, TransactionMetadata};
use phoenix_rise_ix::constants::SPL_TOKEN_PROGRAM_ID;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

use crate::fixture::{
    FixtureActor, FixtureMarket, FixtureMint, FixtureOraclePriceUpdate, FixturePrice,
    SdkLocalnetFixture, decode_fixture_instruction, parse_pubkey, sdk_localnet_seed_bytes,
};
use crate::instructions::{flight_init_ix, oracle_price_update_data, spline_price_update_data};
use crate::programs::{
    SdkLocalnetProgram, SdkLocalnetProgramPaths, load_mainnet_protocol_programs, load_program,
    running_in_ci, should_load_mainnet_bpf_programs,
};

const SPL_TOKEN_MINT_TO_DISCRIMINANT: u8 = 7;
const FIXTURE_ORACLE_UPDATE_TIMESTAMP_BASE: u64 = 1_900_000_000;

/// Executable LiteSVM localnet fixture.
///
/// The context loads Phoenix Eternal and Ember, optionally Hawkeye and Flight,
/// and any extra program under test. It also derives all fixture signers,
/// airdrops their configured lamports, and exposes helpers for replaying
/// fixture transactions.
///
/// After [`SdkLocalnetContext::execute_setup`], the default fixture has:
///
/// - fake USDC and Phoenix collateral mints initialized;
/// - BTC, ETH, and SOL markets active with initial oracle prices;
/// - `priceOracle` delegated for oracle updates;
/// - `splineTrader` registered as spline owner for every market;
/// - `orderbookTrader` collateralized and placing deterministic maker levels;
/// - `taker0`, `taker1`, and `stopLossTaker` funded and collateralized.
pub struct SdkLocalnetContext {
    pub fixture: SdkLocalnetFixture,
    pub svm: LiteSVM,
    signers_by_seed: HashMap<String, Keypair>,
    signer_seed_by_pubkey: HashMap<Pubkey, String>,
    oracle_update_sequence: u64,
}

impl SdkLocalnetContext {
    /// Build a context with only the protocol programs from `program_paths`.
    pub fn new(fixture: SdkLocalnetFixture, program_paths: SdkLocalnetProgramPaths) -> Self {
        Self::new_with_programs(fixture, program_paths, [])
    }

    /// Build a context with protocol programs plus extra programs under test.
    ///
    /// `extra_programs` is where program integration tests load their own SBF
    /// artifact into LiteSVM.
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
                    &phoenix_rise_ix::HAWKEYE_PROGRAM_ID.to_string(),
                    hawkeye,
                );
            }
            if let Some(flight) = program_paths.flight.as_ref() {
                load_program(
                    &mut svm,
                    &phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID.to_string(),
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

    /// Load or replace one program in the existing LiteSVM instance.
    pub fn load_program_from_path(&mut self, program_id: &Pubkey, path: &Path) {
        let bytes = std::fs::read(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        self.svm
            .add_program(*program_id, &bytes)
            .unwrap_or_else(|error| panic!("failed to load {}: {error:?}", path.display()));
    }

    /// Load or replace one extra program.
    pub fn load_program(&mut self, program: SdkLocalnetProgram) {
        self.load_program_from_path(&program.program_id, &program.path);
    }

    /// Run every setup transaction embedded in the fixture.
    ///
    /// This initializes mints, exchange state, BTC/ETH/SOL markets, oracle
    /// prices, funded traders, spline ownership, and initial maker levels.
    pub fn execute_setup(&mut self) {
        for transaction in self.fixture.setup_transactions.clone() {
            self.send_fixture_transaction(&transaction.name);
        }
    }

    /// Add a deterministic signer that was not part of the fixture JSON.
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

    /// Initialize Flight global state if the Flight program is loaded.
    pub fn init_flight(&mut self, max_fee_cap_bps: u64) {
        let global_state = phoenix_rise_ix::flight::get_flight_global_state_address().unwrap();
        if self.svm.get_account(&global_state).is_some() {
            return;
        }
        self.send_instructions(
            vec![flight_init_ix(self.signer_pubkey("payer"), max_fee_cap_bps)],
            "payer",
            "flight-init",
        );
    }

    /// Register a fixture actor as a Flight builder.
    pub fn register_flight_builder(&mut self, builder: &FixtureActor, fee_bps: u64) {
        let ix = phoenix_rise_ix::flight::create_register_builder_ix(
            phoenix_rise_ix::flight::RegisterBuilderParams::builder()
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

    /// Initialize Flight and register a fixture actor as a builder.
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

    /// Move both oracle and spline prices for one market.
    ///
    /// This is the usual helper for PnL and liquidation-style tests. It updates
    /// mark and spot oracle prices through `priceOracle`, then updates the
    /// market spline through `splineTrader`. It does not reseed maker orderbook
    /// levels; replay `orderbookPlaceLevels` if the test needs fresh liquidity
    /// around the original fixture prices.
    pub fn move_market_price_usd(&mut self, symbol: &str, price_usd: f64) {
        let price = FixturePrice::from_usd(price_usd);
        let quote_decimals = self.fixture_mint("fakeUsdc").decimals;
        let market = self.market(symbol);
        let price_ticks = market.price_to_ticks(price, quote_decimals);

        self.update_oracle_prices(&[FixtureOraclePriceUpdate::mark_and_spot(symbol, price)]);
        self.update_spline_price_ticks(symbol, price_ticks);
    }

    /// Update one or more oracle prices at the next deterministic timestamp.
    ///
    /// Use [`FixtureOraclePriceUpdate::mark_and_spot_usd`] for the common case
    /// where mark and spot move together.
    pub fn update_oracle_prices(&mut self, updates: &[FixtureOraclePriceUpdate]) {
        let update_timestamp = self.next_fixture_oracle_update_timestamp();
        self.update_oracle_prices_at(update_timestamp, updates);
    }

    /// Update one or more oracle prices at a specific timestamp.
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

    /// Update one market's spline mid price in Phoenix ticks.
    ///
    /// Use [`FixtureMarket::price_usd_to_ticks`] or
    /// [`FixtureMarket::price_to_ticks`] to calculate `new_mid_price_ticks`.
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

    /// Mint fake USDC to a fixture actor's fake USDC token account.
    pub fn airdrop_usdc_to_actor(&mut self, actor_name: &str, amount_atoms: u64) {
        let actor = self.actor(actor_name);
        self.mint_fake_usdc_to_token_account(
            parse_pubkey(&actor.fake_usdc_token_account).unwrap(),
            amount_atoms,
        );
    }

    /// Mint fake USDC to any token account using the payer mint authority.
    pub fn mint_fake_usdc_to_token_account(&mut self, token_account: Pubkey, amount_atoms: u64) {
        let mint = self.fixture_mint("fakeUsdc");
        let mint_pubkey = parse_pubkey(&mint.pubkey).unwrap();
        let authority = self.signer_pubkey("payer");
        let mut data = Vec::with_capacity(9);
        data.push(SPL_TOKEN_MINT_TO_DISCRIMINANT);
        data.extend_from_slice(&amount_atoms.to_le_bytes());

        self.send_instructions(
            vec![Instruction {
                program_id: SPL_TOKEN_PROGRAM_ID,
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

    /// Replay a named setup transaction or action template from the fixture.
    ///
    /// Useful repeatable actions include `oracleSetPrices`,
    /// `splineUpdatePrices`, `orderbookPlaceLevels`, and
    /// `orderbookCancelLevels`.
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

    /// Send signed instructions and panic with LiteSVM logs on failure.
    pub fn send_instructions(
        &mut self,
        instructions: Vec<Instruction>,
        fee_payer_seed: &str,
        label: &str,
    ) {
        self.send_instructions_with_metadata(instructions, fee_payer_seed, label);
    }

    /// Send signed instructions and return LiteSVM transaction metadata.
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

    /// Send signed instructions and return the LiteSVM result.
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

    /// Return a deterministic fixture signer pubkey by seed.
    pub fn signer_pubkey(&self, seed: &str) -> Pubkey {
        self.signers_by_seed
            .get(seed)
            .unwrap_or_else(|| panic!("missing signer {seed}"))
            .pubkey()
    }

    /// Return a fixture actor by name.
    pub fn actor(&self, name: &str) -> FixtureActor {
        self.fixture
            .actors
            .iter()
            .find(|actor| actor.name == name)
            .unwrap_or_else(|| panic!("missing actor {name}"))
            .clone()
    }

    /// Return a fixture actor authority pubkey by actor name.
    pub fn actor_pubkey(&self, name: &str) -> Pubkey {
        parse_pubkey(&self.actor(name).pubkey).expect("actor pubkey should parse")
    }

    /// Return a fixture actor Phoenix trader account by actor name.
    pub fn actor_trader(&self, name: &str) -> Pubkey {
        parse_pubkey(&self.actor(name).trader_account).expect("actor trader should parse")
    }

    /// Return a fixture mint by name.
    pub fn fixture_mint(&self, name: &str) -> FixtureMint {
        self.fixture
            .mints
            .iter()
            .find(|mint| mint.name == name)
            .unwrap_or_else(|| panic!("missing fixture mint {name}"))
            .clone()
    }

    /// Return the fake USDC mint pubkey.
    pub fn fake_usdc_mint(&self) -> Pubkey {
        parse_pubkey(&self.fixture_mint("fakeUsdc").pubkey).expect("fake USDC mint should parse")
    }

    /// Return the Phoenix collateral mint pubkey.
    pub fn phoenix_collateral_mint(&self) -> Pubkey {
        parse_pubkey(&self.fixture_mint("phoenixCollateral").pubkey)
            .expect("Phoenix collateral mint should parse")
    }

    /// Return a fixture market by symbol.
    pub fn market(&self, symbol: &str) -> FixtureMarket {
        self.fixture
            .markets
            .iter()
            .find(|market| market.symbol == symbol)
            .unwrap_or_else(|| panic!("missing market {symbol}"))
            .clone()
    }

    /// Return raw account data from LiteSVM.
    pub fn account_data(&self, address: &Pubkey) -> Vec<u8> {
        self.svm
            .get_account(address)
            .unwrap_or_else(|| panic!("missing account {address}"))
            .data
    }

    /// Return the SPL token amount stored in a token account.
    pub fn token_amount(&self, token_account: &Pubkey) -> u64 {
        let data = self.account_data(token_account);
        u64::from_le_bytes(data[64..72].try_into().unwrap())
    }
}
