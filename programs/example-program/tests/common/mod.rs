use std::io::{self, Write};
use std::path::PathBuf;

use borsh::{BorshSerialize, to_vec};
use phoenix_rise::ix::HAWKEYE_PROGRAM_ID;
use phoenix_rise::ix::constants::{
    EMBER_PROGRAM_ID, PHOENIX_GLOBAL_CONFIGURATION, PHOENIX_LOG_AUTHORITY, PHOENIX_PROGRAM_ID,
    SPL_TOKEN_PROGRAM_ID, SYSTEM_PROGRAM_ID, compute_discriminant,
};
use phoenix_rise::ix::types::{Direction, OrderFlags, SelfTradeBehavior, Side, StopLossOrderKind};
use phoenix_rise_litesvm_test::{
    FixtureActor, FixtureMarket, PHOENIX_REPO_ROOT_ENV, SdkLocalnetContext, SdkLocalnetProgram,
    default_sdk_localnet_fixture, find_sdk_localnet_program_paths, mainnet_bpf_programs_enabled,
    parse_pubkey, parse_pubkeys, sdk_localnet_vm_required,
};
use rise_example_program::{
    CancelLimitOrderParams, CancelStopLossCpiParams, DepositEmberThenPhoenixParams,
    LimitOrderCpiParams, MarketOrderCpiParams, MarketOrderWithoutArenas, StopLossCpiParams,
    SubaccountCloseAndSweepParams, SubaccountOpenAndTradeParams, WithdrawPhoenixThenEmberParams,
};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const EXAMPLE_PROGRAM_ENV: &str = "RISE_EXAMPLE_PROGRAM_SO";

pub const BTC_ASSET_ID: u64 = 0;
pub const BTC_BASE_LOTS: u64 = 25;
pub const LIMIT_BASE_LOTS: u64 = 10;
pub const DEPOSIT_AMOUNT: u64 = 5_000_000;
pub const SUBACCOUNT_TRANSFER_AMOUNT: u64 = 250_000_000_000;

pub fn setup_context() -> Option<(SdkLocalnetContext, Pubkey)> {
    let Some(program_paths) = find_sdk_localnet_program_paths() else {
        if sdk_localnet_vm_required() {
            panic!("missing local Phoenix/Ember/Hawkeye SBF artifacts");
        }
        eprintln!(
            "skipping example-program tests because local protocol SBF artifacts are missing"
        );
        return None;
    };
    if program_paths.hawkeye.is_none() && !mainnet_bpf_programs_enabled() {
        if sdk_localnet_vm_required() {
            panic!("missing local Hawkeye SBF artifact");
        }
        eprintln!("skipping example-program tests because phoenix_hawkeye.so is missing");
        return None;
    }
    let Some(example_program_path) = find_example_program_path() else {
        if sdk_localnet_vm_required() {
            panic!(
                "missing example-program SBF artifact; run cargo-build-sbf for \
                 rise/programs/example-program or set {EXAMPLE_PROGRAM_ENV}"
            );
        }
        eprintln!("skipping example-program tests because its SBF artifact is missing");
        return None;
    };

    let fixture = default_sdk_localnet_fixture().expect("fixture should deserialize");
    let program_id = Pubkey::new_from_array(rise_example_program::ID);
    let mut context = SdkLocalnetContext::new_with_programs(
        fixture,
        program_paths,
        [SdkLocalnetProgram::new(program_id, example_program_path)],
    );
    context.execute_setup();
    context.svm.warp_to_slot(200);
    Some((context, program_id))
}

pub fn prime_market(context: &mut SdkLocalnetContext) {
    context.send_fixture_transaction("oracleSetPrices");
    context.send_fixture_transaction("splineUpdatePrices");
    context.send_fixture_transaction("orderbookPlaceLevels");
}

pub fn with_compute_budget(ix: Instruction) -> Vec<Instruction> {
    vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ix,
    ]
}

pub fn send_and_print_logs(
    context: &mut SdkLocalnetContext,
    instructions: Vec<Instruction>,
    fee_payer_seed: &str,
    label: &str,
) -> Vec<String> {
    let tx = context.send_instructions_with_metadata(instructions, fee_payer_seed, label);
    print_tx_logs(label, &tx.logs);
    tx.logs
}

fn print_tx_logs(label: &str, logs: &[String]) {
    let mut stderr = io::stderr().lock();
    let _ = writeln!(stderr, "\n=== {} transaction logs ===", label);
    if logs.is_empty() {
        let _ = writeln!(stderr, "<no logs>");
    } else {
        for log in logs {
            let _ = writeln!(stderr, "{log}");
        }
    }
    let _ = writeln!(stderr, "=== end {} transaction logs ===\n", label);
}

pub fn deposit_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    program_id: Pubkey,
    amount: u64,
) -> Instruction {
    let params = DepositEmberThenPhoenixParams {
        ember_amount: amount,
        phoenix_amount: amount,
        global_trader_index_count: global_trader_index_count(context),
        active_trader_buffer_count: active_trader_buffer_count(context),
    };
    let mut accounts = vec![
        AccountMeta::new_readonly(EMBER_PROGRAM_ID, false),
        AccountMeta::new_readonly(parse_pubkey(&actor.pubkey).unwrap(), true),
        AccountMeta::new_readonly(
            parse_pubkey(&context.fixture.addresses.ember_state).unwrap(),
            false,
        ),
        AccountMeta::new_readonly(context.fake_usdc_mint(), false),
        AccountMeta::new(context.phoenix_collateral_mint(), false),
        AccountMeta::new(parse_pubkey(&actor.fake_usdc_token_account).unwrap(), false),
        AccountMeta::new(parse_pubkey(&actor.phoenix_token_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.ember_vault).unwrap(),
            false,
        ),
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new(parse_pubkey(&actor.trader_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.global_vault).unwrap(),
            false,
        ),
    ];
    accounts.extend(dynamic_account_metas(context));
    example_ix(
        program_id,
        "global:deposit_ember_then_phoenix",
        &params,
        accounts,
    )
}

pub fn withdraw_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    program_id: Pubkey,
    phoenix_amount: u64,
    ember_amount: Option<u64>,
) -> Instruction {
    let params = WithdrawPhoenixThenEmberParams {
        phoenix_amount,
        ember_amount,
        global_trader_index_count: global_trader_index_count(context),
        active_trader_buffer_count: active_trader_buffer_count(context),
    };
    let mut accounts = vec![
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new_readonly(parse_pubkey(&actor.pubkey).unwrap(), true),
        AccountMeta::new(parse_pubkey(&actor.trader_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap(),
            false,
        ),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.global_vault).unwrap(),
            false,
        ),
        AccountMeta::new(parse_pubkey(&actor.phoenix_token_account).unwrap(), false),
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.withdraw_queue).unwrap(),
            false,
        ),
        AccountMeta::new_readonly(EMBER_PROGRAM_ID, false),
        AccountMeta::new_readonly(
            parse_pubkey(&context.fixture.addresses.ember_state).unwrap(),
            false,
        ),
        AccountMeta::new_readonly(context.fake_usdc_mint(), false),
        AccountMeta::new(context.phoenix_collateral_mint(), false),
        AccountMeta::new(parse_pubkey(&actor.fake_usdc_token_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.ember_vault).unwrap(),
            false,
        ),
    ];
    accounts.extend(dynamic_account_metas(context));
    example_ix(
        program_id,
        "global:withdraw_phoenix_then_ember",
        &params,
        accounts,
    )
}

pub fn place_market_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    side: Side,
    base_lots: u64,
    quote_lots: u64,
    client_order_id: u128,
    order_flags: OrderFlags,
) -> Instruction {
    let params = MarketOrderCpiParams {
        side,
        price_in_ticks: None,
        num_base_lots: base_lots,
        num_quote_lots: Some(quote_lots),
        min_base_lots_to_fill: base_lots.saturating_sub(1),
        min_quote_lots_to_fill: 0,
        self_trade_behavior: SelfTradeBehavior::CancelProvide,
        match_limit: None,
        client_order_id,
        last_valid_slot: None,
        order_flags,
        cancel_existing: false,
        global_trader_index_count: global_trader_index_count(context),
        active_trader_buffer_count: active_trader_buffer_count(context),
    };
    example_ix(
        program_id,
        "global:place_market_order_and_log_return",
        &params,
        market_account_metas(context, actor, market),
    )
}

pub fn place_limit_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    side: Side,
    price_in_ticks: u64,
    base_lots: u64,
    client_order_id: u128,
) -> Instruction {
    let params = LimitOrderCpiParams {
        side,
        price_in_ticks,
        num_base_lots: base_lots,
        self_trade_behavior: SelfTradeBehavior::CancelProvide,
        match_limit: None,
        client_order_id,
        last_valid_slot: None,
        order_flags: OrderFlags::None,
        cancel_existing: false,
        global_trader_index_count: global_trader_index_count(context),
        active_trader_buffer_count: active_trader_buffer_count(context),
    };
    example_ix(
        program_id,
        "global:place_limit_order",
        &params,
        market_account_metas(context, actor, market),
    )
}

pub fn cancel_limit_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    price_in_ticks: u64,
    order_sequence_number: u64,
) -> Instruction {
    let params = CancelLimitOrderParams {
        node_pointer: 0,
        price_in_ticks,
        order_sequence_number,
        global_trader_index_count: global_trader_index_count(context),
        active_trader_buffer_count: active_trader_buffer_count(context),
    };
    example_ix(
        program_id,
        "global:cancel_limit_order",
        &params,
        market_account_metas(context, actor, market),
    )
}

pub fn place_stop_loss_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    stop_loss: Pubkey,
    trigger_price: u64,
    execution_price: u64,
) -> Instruction {
    let params = StopLossCpiParams {
        trigger_price,
        execution_price,
        trade_side: Side::Ask,
        execution_direction: Direction::LessThan,
        order_kind: StopLossOrderKind::IOC,
        global_trader_index_count: global_trader_index_count(context),
        active_trader_buffer_count: active_trader_buffer_count(context),
    };
    let actor_key = parse_pubkey(&actor.pubkey).unwrap();
    let mut accounts = vec![
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new_readonly(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new(actor_key, true),
        AccountMeta::new(parse_pubkey(&actor.trader_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap(),
            false,
        ),
        AccountMeta::new(parse_pubkey(&market.orderbook).unwrap(), false),
        AccountMeta::new(parse_pubkey(&market.spline).unwrap(), false),
        AccountMeta::new_readonly(actor_key, true),
        AccountMeta::new(stop_loss, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];
    accounts.extend(dynamic_account_metas(context));
    example_ix(program_id, "global:place_stop_loss", &params, accounts)
}

pub fn cancel_stop_loss_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    program_id: Pubkey,
    stop_loss: Pubkey,
) -> Instruction {
    let actor_key = parse_pubkey(&actor.pubkey).unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new_readonly(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new(actor_key, false),
        AccountMeta::new_readonly(parse_pubkey(&actor.trader_account).unwrap(), false),
        AccountMeta::new_readonly(actor_key, true),
        AccountMeta::new(stop_loss, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
    ];
    let params = CancelStopLossCpiParams {
        execution_direction: Direction::LessThan,
    };
    let _ = context;
    example_ix(program_id, "global:cancel_stop_loss", &params, accounts)
}

pub fn register_subaccount_and_trade_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    parent_trader: Pubkey,
    child_trader: Pubkey,
    child_subaccount_index: u8,
    order: MarketOrderWithoutArenas,
) -> Instruction {
    let params = SubaccountOpenAndTradeParams {
        max_positions: 1,
        trader_pda_index: 0,
        child_subaccount_index,
        transfer_amount: SUBACCOUNT_TRANSFER_AMOUNT,
        order,
        global_trader_index_count: global_trader_index_count(context),
        active_trader_buffer_count: active_trader_buffer_count(context),
    };
    let actor_key = parse_pubkey(&actor.pubkey).unwrap();
    let mut accounts = vec![
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(HAWKEYE_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new(actor_key, true),
        AccountMeta::new_readonly(actor_key, true),
        AccountMeta::new(parent_trader, false),
        AccountMeta::new(child_trader, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap(),
            false,
        ),
        AccountMeta::new(parse_pubkey(&market.orderbook).unwrap(), false),
        AccountMeta::new(parse_pubkey(&market.spline).unwrap(), false),
    ];
    accounts.extend(dynamic_account_metas(context));
    example_ix(
        program_id,
        "global:register_subaccount_sync_transfer_and_market_order",
        &params,
        accounts,
    )
}

pub fn close_subaccount_and_sweep_ix(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
    program_id: Pubkey,
    parent_trader: Pubkey,
    child_trader: Pubkey,
    order: MarketOrderWithoutArenas,
) -> Instruction {
    let params = SubaccountCloseAndSweepParams {
        order,
        global_trader_index_count: global_trader_index_count(context),
        active_trader_buffer_count: active_trader_buffer_count(context),
    };
    let mut accounts = vec![
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(HAWKEYE_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new_readonly(parse_pubkey(&actor.pubkey).unwrap(), true),
        AccountMeta::new(child_trader, false),
        AccountMeta::new(parent_trader, false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap(),
            false,
        ),
        AccountMeta::new(parse_pubkey(&market.orderbook).unwrap(), false),
        AccountMeta::new(parse_pubkey(&market.spline).unwrap(), false),
    ];
    accounts.extend(dynamic_account_metas(context));
    example_ix(
        program_id,
        "global:close_subaccount_position_and_sweep",
        &params,
        accounts,
    )
}

pub fn quote_lots_for_price(
    market: &FixtureMarket,
    price_usd: f64,
    quote_decimals: u8,
    base_lots: u64,
) -> u64 {
    let price_ticks = market.price_usd_to_ticks(price_usd, quote_decimals) + 1;
    price_ticks * market.tick_size_in_quote_lots_per_base_lot * base_lots
}

pub fn matching_order_id(logs: &[String], label: &str) -> (u64, u64) {
    let needle = format!("{label} matching response");
    let log = logs
        .iter()
        .find(|log| log.contains(&needle) && log.contains("order_id_price_in_ticks="))
        .unwrap_or_else(|| panic!("missing matching response order id for {label}"));
    (
        parse_log_field(log, "order_id_price_in_ticks"),
        parse_log_field(log, "order_sequence_number"),
    )
}

pub fn assert_logs_contain(logs: &[String], needle: &str) {
    assert!(
        logs.iter().any(|log| log.contains(needle)),
        "expected logs to contain {needle:?}"
    );
}

pub fn trader_pda(authority: &Pubkey, pda_index: u8, subaccount_index: u8) -> Pubkey {
    let pda_schema = [pda_index, subaccount_index];
    Pubkey::find_program_address(
        &[b"trader", authority.as_ref(), pda_schema.as_ref()],
        &*PHOENIX_PROGRAM_ID,
    )
    .0
}

fn market_account_metas(
    context: &SdkLocalnetContext,
    actor: &FixtureActor,
    market: &FixtureMarket,
) -> Vec<AccountMeta> {
    let mut accounts = vec![
        AccountMeta::new_readonly(*PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(HAWKEYE_PROGRAM_ID, false),
        AccountMeta::new_readonly(*PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(*PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new_readonly(parse_pubkey(&actor.pubkey).unwrap(), true),
        AccountMeta::new(parse_pubkey(&actor.trader_account).unwrap(), false),
        AccountMeta::new(
            parse_pubkey(&context.fixture.addresses.perp_asset_map).unwrap(),
            false,
        ),
        AccountMeta::new(parse_pubkey(&market.orderbook).unwrap(), false),
        AccountMeta::new(parse_pubkey(&market.spline).unwrap(), false),
    ];
    accounts.extend(dynamic_account_metas(context));
    accounts
}

fn dynamic_account_metas(context: &SdkLocalnetContext) -> Vec<AccountMeta> {
    parse_pubkeys(&context.fixture.addresses.global_trader_index)
        .into_iter()
        .chain(parse_pubkeys(
            &context.fixture.addresses.active_trader_buffer,
        ))
        .map(|pubkey| AccountMeta::new(pubkey, false))
        .collect()
}

fn example_ix<P: BorshSerialize>(
    program_id: Pubkey,
    discriminant: &str,
    params: &P,
    accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut data = compute_discriminant(discriminant).to_vec();
    data.extend(to_vec(params).expect("example params should serialize"));
    Instruction {
        program_id,
        accounts,
        data,
    }
}

fn global_trader_index_count(context: &SdkLocalnetContext) -> u8 {
    context
        .fixture
        .addresses
        .global_trader_index
        .len()
        .try_into()
        .expect("fixture global trader index count should fit in u8")
}

fn active_trader_buffer_count(context: &SdkLocalnetContext) -> u8 {
    context
        .fixture
        .addresses
        .active_trader_buffer
        .len()
        .try_into()
        .expect("fixture active trader buffer count should fit in u8")
}

fn parse_log_field(log: &str, field: &str) -> u64 {
    let needle = format!("{field}=");
    let value = log
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&needle))
        .unwrap_or_else(|| panic!("missing log field {field}: {log}"));
    value.parse().unwrap_or_else(|_| {
        panic!("invalid log field {field}: {value}");
    })
}

fn find_example_program_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var(EXAMPLE_PROGRAM_ENV) {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    for root in default_roots() {
        for path in [
            root.join("rise/programs/example-program/target/deploy/rise_example_program.so"),
            root.join("target/deploy/rise_example_program.so"),
        ] {
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}

fn default_roots() -> Vec<PathBuf> {
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
