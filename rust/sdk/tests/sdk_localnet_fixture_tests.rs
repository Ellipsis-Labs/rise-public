mod test_harness;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use borsh::{BorshSerialize, to_vec};
use litesvm::LiteSVM;
use litesvm::types::TransactionMetadata;
use phoenix_rise::accounts::owned::{
    ConditionalOrderCollection, Orderbook, StopLossOrderKind as AccountStopLossOrderKind, Trader,
};
use phoenix_rise::core::{
    BracketLeg, BracketLegOrders, BracketLegTicket, LimitOrderTicket, MarketOrderTicket,
    PhoenixMetadata, PhoenixTxBuilder, TraderKey,
};
use phoenix_rise::ix::constants::{
    PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID, SYSTEM_PROGRAM_ID,
    compute_discriminant,
};
use phoenix_rise::ix::prelude::{
    Direction, HAWKEYE_PROGRAM_ID, HAWKEYE_RETURN_VERSION, HawkeyeReturnData,
    OnboardTraderDelegatedParams, OrderFlags, RegisterTraderParams, SelfTradeBehavior, Side,
    create_onboard_trader_delegated_ix, create_register_trader_ix, decode_hawkeye_return_data,
    get_conditional_orders_address,
};
use phoenix_rise::types::prelude::{
    AuthoritySetView, ExchangeKeysView, ExchangeMarketConfig, ExchangeRiskFactors, ExchangeView,
    MarketStatus,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;
use test_harness::{
    DEFAULT_SDK_LOCALNET_FIXTURE_JSON, FixtureMarket, SdkLocalnetFixture,
    decode_fixture_instruction, default_sdk_localnet_fixture, default_sdk_localnet_seed_bytes,
    sdk_localnet_seed_bytes,
};

const ROOT_SDK_LOCALNET_FIXTURE_JSON: &str =
    include_str!("../../litesvm-test/test-fixtures/default-localnet.json");
const REQUIRED_PROGRAM_ARTIFACT_ENV: &str = "RISE_SDK_LOCALNET_REQUIRE_PROGRAMS";
const PHOENIX_REPO_ROOT_ENV: &str = "PHOENIX_REPO_ROOT";
const ETERNAL_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_ETERNAL_SO";
const EMBER_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_EMBER_SO";
const HAWKEYE_PROGRAM_ENV: &str = "RISE_SDK_LOCALNET_HAWKEYE_SO";
const STOP_LOSS_PERMISSION: u64 = 1 << 2;
const TRADER_ONBOARDING_PERMISSION: u64 = 1 << 4;
const ORACLE_UPDATE_TIMESTAMP: u64 = 1_900_000_001;
const ORACLE_PRICE_EXPO: u8 = 3;
const MAINNET_DEFAULT_TAKER_FEE_MICRO: u32 = 350;
const MAINNET_DEFAULT_MAKER_FEE_MICRO: i32 = 50;
const MICRO_FEE_DENOMINATOR: f64 = 1_000_000.0;
const TRADER_CAPABILITY_HOT: u32 = 1 << 0;
const TRADER_CAPABILITY_FROZEN: u32 = (1 << 1) | (1 << 2);
const TRADER_CAPABILITY_COLD: u32 = (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5);
const BTC_ASSET_ID: u32 = 0;

#[test]
fn default_sdk_localnet_fixture_loads_and_decodes() {
    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");

    assert_eq!(fixture.schema_version, 1);
    assert_eq!(fixture.name, "rise-sdk-default-localnet");
    assert!(!fixture.addresses.log_authority.is_empty());
    assert!(!fixture.addresses.perp_asset_map.is_empty());
    assert!(!fixture.addresses.withdraw_queue.is_empty());
    assert_eq!(fixture.markets.len(), 3);
    assert!(
        fixture
            .markets
            .iter()
            .all(|market| market.default_taker_fee_micro == MAINNET_DEFAULT_TAKER_FEE_MICRO)
    );
    assert!(
        fixture
            .markets
            .iter()
            .all(|market| market.default_maker_fee_micro == MAINNET_DEFAULT_MAKER_FEE_MICRO)
    );
    assert_eq!(fixture.actors.len(), 5);
    assert!(
        fixture
            .actors
            .iter()
            .all(|actor| !actor.fake_usdc_token_account.is_empty())
    );
    assert!(
        fixture
            .actors
            .iter()
            .all(|actor| !actor.phoenix_token_account.is_empty())
    );
    assert!(fixture.setup_transactions.len() >= 8);
    assert!(fixture.action_templates.len() >= 4);

    for transaction in fixture
        .setup_transactions
        .iter()
        .chain(fixture.action_templates.iter())
    {
        assert!(
            !transaction.signer_seeds.is_empty(),
            "{} should identify required signer seeds",
            transaction.name
        );
        assert!(
            !transaction.instructions.is_empty(),
            "{} should contain serialized instructions",
            transaction.name
        );

        for instruction in &transaction.instructions {
            let decoded =
                decode_fixture_instruction(instruction).expect("instruction should decode");
            assert_eq!(decoded.accounts.len(), instruction.accounts.len());
        }
    }
}

#[test]
fn packaged_fixture_copy_matches_root_fixture() {
    assert_eq!(
        DEFAULT_SDK_LOCALNET_FIXTURE_JSON, ROOT_SDK_LOCALNET_FIXTURE_JSON,
        "the Rust test fixture should match \
         rise/rust/litesvm-test/test-fixtures/default-localnet.json"
    );
}

#[test]
fn default_seed_derivation_is_stable() {
    let payer_seed = default_sdk_localnet_seed_bytes("payer");
    assert_eq!(payer_seed.len(), 32);
    assert_ne!(payer_seed, [0_u8; 32]);
}

#[test]
fn spline_update_fixture_prices_are_ticks() {
    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");
    let transaction = fixture
        .action_templates
        .iter()
        .find(|transaction| transaction.name == "splineUpdatePrices")
        .expect("spline update template should exist");

    for market in &fixture.markets {
        let instruction = transaction
            .instructions
            .iter()
            .find(|instruction| instruction.name == format!("update{}SplinePrice", market.symbol))
            .unwrap_or_else(|| panic!("missing spline update for {}", market.symbol));
        let decoded = decode_fixture_instruction(instruction).expect("instruction should decode");

        assert_eq!(decoded.data.len(), 18);
        assert_eq!(
            u64::from_le_bytes(decoded.data[8..16].try_into().unwrap()),
            initial_mark_price_ticks_for_fixture_market(market),
            "{} spline fixture update should encode ticks",
            market.symbol
        );
        assert_eq!(decoded.data[16], 0);
        assert_eq!(decoded.data[17], 1);
    }
}

#[tokio::test]
async fn rise_sdk_limit_order_with_conditionals_executes_stop_loss() {
    let Some(program_paths) = find_sdk_localnet_program_paths() else {
        if sdk_localnet_vm_required() {
            panic!(
                "missing local SBF artifacts; run `mise build-sbf` or set \
                 {ETERNAL_PROGRAM_ENV}/{EMBER_PROGRAM_ENV}"
            );
        }
        eprintln!("skipping LiteSVM flow because local SBF artifacts are missing");
        return;
    };

    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");
    let mut context = SdkLocalnetContext::new(fixture, program_paths);
    context.execute_setup();

    let metadata = phoenix_metadata_from_fixture(&context.fixture);
    let builder = PhoenixTxBuilder::new(&metadata);
    let taker = context.actor("taker0");
    let market = context.market("BTC");
    let taker_authority = parse_pubkey(&taker.pubkey);
    let taker_trader = parse_pubkey(&taker.trader_account);
    let orderbook = parse_pubkey(&market.orderbook);
    let spline = parse_pubkey(&market.spline);
    let perp_asset_map = parse_pubkey(&context.fixture.addresses.perp_asset_map);
    let global_trader_index = parse_pubkeys(&context.fixture.addresses.global_trader_index);
    let active_trader_buffer = parse_pubkeys(&context.fixture.addresses.active_trader_buffer);
    let trader_conditional_orders = get_conditional_orders_address(&taker_trader).unwrap();

    let take_profit_ticks = price_to_ticks(&builder, "BTC", 110_000.0);
    let stop_loss_ticks = price_to_ticks(&builder, "BTC", 85_000.0);
    let expected_stop_loss_execution_ticks = stop_loss_ticks * 9 / 10;

    let bracket = BracketLegTicket::new(
        Arc::new(RpcClient::new_mock("succeeds".to_string())),
        BracketLegOrders {
            stop_loss: Some(BracketLeg::new(85_000.0)),
            take_profit: Some(BracketLeg::new(110_000.0)),
        },
    );
    let parent_ticket = LimitOrderTicket::builder()
        .authority(taker_authority)
        .trader_account(taker_trader)
        .symbol("BTC")
        .side(Side::Bid)
        .price(90_000.0)
        .num_base_lots(100)
        .self_trade_behavior(SelfTradeBehavior::CancelProvide)
        .order_flags(OrderFlags::None)
        .bracket_leg_ticket(bracket)
        .build()
        .unwrap();
    let parent_ixs = builder.place_limit_order(parent_ticket).await.unwrap();
    assert_eq!(
        parent_ixs.len(),
        2,
        "mock RPC reports no conditional-order account, so the SDK should create it before \
         placing the bracketed limit order"
    );
    context.send_instructions(parent_ixs, "taker-0000", "place-limit-with-conditionals");

    let conditional_orders = decode_conditional_orders(&context, &trader_conditional_orders);
    let active_order_index = *conditional_orders
        .active_order_indices
        .first()
        .expect("attached conditional order should be active");
    let conditional_order = &conditional_orders.orders[active_order_index as usize];
    assert_eq!(
        conditional_order.greater_trigger_order.order_kind,
        AccountStopLossOrderKind::Limit
    );
    assert_eq!(
        conditional_order.greater_trigger_order.execution_price,
        take_profit_ticks
    );
    assert_eq!(
        conditional_order.less_trigger_order.order_kind,
        AccountStopLossOrderKind::IOC
    );
    assert_eq!(
        conditional_order.less_trigger_order.execution_price,
        expected_stop_loss_execution_ticks
    );

    context.send_fixture_transaction("orderbookCancelLevels");

    let fill_parent_ticket = MarketOrderTicket::builder()
        .authority(context.actor_pubkey("orderbookTrader"))
        .trader_account(context.actor_trader("orderbookTrader"))
        .symbol("BTC")
        .side(Side::Ask)
        .price(90_000.0)
        .num_base_lots(100)
        .self_trade_behavior(SelfTradeBehavior::CancelProvide)
        .order_flags(OrderFlags::None)
        .build()
        .unwrap();
    context.send_instructions(
        builder
            .place_market_order(fill_parent_ticket)
            .await
            .unwrap(),
        "orderbook-trader",
        "fill-parent-limit",
    );

    assert!(
        effective_base_lot_position(&context, &taker_trader, BTC_ASSET_ID).unwrap_or_default() > 0,
        "parent fill should open a long position"
    );

    let payer = context.signer_pubkey("payer");
    let executor = context.signer_pubkey("stop-loss-executor");
    let executor_permission = get_permission_address(&payer, &executor);
    context.send_instructions(
        vec![
            create_permission_ix(&payer, &payer, &executor),
            set_permission_ix(&payer, &executor, STOP_LOSS_PERMISSION),
        ],
        "payer",
        "delegate-stop-loss-executor",
    );

    context.send_instructions(
        vec![ping_conditional_order_ix(
            executor,
            executor_permission,
            orderbook,
            taker_trader,
            &global_trader_index,
            &active_trader_buffer,
            trader_conditional_orders,
        )],
        "stop-loss-executor",
        "ping-conditional-order",
    );

    context.send_instructions(
        vec![
            update_oracle_prices_with_ordering_ix(
                context.signer_pubkey("price-oracle"),
                get_permission_address(&payer, &context.signer_pubkey("price-oracle")),
                perp_asset_map,
                "84000",
            ),
            update_spline_price_ix(
                context.actor_pubkey("splineTrader"),
                context.actor_trader("splineTrader"),
                spline,
                price_to_ticks(&builder, "BTC", 84_000.0),
            ),
        ],
        "price-oracle",
        "move-mark-price-through-stop-loss-trigger",
    );

    let exit_liquidity_ticket = LimitOrderTicket::builder()
        .authority(context.actor_pubkey("orderbookTrader"))
        .trader_account(context.actor_trader("orderbookTrader"))
        .symbol("BTC")
        .side(Side::Bid)
        .price(85_000.0)
        .num_base_lots(100)
        .self_trade_behavior(SelfTradeBehavior::CancelProvide)
        .order_flags(OrderFlags::None)
        .build()
        .unwrap();
    context.send_instructions(
        builder
            .place_limit_order(exit_liquidity_ticket)
            .await
            .unwrap(),
        "orderbook-trader",
        "place-stop-loss-exit-liquidity",
    );

    context.send_instructions(
        vec![execute_conditional_order_ix(
            executor,
            executor_permission,
            taker_trader,
            perp_asset_map,
            &global_trader_index,
            &active_trader_buffer,
            orderbook,
            spline,
            trader_conditional_orders,
            active_order_index,
            Direction::LessThan,
        )],
        "stop-loss-executor",
        "execute-stop-loss-conditional-order",
    );

    assert_eq!(
        effective_base_lot_position(&context, &taker_trader, BTC_ASSET_ID).unwrap_or_default(),
        0,
        "executing the conditional stop loss should close the long position"
    );
    assert!(
        !orderbook_has_resting_price(
            &context,
            &orderbook,
            price_to_ticks(&builder, "BTC", 85_000.0)
        ),
        "stop-loss execution should consume the maker bid"
    );
}

#[test]
fn rise_sdk_onboard_trader_delegated_ix_executes() {
    let Some(program_paths) = find_sdk_localnet_program_paths() else {
        if sdk_localnet_vm_required() {
            panic!(
                "missing local SBF artifacts; run `mise build-sbf` or set \
                 {ETERNAL_PROGRAM_ENV}/{EMBER_PROGRAM_ENV}"
            );
        }
        eprintln!("skipping LiteSVM flow because local SBF artifacts are missing");
        return;
    };

    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");
    let mut context = SdkLocalnetContext::new(fixture, program_paths);
    context.execute_setup();

    let payer = context.signer_pubkey("payer");
    let delegate = context.add_signer("rise-onboarding-delegate", 10_000_000_000);
    let trader_authority = context.add_signer("rise-gated-trader", 10_000_000_000);
    let trader_key = TraderKey::new(trader_authority);
    let trader_account = trader_key.pda();

    #[derive(BorshSerialize)]
    struct ChangeExchangeStatusData {
        active: Option<bool>,
        gated: Option<bool>,
    }

    let mut set_gated_exchange_data =
        compute_discriminant("global:change_exchange_status").to_vec();
    set_gated_exchange_data.extend_from_slice(
        &to_vec(&ChangeExchangeStatusData {
            active: Some(true),
            gated: Some(true),
        })
        .unwrap(),
    );
    context.send_instructions(
        vec![Instruction {
            program_id: *PHOENIX_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
                AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
                AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
                AccountMeta::new_readonly(payer, true),
                AccountMeta::new(payer, false),
            ],
            data: set_gated_exchange_data,
        }],
        "payer",
        "enable-gated-exchange",
    );

    let register_params = RegisterTraderParams::builder()
        .payer(payer)
        .trader(trader_authority)
        .trader_account(trader_account)
        .max_positions(128)
        .trader_pda_index(trader_key.pda_index)
        .subaccount_index(trader_key.subaccount_index)
        .build()
        .unwrap();
    context.send_instructions(
        vec![create_register_trader_ix(register_params).unwrap().into()],
        "payer",
        "register-gated-trader",
    );

    assert_eq!(
        decode_trader(&context, &trader_account).state.flags,
        TRADER_CAPABILITY_FROZEN,
        "gated registration should leave the trader frozen until delegated onboarding"
    );

    context.send_instructions(
        vec![
            create_permission_ix(&payer, &payer, &delegate),
            set_permission_ix(&payer, &delegate, TRADER_ONBOARDING_PERMISSION),
        ],
        "payer",
        "delegate-onboarding",
    );

    let onboard_params = OnboardTraderDelegatedParams::builder()
        .authority(delegate)
        .permission_account(get_permission_address(&payer, &delegate))
        .trader_account(trader_account)
        .global_trader_index(parse_pubkeys(
            &context.fixture.addresses.global_trader_index,
        ))
        .active_trader_buffer(parse_pubkeys(
            &context.fixture.addresses.active_trader_buffer,
        ))
        .build()
        .unwrap();
    context.send_instructions(
        vec![
            create_onboard_trader_delegated_ix(onboard_params)
                .unwrap()
                .into(),
        ],
        "rise-onboarding-delegate",
        "onboard-trader-delegated",
    );

    assert_eq!(
        decode_trader(&context, &trader_account).state.flags,
        TRADER_CAPABILITY_COLD,
        "delegated onboarding should restore the full cold capability set"
    );
}

#[tokio::test]
async fn rise_sdk_hawkeye_views_execute_and_decode_return_data() {
    let Some(program_paths) = find_sdk_localnet_program_paths() else {
        if sdk_localnet_vm_required() {
            panic!(
                "missing local SBF artifacts; run `mise build-sbf` or set \
                 {ETERNAL_PROGRAM_ENV}/{EMBER_PROGRAM_ENV}/{HAWKEYE_PROGRAM_ENV}"
            );
        }
        eprintln!("skipping Hawkeye LiteSVM flow because local SBF artifacts are missing");
        return;
    };
    if program_paths.hawkeye.is_none() {
        if sdk_localnet_vm_required() {
            panic!("missing local Hawkeye SBF artifact; set {HAWKEYE_PROGRAM_ENV}");
        }
        eprintln!("skipping Hawkeye LiteSVM flow because phoenix_hawkeye.so is missing");
        return;
    }

    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");
    let mut context = SdkLocalnetContext::new(fixture, program_paths);
    context.execute_setup();

    let metadata = phoenix_metadata_from_fixture(&context.fixture);
    let builder = PhoenixTxBuilder::new(&metadata);
    let actor = context.actor("taker0");
    let authority = parse_pubkey(&actor.pubkey);
    let trader = parse_pubkey(&actor.trader_account);

    let ticket = MarketOrderTicket::builder()
        .authority(authority)
        .trader_account(trader)
        .symbol("BTC")
        .side(Side::Bid)
        .price(101_000.0)
        .num_base_lots(100)
        .self_trade_behavior(SelfTradeBehavior::CancelProvide)
        .order_flags(OrderFlags::None)
        .build()
        .unwrap();
    context.send_instructions(
        builder.place_market_order(ticket).await.unwrap(),
        &actor.seed,
        "hawkeye-open-position",
    );

    let margin = decode_hawkeye_return(
        &context.send_instructions_with_metadata(
            builder
                .build_hawkeye_view_margin_for_trader(trader)
                .unwrap(),
            "payer",
            "hawkeye-view-margin",
        ),
    );
    let HawkeyeReturnData::Margin(margin) = margin else {
        panic!("view_margin returned unexpected Hawkeye payload: {margin:?}");
    };
    assert_eq!(margin.version, HAWKEYE_RETURN_VERSION);
    assert!(
        margin.position_count > 0,
        "opened position should appear in margin view"
    );
    assert!(
        margin.initial_margin_quote_lots > 0,
        "margin view should include position margin"
    );

    let asset = decode_hawkeye_return(
        &context.send_instructions_with_metadata(
            builder
                .build_hawkeye_view_margin_for_asset_for_trader(trader, BTC_ASSET_ID)
                .unwrap(),
            "payer",
            "hawkeye-view-margin-for-asset",
        ),
    );
    let HawkeyeReturnData::Asset(asset) = asset else {
        panic!("view_margin_for_asset returned unexpected Hawkeye payload: {asset:?}");
    };
    assert_eq!(asset.asset_id, BTC_ASSET_ID);
    assert_eq!(asset.version, HAWKEYE_RETURN_VERSION);
    assert_ne!(asset.base_lots, 0);
    assert!(asset.mark_price_ticks > 0);

    let liquidation = decode_hawkeye_return(
        &context.send_instructions_with_metadata(
            builder
                .build_hawkeye_view_liquidation_price_for_trader(trader, BTC_ASSET_ID)
                .unwrap(),
            "payer",
            "hawkeye-view-liquidation-price",
        ),
    );
    let HawkeyeReturnData::LiquidationPrice(liquidation) = liquidation else {
        panic!("view_liquidation_price returned unexpected Hawkeye payload: {liquidation:?}");
    };
    assert_eq!(liquidation.asset_id, BTC_ASSET_ID);
    assert_eq!(liquidation.version, HAWKEYE_RETURN_VERSION);
    assert!(liquidation.mark_price_ticks > 0);

    let bbo = decode_hawkeye_return(&context.send_instructions_with_metadata(
        builder.build_hawkeye_view_bbo("BTC").unwrap(),
        "payer",
        "hawkeye-view-bbo",
    ));
    let HawkeyeReturnData::Bbo(bbo) = bbo else {
        panic!("view_bbo returned unexpected Hawkeye payload: {bbo:?}");
    };
    assert_eq!(bbo.version, HAWKEYE_RETURN_VERSION);
    assert!(
        bbo.best_bid_ticks().is_some() || bbo.best_ask_ticks().is_some(),
        "fixture should have BBO liquidity"
    );
    assert!(bbo.mark_price_ticks > 0);
    assert!(bbo.index_price_ticks > 0);

    let funding = decode_hawkeye_return(
        &context.send_instructions_with_metadata(
            builder
                .build_hawkeye_view_funding_for_trader(trader, BTC_ASSET_ID)
                .unwrap(),
            "payer",
            "hawkeye-view-funding",
        ),
    );
    let HawkeyeReturnData::Funding(funding) = funding else {
        panic!("view_funding returned unexpected Hawkeye payload: {funding:?}");
    };
    assert_eq!(funding.asset_id, BTC_ASSET_ID);
    assert_eq!(funding.version, HAWKEYE_RETURN_VERSION);
    assert!(
        funding.total_accumulated_funding_quote_lots().is_some(),
        "opened position should expose accumulated funding"
    );
}

struct SdkLocalnetProgramPaths {
    phoenix_eternal: PathBuf,
    ember: PathBuf,
    hawkeye: Option<PathBuf>,
}

struct SdkLocalnetContext {
    fixture: SdkLocalnetFixture,
    svm: LiteSVM,
    signers_by_seed: HashMap<String, Keypair>,
    signer_seed_by_pubkey: HashMap<Pubkey, String>,
}

impl SdkLocalnetContext {
    fn new(fixture: SdkLocalnetFixture, program_paths: SdkLocalnetProgramPaths) -> Self {
        let mut svm = LiteSVM::new();
        load_program(
            &mut svm,
            &fixture.programs.phoenix_eternal,
            &program_paths.phoenix_eternal,
        );
        load_program(&mut svm, &fixture.programs.ember, &program_paths.ember);
        if let Some(hawkeye) = program_paths.hawkeye.as_ref() {
            load_program(&mut svm, &HAWKEYE_PROGRAM_ID.to_string(), hawkeye);
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
                parse_pubkey(&signer_config.pubkey),
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

    fn execute_setup(&mut self) {
        for transaction in self.fixture.setup_transactions.clone() {
            self.send_fixture_transaction(&transaction.name);
        }
    }

    fn add_signer(&mut self, seed: &str, initial_lamports: u64) -> Pubkey {
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

    fn send_fixture_transaction(&mut self, name: &str) {
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

    fn send_instructions(
        &mut self,
        instructions: Vec<Instruction>,
        fee_payer_seed: &str,
        label: &str,
    ) {
        self.send_instructions_with_metadata(instructions, fee_payer_seed, label);
    }

    fn send_instructions_with_metadata(
        &mut self,
        instructions: Vec<Instruction>,
        fee_payer_seed: &str,
        label: &str,
    ) -> TransactionMetadata {
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

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&fee_payer.pubkey()),
            &signers,
            self.svm.latest_blockhash(),
        );
        self.svm.send_transaction(tx).unwrap_or_else(|error| {
            panic!(
                "LiteSVM transaction failed for {label}: {:?}\n{}",
                error.err,
                error.meta.logs.join("\n")
            )
        })
    }

    fn signer_pubkey(&self, seed: &str) -> Pubkey {
        self.signers_by_seed
            .get(seed)
            .unwrap_or_else(|| panic!("missing signer {seed}"))
            .pubkey()
    }

    fn actor(&self, name: &str) -> test_harness::FixtureActor {
        self.fixture
            .actors
            .iter()
            .find(|actor| actor.name == name)
            .unwrap_or_else(|| panic!("missing actor {name}"))
            .clone()
    }

    fn actor_pubkey(&self, name: &str) -> Pubkey {
        parse_pubkey(&self.actor(name).pubkey)
    }

    fn actor_trader(&self, name: &str) -> Pubkey {
        parse_pubkey(&self.actor(name).trader_account)
    }

    fn market(&self, symbol: &str) -> FixtureMarket {
        self.fixture
            .markets
            .iter()
            .find(|market| market.symbol == symbol)
            .unwrap_or_else(|| panic!("missing market {symbol}"))
            .clone()
    }
}

fn sdk_localnet_vm_required() -> bool {
    matches!(
        std::env::var(REQUIRED_PROGRAM_ARTIFACT_ENV).as_deref(),
        Ok("1") | Ok("true")
    )
}

fn find_sdk_localnet_program_paths() -> Option<SdkLocalnetProgramPaths> {
    let explicit = match (
        std::env::var(ETERNAL_PROGRAM_ENV),
        std::env::var(EMBER_PROGRAM_ENV),
        std::env::var(HAWKEYE_PROGRAM_ENV),
    ) {
        (Ok(phoenix_eternal), Ok(ember), hawkeye) => Some(SdkLocalnetProgramPaths {
            phoenix_eternal: PathBuf::from(phoenix_eternal),
            ember: PathBuf::from(ember),
            hawkeye: hawkeye.ok().map(PathBuf::from),
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
            },
            SdkLocalnetProgramPaths {
                phoenix_eternal: root.join("target/deploy/phoenix_eternal.so"),
                ember: root.join("target/deploy/phoenix_ember_program.so"),
                hawkeye: optional_program_path(root.join("target/deploy/phoenix_hawkeye.so")),
            },
            SdkLocalnetProgramPaths {
                phoenix_eternal: root.join("programs/eternal/target/deploy/phoenix_eternal.so"),
                ember: root.join("programs/ember/target/deploy/phoenix_ember_program.so"),
                hawkeye: optional_program_path(
                    root.join("programs/phoenix-hawkeye/target/deploy/phoenix_hawkeye.so"),
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
    let mut roots = Vec::new();
    if let Ok(root) = std::env::var(PHOENIX_REPO_ROOT_ENV) {
        roots.push(PathBuf::from(root));
    }
    roots.push(manifest_dir.join("../.."));
    roots.push(std::env::current_dir().unwrap_or_else(|_| manifest_dir.clone()));
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
}

fn optional_program_path(path: PathBuf) -> Option<PathBuf> {
    path.exists().then_some(path)
}

fn load_program(svm: &mut LiteSVM, program_id: &str, path: &Path) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    svm.add_program(parse_pubkey(program_id), &bytes)
        .unwrap_or_else(|error| panic!("failed to load {}: {error:?}", path.display()));
}

fn phoenix_metadata_from_fixture(fixture: &SdkLocalnetFixture) -> PhoenixMetadata {
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
            .find(|mint| mint.name == "phoenix")
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

fn price_to_ticks(builder: &PhoenixTxBuilder<'_>, symbol: &str, price: f64) -> u64 {
    builder
        .order_ticket_metadata(symbol)
        .unwrap()
        .market_calc
        .price_to_ticks(price)
        .unwrap()
        .as_inner()
}

fn initial_mark_price_ticks_for_fixture_market(market: &FixtureMarket) -> u64 {
    let mut numerator = market.initial_mark_price_atoms as u128 * 1_000;
    let mut denominator = market.tick_size_in_quote_lots_per_base_lot as u128;
    let decimal_scale = 10_u128.pow(market.base_lot_decimals.unsigned_abs() as u32);

    if market.base_lot_decimals >= 0 {
        denominator *= decimal_scale;
    } else {
        numerator *= decimal_scale;
    }

    (numerator / denominator) as u64
}

fn decode_conditional_orders(
    context: &SdkLocalnetContext,
    address: &Pubkey,
) -> ConditionalOrderCollection {
    ConditionalOrderCollection::try_from_account_bytes(&account_data(context, address)).unwrap()
}

fn decode_trader(context: &SdkLocalnetContext, address: &Pubkey) -> Trader {
    Trader::try_from_account_bytes(&account_data(context, address)).unwrap()
}

fn decode_orderbook(context: &SdkLocalnetContext, address: &Pubkey) -> Orderbook {
    Orderbook::try_from_account_bytes(&account_data(context, address)).unwrap()
}

fn account_data(context: &SdkLocalnetContext, address: &Pubkey) -> Vec<u8> {
    context
        .svm
        .get_account(address)
        .unwrap_or_else(|| panic!("missing account {address}"))
        .data
}

fn decode_hawkeye_return(tx: &TransactionMetadata) -> HawkeyeReturnData {
    assert_eq!(tx.return_data.program_id, HAWKEYE_PROGRAM_ID);
    assert!(
        !tx.return_data.data.is_empty(),
        "Hawkeye transaction should set return data"
    );
    decode_hawkeye_return_data(&tx.return_data.data).expect("decode Hawkeye return data")
}

fn orderbook_has_resting_price(
    context: &SdkLocalnetContext,
    orderbook: &Pubkey,
    price_in_ticks: u64,
) -> bool {
    decode_orderbook(context, orderbook)
        .bids
        .iter()
        .any(|entry| {
            entry.order_id.price_in_ticks == price_in_ticks
                && entry.order.num_base_lots_remaining > 0
        })
}

fn effective_base_lot_position(
    context: &SdkLocalnetContext,
    trader_account: &Pubkey,
    asset_id: u32,
) -> Option<i64> {
    let trader = decode_trader(context, trader_account);
    let cold_position = trader
        .positions
        .entries
        .iter()
        .find(|(position_asset_id, _)| *position_asset_id == asset_id as u64)
        .map(|(_, position)| position.base_lot_position);

    if trader.state.flags & TRADER_CAPABILITY_HOT == 0 {
        return cold_position;
    }

    let trader_pointer = read_global_trader_pointer(context, trader_account)?;
    read_active_base_lot_position(context, trader_pointer, asset_id)
}

fn read_global_trader_pointer(
    context: &SdkLocalnetContext,
    trader_account: &Pubkey,
) -> Option<u32> {
    find_dynamic_tree_node(
        &account_data_array(context, &context.fixture.addresses.global_trader_index),
        GLOBAL_TRADER_INDEX_NODE_STRIDE_BYTES,
        |bytes, key_offset| {
            compare_bytes(trader_account.as_ref(), &bytes[key_offset..key_offset + 32])
        },
    )
    .map(|node| node.node_pointer)
}

fn read_active_base_lot_position(
    context: &SdkLocalnetContext,
    trader_pointer: u32,
    asset_id: u32,
) -> Option<i64> {
    let node = find_dynamic_tree_node(
        &account_data_array(context, &context.fixture.addresses.active_trader_buffer),
        ACTIVE_TRADER_BUFFER_NODE_STRIDE_BYTES,
        |bytes, key_offset| compare_trader_position_id(bytes, key_offset, trader_pointer, asset_id),
    )?;
    Some(read_i64_le(
        &node.bytes,
        node.key_offset + TRADER_POSITION_ID_BYTES,
    ))
}

const GLOBAL_TRADER_INDEX_ACCOUNT_HEADER_BYTES: usize = 48;
const ACTIVE_TRADER_BUFFER_ACCOUNT_HEADER_BYTES: usize = 48;
const MULTI_ARENA_SUPERBLOCK_BYTES: usize = 32;
const RED_BLACK_TREE_HEADER_BYTES: usize = 16;
const RED_BLACK_TREE_NODE_REGISTER_BYTES: usize = 16;
const INDEXED_ARENA_HEADER_BYTES: usize = 32;
const TRADER_STATE_BYTES: usize = 16;
const TRADER_POSITION_ID_BYTES: usize = 8;
const TRADER_POSITION_STATE_BYTES: usize = 64;
const GLOBAL_TRADER_INDEX_NODE_STRIDE_BYTES: usize =
    RED_BLACK_TREE_NODE_REGISTER_BYTES + 32 + TRADER_STATE_BYTES;
const ACTIVE_TRADER_BUFFER_NODE_STRIDE_BYTES: usize =
    RED_BLACK_TREE_NODE_REGISTER_BYTES + TRADER_POSITION_ID_BYTES + TRADER_POSITION_STATE_BYTES;

struct DynamicTreeNode {
    node_pointer: u32,
    bytes: Vec<u8>,
    key_offset: usize,
}

fn account_data_array(context: &SdkLocalnetContext, addresses: &[String]) -> Vec<Vec<u8>> {
    addresses
        .iter()
        .map(|address| account_data(context, &parse_pubkey(address)))
        .collect()
}

fn find_dynamic_tree_node(
    buffers: &[Vec<u8>],
    node_stride_bytes: usize,
    compare_key: impl Fn(&[u8], usize) -> i32,
) -> Option<DynamicTreeNode> {
    let first_buffer = buffers.first().expect("dynamic tree should have header");
    let account_header_bytes = if node_stride_bytes == GLOBAL_TRADER_INDEX_NODE_STRIDE_BYTES {
        GLOBAL_TRADER_INDEX_ACCOUNT_HEADER_BYTES
    } else {
        ACTIVE_TRADER_BUFFER_ACCOUNT_HEADER_BYTES
    };
    let superblock_offset = account_header_bytes;
    let tree_header_offset = superblock_offset + MULTI_ARENA_SUPERBLOCK_BYTES;
    let num_nodes_per_arena = read_u32_le(first_buffer, superblock_offset + 8);
    let mut node_pointer = read_u32_le(first_buffer, tree_header_offset);
    let mut guard = 0_u32;

    while node_pointer != 0 {
        assert_ne!(
            num_nodes_per_arena, 0,
            "dynamic tree superblock is uninitialized"
        );
        guard += 1;
        assert!(
            guard <= read_u32_le(first_buffer, superblock_offset) + 1,
            "dynamic tree traversal exceeded tree size"
        );

        let node = dynamic_tree_node(
            buffers,
            node_pointer,
            num_nodes_per_arena,
            node_stride_bytes,
            account_header_bytes,
        );
        let comparison = compare_key(&node.bytes, node.key_offset);
        if comparison == 0 {
            return Some(node);
        }

        node_pointer = read_u32_le(
            &node.bytes,
            node.node_offset() + if comparison < 0 { 0 } else { 4 },
        );
    }

    None
}

impl DynamicTreeNode {
    fn node_offset(&self) -> usize {
        self.key_offset - RED_BLACK_TREE_NODE_REGISTER_BYTES
    }
}

fn dynamic_tree_node(
    buffers: &[Vec<u8>],
    node_pointer: u32,
    num_nodes_per_arena: u32,
    node_stride_bytes: usize,
    account_header_bytes: usize,
) -> DynamicTreeNode {
    let page_no = node_pointer - 1;
    let arena_index = page_no / num_nodes_per_arena;
    let node_index = page_no % num_nodes_per_arena;
    let bytes = buffers
        .get(arena_index as usize)
        .unwrap_or_else(|| panic!("missing dynamic tree arena {arena_index}"))
        .clone();
    let arena_offset = if arena_index == 0 {
        account_header_bytes + MULTI_ARENA_SUPERBLOCK_BYTES + RED_BLACK_TREE_HEADER_BYTES
    } else {
        INDEXED_ARENA_HEADER_BYTES
    };
    let node_offset = arena_offset + node_index as usize * node_stride_bytes;

    DynamicTreeNode {
        node_pointer,
        bytes,
        key_offset: node_offset + RED_BLACK_TREE_NODE_REGISTER_BYTES,
    }
}

fn compare_trader_position_id(
    bytes: &[u8],
    key_offset: usize,
    trader_pointer: u32,
    asset_id: u32,
) -> i32 {
    let node_trader_pointer = read_u32_le(bytes, key_offset);
    if trader_pointer != node_trader_pointer {
        return if trader_pointer < node_trader_pointer {
            -1
        } else {
            1
        };
    }

    let node_asset_id = read_u32_le(bytes, key_offset + 4);
    if asset_id == node_asset_id {
        0
    } else if asset_id < node_asset_id {
        -1
    } else {
        1
    }
}

fn compare_bytes(left: &[u8], right: &[u8]) -> i32 {
    match left.cmp(right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_i64_le(bytes: &[u8], offset: usize) -> i64 {
    i64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn create_permission_ix(payer: &Pubkey, authority: &Pubkey, user: &Pubkey) -> Instruction {
    Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
            AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
            AccountMeta::new_readonly(*payer, true),
            AccountMeta::new(get_permission_address(authority, user), false),
            AccountMeta::new_readonly(*authority, false),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data: compute_discriminant("global:create_permission").to_vec(),
    }
}

fn set_permission_ix(authority: &Pubkey, user: &Pubkey, permission: u64) -> Instruction {
    #[derive(BorshSerialize)]
    struct SetPermissionData {
        permission: u64,
        expires_at_timestamp: Option<i64>,
        allowed_signer_actions: Option<i64>,
    }

    let mut data = compute_discriminant("global:set_permission").to_vec();
    data.extend_from_slice(
        &to_vec(&SetPermissionData {
            permission,
            expires_at_timestamp: None,
            allowed_signer_actions: None,
        })
        .unwrap(),
    );

    Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
            AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
            AccountMeta::new(get_permission_address(authority, user), false),
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(*user, false),
        ],
        data,
    }
}

fn ping_conditional_order_ix(
    signer: Pubkey,
    permission_account: Pubkey,
    orderbook: Pubkey,
    trader_account: Pubkey,
    global_trader_index: &[Pubkey],
    active_trader_buffer: &[Pubkey],
    trader_conditional_orders: Pubkey,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new_readonly(signer, true),
        AccountMeta::new(permission_account, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(trader_account, false),
    ];
    accounts.extend(
        global_trader_index
            .iter()
            .map(|pubkey| AccountMeta::new(*pubkey, false)),
    );
    accounts.extend(
        active_trader_buffer
            .iter()
            .map(|pubkey| AccountMeta::new(*pubkey, false)),
    );
    accounts.push(AccountMeta::new(trader_conditional_orders, false));

    Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data: compute_discriminant("global:ping_conditional_orders").to_vec(),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_conditional_order_ix(
    signer: Pubkey,
    permission_account: Pubkey,
    trader_account: Pubkey,
    perp_asset_map: Pubkey,
    global_trader_index: &[Pubkey],
    active_trader_buffer: &[Pubkey],
    orderbook: Pubkey,
    spline_collection: Pubkey,
    trader_conditional_orders: Pubkey,
    conditional_order_index: u8,
    trigger_direction: Direction,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new_readonly(signer, true),
        AccountMeta::new(permission_account, false),
        AccountMeta::new(trader_account, false),
        AccountMeta::new(perp_asset_map, false),
    ];
    accounts.extend(
        global_trader_index
            .iter()
            .map(|pubkey| AccountMeta::new(*pubkey, false)),
    );
    accounts.extend(
        active_trader_buffer
            .iter()
            .map(|pubkey| AccountMeta::new(*pubkey, false)),
    );
    accounts.extend([
        AccountMeta::new(orderbook, false),
        AccountMeta::new(spline_collection, false),
        AccountMeta::new(trader_conditional_orders, false),
    ]);

    let mut data = compute_discriminant("global:execute_conditional_orders").to_vec();
    data.push(conditional_order_index);
    data.push(trigger_direction as u8);

    Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts,
        data,
    }
}

fn update_oracle_prices_with_ordering_ix(
    oracle: Pubkey,
    permission_account: Pubkey,
    perp_asset_map: Pubkey,
    price_usd: &str,
) -> Instruction {
    #[derive(Clone, Copy, BorshSerialize)]
    struct OraclePrice {
        value: u64,
        expo: u8,
    }
    #[derive(Clone, Copy, BorshSerialize)]
    struct UpdateOraclePrice {
        perp_asset_id: [u8; 16],
        new_exchange_perp_price: Option<OraclePrice>,
        new_exchange_spot_price: OraclePrice,
    }
    #[derive(Clone, Copy, BorshSerialize)]
    struct UpdateOracleFlags {
        flags: u8,
    }
    #[derive(BorshSerialize)]
    struct UpdateOraclePricesWithOrdering {
        update_timestamp: u64,
        updates: Vec<UpdateOraclePrice>,
        flags: UpdateOracleFlags,
    }

    let price = OraclePrice {
        value: price_usd_to_oracle_atoms(price_usd),
        expo: ORACLE_PRICE_EXPO,
    };
    let mut data = compute_discriminant("global:update_oracle_prices_with_ordering").to_vec();
    data.extend_from_slice(
        &to_vec(&UpdateOraclePricesWithOrdering {
            update_timestamp: ORACLE_UPDATE_TIMESTAMP,
            updates: vec![UpdateOraclePrice {
                perp_asset_id: symbol_bytes("BTC"),
                new_exchange_perp_price: Some(price),
                new_exchange_spot_price: price,
            }],
            flags: UpdateOracleFlags { flags: 0 },
        })
        .unwrap(),
    );

    Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
            AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
            AccountMeta::new_readonly(*PHOENIX_GLOBAL_CONFIGURATION, false),
            AccountMeta::new_readonly(oracle, true),
            AccountMeta::new(permission_account, false),
            AccountMeta::new(perp_asset_map, false),
        ],
        data,
    }
}

fn update_spline_price_ix(
    signer: Pubkey,
    trader_account: Pubkey,
    spline_collection: Pubkey,
    price_in_ticks: u64,
) -> Instruction {
    #[derive(BorshSerialize)]
    struct UpdateSplinePrice {
        new_mid_price: u64,
        user_update_slot: Option<u64>,
        refresh_regions: bool,
    }

    let mut data = compute_discriminant("global:update_spline_price").to_vec();
    data.extend_from_slice(
        &to_vec(&UpdateSplinePrice {
            new_mid_price: price_in_ticks,
            user_update_slot: None,
            refresh_regions: true,
        })
        .unwrap(),
    );

    Instruction {
        program_id: *PHOENIX_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
            AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
            AccountMeta::new_readonly(signer, true),
            AccountMeta::new_readonly(trader_account, false),
            AccountMeta::new(spline_collection, false),
        ],
        data,
    }
}

fn get_permission_address(authority: &Pubkey, user: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"permission", authority.as_ref(), user.as_ref()],
        &*PHOENIX_PROGRAM_ID,
    )
    .0
}

fn price_usd_to_oracle_atoms(price_usd: &str) -> u64 {
    let (whole, fraction) = price_usd.split_once('.').unwrap_or((price_usd, ""));
    let mut padded_fraction = fraction.to_string();
    padded_fraction.truncate(3);
    while padded_fraction.len() < 3 {
        padded_fraction.push('0');
    }
    whole.parse::<u64>().unwrap() * 1_000 + padded_fraction.parse::<u64>().unwrap()
}

fn symbol_bytes(symbol: &str) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[..symbol.len()].copy_from_slice(symbol.as_bytes());
    bytes
}

fn parse_pubkeys(values: &[String]) -> Vec<Pubkey> {
    values.iter().map(|value| parse_pubkey(value)).collect()
}

fn parse_pubkey(value: &str) -> Pubkey {
    Pubkey::from_str(value).unwrap_or_else(|error| panic!("invalid pubkey {value}: {error}"))
}
