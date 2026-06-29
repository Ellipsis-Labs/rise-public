//! CLI for building and submitting the close-position-and-withdraw demo
//! instruction against mainnet metadata.

use std::env;
use std::str::FromStr;

use borsh::to_vec;
use clap::{Args, Parser, Subcommand, ValueEnum};
use phoenix_rise::api::{PhoenixHttpClient, PhoenixMetadata, TraderKey};
use phoenix_rise::ix;
use phoenix_rise::ix::constants::{
    EMBER_PROGRAM_ID, PROD_PHOENIX_GLOBAL_CONFIGURATION, PROD_PHOENIX_LOG_AUTHORITY,
    PROD_PHOENIX_PROGRAM_ID, SPL_TOKEN_PROGRAM_ID, USDC_MINT, compute_discriminant,
    get_associated_token_address, get_ember_state_address, get_ember_vault_address,
};
use phoenix_rise::ix::create_ata::create_associated_token_account_idempotent_ix;
use phoenix_rise::types::prelude::{ExchangeKeysView, TraderStateResponse, TraderView};
use rise_close_position_and_withdraw::{
    ClosePositionAndWithdrawParams, SelfTradeBehavior, Side, WithdrawAllCollateralParams,
    WithdrawMode,
};
use solana_commitment_config::CommitmentConfig;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::read_keypair_file;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DEFAULT_MAINNET_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_cli(Cli::parse());

    let keypair = read_keypair_file(&config.keypair_path)
        .map_err(|error| format!("failed to read keypair {}: {error}", config.keypair_path))?;
    let authority = keypair.pubkey();

    println!("RPC URL: {}", config.rpc_url);
    println!("Authority: {authority}");
    println!("Demo program: {}", config.close_position_program_id);
    println!(
        "Mode: {}",
        if config.send_transaction {
            "send transaction"
        } else {
            "simulate only"
        }
    );

    let rpc = RpcClient::new_with_commitment(config.rpc_url.clone(), CommitmentConfig::confirmed());

    println!("Fetching Phoenix exchange metadata...");
    let http = PhoenixHttpClient::new_from_env()?;
    let exchange = http.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);
    let keys = metadata.keys();
    assert_mainnet_keys(keys)?;

    let canonical_mint = parse_pubkey(&keys.canonical_mint, "canonical mint")?;
    let global_vault = parse_pubkey(&keys.global_vault, "global vault")?;
    let perp_asset_map = parse_pubkey(&keys.perp_asset_map, "perp asset map")?;
    let withdraw_queue = parse_pubkey(&keys.withdraw_queue, "withdraw queue")?;
    let global_trader_index = parse_pubkeys(&keys.global_trader_index, "global trader index")?;
    let active_trader_buffer = parse_pubkeys(&keys.active_trader_buffer, "active trader buffer")?;
    let usdc_mint = USDC_MINT;

    let close_target = match &config.command {
        Command::Close { symbol, .. } => {
            Some(resolve_close_target(&http, &config, authority, symbol).await?)
        }
        Command::WithdrawAll => None,
    };
    let trader_account = close_target
        .as_ref()
        .map(|target| target.trader_account)
        .unwrap_or_else(|| {
            config.trader_account.unwrap_or_else(|| {
                TraderKey::new_with_idx(
                    authority,
                    config.trader_pda_index,
                    config.trader_subaccount_index,
                )
                .pda()
            })
        });
    println!("Trader account: {trader_account}");
    if let Some(target) = &close_target {
        println!(
            "Resolved close: symbol={} pda_index={} subaccount_index={} position={} side={:?} \
             num_base_lots={}",
            target.symbol,
            target.trader_pda_index,
            target.trader_subaccount_index,
            target.position_ui,
            target.side,
            target.num_base_lots
        );
    } else if config.trader_account.is_none() {
        println!(
            "Resolved trader indexes: pda_index={} subaccount_index={}",
            config.trader_pda_index, config.trader_subaccount_index
        );
    }

    rpc.get_account(&config.close_position_program_id)
        .await
        .map_err(|error| {
            format!(
                "close-position-and-withdraw program is not deployed at {} on this RPC: {error}",
                config.close_position_program_id
            )
        })?;
    rpc.get_account(&trader_account)
        .await
        .map_err(|error| format!("trader account {trader_account} was not found: {error}"))?;

    let trader_phoenix_token_account = match config.trader_phoenix_token_account {
        Some(account) => account,
        None => get_associated_token_address(&authority, &canonical_mint)?,
    };
    let usdc_wallet = config.usdc_wallet.unwrap_or(authority);
    if config.trader_usdc_token_account.is_none() && usdc_wallet != authority {
        return Err("--usdc-wallet must match the signing authority unless \
                    --trader-usdc-token-account is supplied; the close-position-and-withdraw \
                    program rejects withdrawals to token accounts not owned by the trader \
                    authority"
            .into());
    }
    let trader_usdc_token_account = match config.trader_usdc_token_account {
        Some(account) => account,
        None => get_associated_token_address(&usdc_wallet, &usdc_mint)?,
    };

    let mut instructions = Vec::new();
    if config.compute_unit_limit > 0 {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(
            config.compute_unit_limit,
        ));
    }
    if config.compute_unit_price_micro_lamports > 0 {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
            config.compute_unit_price_micro_lamports,
        ));
    }
    if config.create_atas {
        if config.trader_phoenix_token_account.is_none() {
            instructions.push(
                create_associated_token_account_idempotent_ix(
                    authority,
                    authority,
                    canonical_mint,
                )?
                .into(),
            );
        }
        if config.trader_usdc_token_account.is_none() {
            instructions.push(
                create_associated_token_account_idempotent_ix(authority, usdc_wallet, usdc_mint)?
                    .into(),
            );
        }
    }

    let demo_accounts = DemoAccounts {
        authority,
        trader_account,
        perp_asset_map,
        global_vault,
        trader_phoenix_token_account,
        withdraw_queue,
        usdc_mint,
        canonical_mint,
        trader_usdc_token_account,
        global_trader_index,
        active_trader_buffer,
    };

    let demo_ix = match &config.command {
        Command::WithdrawAll => build_withdraw_all_instruction(&config, &demo_accounts)?,
        Command::Close {
            symbol: _,
            use_builders,
        } => {
            let target = close_target
                .as_ref()
                .expect("close target must be resolved for close command");
            build_close_instruction(CloseBuildRequest {
                metadata: &metadata,
                config: &config,
                accounts: &demo_accounts,
                symbol: &target.symbol,
                side: target.side,
                num_base_lots: target.num_base_lots,
                use_builders: *use_builders,
            })?
        }
    };
    instructions.push(demo_ix);

    println!("Instruction count: {}", instructions.len());
    println!("Trader Phoenix token account: {trader_phoenix_token_account}");
    println!("Trader USDC token account: {trader_usdc_token_account}");
    if usdc_wallet != authority {
        println!("USDC wallet owner: {usdc_wallet}");
        match &config.command {
            Command::WithdrawAll => println!(
                "Token owner verification is enabled; the USDC token account must be \
                 authority-owned."
            ),
            Command::Close { .. } => println!(
                "The close instruction still validates the USDC token account owner before CPI."
            ),
        }
    }

    let blockhash = rpc.get_latest_blockhash().await?;
    let tx =
        Transaction::new_signed_with_payer(&instructions, Some(&authority), &[&keypair], blockhash);

    println!("Simulating transaction...");
    let simulation = rpc.simulate_transaction(&tx).await?;
    println!("Simulation error: {:?}", simulation.value.err);
    print_logs(simulation.value.logs.as_deref());
    if simulation.value.err.is_some() {
        return Err("simulation failed".into());
    }

    if !config.send_transaction {
        println!("Simulation succeeded. Pass --send-transaction to send it.");
        return Ok(());
    }

    println!("Simulation succeeded. Sending transaction because --send-transaction was provided.");
    let signature = rpc.send_and_confirm_transaction(&tx).await?;
    println!("Transaction confirmed: {signature}");
    println!("Explorer: https://explorer.solana.com/tx/{signature}");

    Ok(())
}

#[derive(Debug, Parser)]
#[command(
    name = "close-position-and-withdraw",
    about = "Invoke the Phoenix close-position-and-withdraw demo program"
)]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    #[command(about = "Find a symbol position for the keypair, close it, and withdraw collateral")]
    Close(CloseArgs),
    #[command(about = "Withdraw all currently free collateral for a trader account")]
    WithdrawAll(WithdrawAllArgs),
    #[command(
        about = "Withdraw all currently free collateral for a derived trader subaccount",
        alias = "withdraw-subaccount"
    )]
    WithdrawAllSubaccount(WithdrawAllSubaccountArgs),
}

#[derive(Debug, Args)]
struct CloseArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[arg(long, env = "SYMBOL")]
    symbol: String,
    #[arg(long, env = "USE_BUILDERS")]
    use_builders: bool,
}

#[derive(Debug, Args)]
struct WithdrawAllArgs {
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Debug, Args)]
struct WithdrawAllSubaccountArgs {
    #[command(flatten)]
    base: BaseArgs,
    #[arg(long, env = "SUBACCOUNT_INDEX")]
    subaccount_index: u8,
    #[arg(long, env = "PDA_INDEX", default_value_t = 0)]
    pda_index: u8,
}

#[derive(Debug, Args)]
struct CommonArgs {
    #[command(flatten)]
    base: BaseArgs,
    #[arg(long, env = "TRADER_ACCOUNT")]
    trader_account: Option<Pubkey>,
    #[arg(long, env = "TRADER_PDA_INDEX", default_value_t = 0)]
    trader_pda_index: u8,
    #[arg(long, env = "TRADER_SUBACCOUNT_INDEX", default_value_t = 0)]
    trader_subaccount_index: u8,
}

#[derive(Debug, Args)]
struct BaseArgs {
    #[arg(long, env = "RPC_URL", default_value = DEFAULT_MAINNET_RPC_URL)]
    rpc_url: String,
    #[arg(long, env = "KEYPAIR_PATH", default_value_t = default_keypair_path())]
    keypair_path: String,
    #[arg(long, env = "CLOSE_POSITION_PROGRAM_ID")]
    close_position_program_id: Option<Pubkey>,
    #[arg(long, env = "TRADER_PHOENIX_TOKEN_ACCOUNT")]
    trader_phoenix_token_account: Option<Pubkey>,
    #[arg(long, env = "TRADER_USDC_TOKEN_ACCOUNT")]
    trader_usdc_token_account: Option<Pubkey>,
    #[arg(long, env = "USDC_WALLET")]
    usdc_wallet: Option<Pubkey>,
    #[arg(long, env = "WITHDRAW_MODE", value_enum, default_value = "all")]
    withdraw_mode: CliWithdrawMode,
    #[arg(long, env = "SKIP_ATA_CREATION")]
    skip_ata_creation: bool,
    #[arg(long, env = "SEND_TRANSACTION")]
    send_transaction: bool,
    #[arg(long, env = "COMPUTE_UNIT_LIMIT", default_value_t = 1_400_000)]
    compute_unit_limit: u32,
    #[arg(long, env = "COMPUTE_UNIT_PRICE_MICRO_LAMPORTS", default_value_t = 0)]
    compute_unit_price_micro_lamports: u64,
    #[arg(long, env = "PRICE_IN_TICKS")]
    price_in_ticks: Option<u64>,
    #[arg(long, env = "MIN_BASE_LOTS_TO_FILL")]
    min_base_lots_to_fill: Option<u64>,
    #[arg(long, env = "MIN_QUOTE_LOTS_TO_FILL", default_value_t = 1)]
    min_quote_lots_to_fill: u64,
    #[arg(long, env = "MATCH_LIMIT")]
    match_limit: Option<u64>,
    #[arg(long, env = "CLIENT_ORDER_ID", default_value_t = 0)]
    client_order_id: u128,
    #[arg(long, env = "LAST_VALID_SLOT")]
    last_valid_slot: Option<u64>,
    #[arg(long, env = "CANCEL_EXISTING")]
    cancel_existing: bool,
    #[arg(long, env = "BUILDER_AUTHORITY")]
    builder_authority: Option<Pubkey>,
    #[arg(long, env = "BUILDER_TRADER_ACCOUNT")]
    builder_trader_account: Option<Pubkey>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliWithdrawMode {
    All,
    Diff,
}

impl From<CliWithdrawMode> for WithdrawMode {
    fn from(value: CliWithdrawMode) -> Self {
        match value {
            CliWithdrawMode::All => WithdrawMode::AllFreeCollateral,
            CliWithdrawMode::Diff => WithdrawMode::OrderFreeCollateralDelta,
        }
    }
}

#[derive(Debug)]
struct Config {
    command: Command,
    rpc_url: String,
    keypair_path: String,
    close_position_program_id: Pubkey,
    trader_account: Option<Pubkey>,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    trader_phoenix_token_account: Option<Pubkey>,
    trader_usdc_token_account: Option<Pubkey>,
    usdc_wallet: Option<Pubkey>,
    withdraw_mode: WithdrawMode,
    create_atas: bool,
    send_transaction: bool,
    compute_unit_limit: u32,
    compute_unit_price_micro_lamports: u64,
    price_in_ticks: Option<u64>,
    min_base_lots_to_fill: Option<u64>,
    min_quote_lots_to_fill: u64,
    match_limit: Option<u64>,
    client_order_id: u128,
    last_valid_slot: Option<u64>,
    cancel_existing: bool,
    builder_authority: Option<Pubkey>,
    builder_trader_account: Option<Pubkey>,
}

#[derive(Debug)]
enum Command {
    WithdrawAll,
    Close { symbol: String, use_builders: bool },
}

impl Config {
    fn from_cli(cli: Cli) -> Self {
        let (base, trader_account, trader_pda_index, trader_subaccount_index, command) =
            match cli.command {
                CliCommand::Close(args) => (
                    args.common.base,
                    args.common.trader_account,
                    args.common.trader_pda_index,
                    args.common.trader_subaccount_index,
                    Command::Close {
                        symbol: args.symbol.to_ascii_uppercase(),
                        use_builders: args.use_builders,
                    },
                ),
                CliCommand::WithdrawAll(args) => (
                    args.common.base,
                    args.common.trader_account,
                    args.common.trader_pda_index,
                    args.common.trader_subaccount_index,
                    Command::WithdrawAll,
                ),
                CliCommand::WithdrawAllSubaccount(args) => (
                    args.base,
                    None,
                    args.pda_index,
                    args.subaccount_index,
                    Command::WithdrawAll,
                ),
            };

        Self {
            command,
            rpc_url: base.rpc_url,
            keypair_path: base.keypair_path,
            close_position_program_id: base
                .close_position_program_id
                .unwrap_or_else(|| Pubkey::new_from_array(rise_close_position_and_withdraw::ID)),
            trader_account,
            trader_pda_index,
            trader_subaccount_index,
            trader_phoenix_token_account: base.trader_phoenix_token_account,
            trader_usdc_token_account: base.trader_usdc_token_account,
            usdc_wallet: base.usdc_wallet,
            withdraw_mode: base.withdraw_mode.into(),
            create_atas: !base.skip_ata_creation,
            send_transaction: base.send_transaction,
            compute_unit_limit: base.compute_unit_limit,
            compute_unit_price_micro_lamports: base.compute_unit_price_micro_lamports,
            price_in_ticks: base.price_in_ticks,
            min_base_lots_to_fill: base.min_base_lots_to_fill,
            min_quote_lots_to_fill: base.min_quote_lots_to_fill,
            match_limit: base.match_limit,
            client_order_id: base.client_order_id,
            last_valid_slot: base.last_valid_slot,
            cancel_existing: base.cancel_existing,
            builder_authority: base.builder_authority,
            builder_trader_account: base.builder_trader_account,
        }
    }
}

struct DemoAccounts {
    authority: Pubkey,
    trader_account: Pubkey,
    perp_asset_map: Pubkey,
    global_vault: Pubkey,
    trader_phoenix_token_account: Pubkey,
    withdraw_queue: Pubkey,
    usdc_mint: Pubkey,
    canonical_mint: Pubkey,
    trader_usdc_token_account: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
}

struct CloseBuildRequest<'a> {
    metadata: &'a PhoenixMetadata,
    config: &'a Config,
    accounts: &'a DemoAccounts,
    symbol: &'a str,
    side: Side,
    num_base_lots: u64,
    use_builders: bool,
}

struct CloseTarget {
    trader_account: Pubkey,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    symbol: String,
    side: Side,
    num_base_lots: u64,
    position_ui: String,
}

async fn resolve_close_target(
    http: &PhoenixHttpClient,
    config: &Config,
    authority: Pubkey,
    symbol: &str,
) -> Result<CloseTarget, Box<dyn std::error::Error>> {
    if let Some(trader_account) = config.trader_account {
        println!("Fetching trader account {trader_account}...");
        let trader = http.traders().get_trader_by_pubkey(&trader_account).await?;
        let Some(target) = close_target_from_trader(&trader, authority, symbol)? else {
            return Err(
                format!("trader account {trader_account} has no active {symbol} position").into(),
            );
        };
        return Ok(target);
    }

    println!(
        "Finding active {symbol} position for authority {authority} at PDA index {}...",
        config.trader_pda_index
    );
    let state: TraderStateResponse = http
        .get_json(&format!(
            "/trader/{authority}/state?pdaIndex={}",
            config.trader_pda_index
        ))
        .await?;
    let matches = state
        .traders
        .iter()
        .filter_map(|trader| close_target_from_trader(trader, authority, symbol).transpose())
        .collect::<Result<Vec<_>, _>>()?;

    match matches.len() {
        0 => Err(format!(
            "no active {symbol} position found for authority {authority} at PDA index {}",
            config.trader_pda_index
        )
        .into()),
        1 => Ok(matches.into_iter().next().expect("one close target")),
        _ => {
            let accounts = matches
                .iter()
                .map(|target| {
                    format!(
                        "{} (subaccount {}, position {})",
                        target.trader_account, target.trader_subaccount_index, target.position_ui
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "found multiple active {symbol} positions for authority {authority}; set \
                 --trader-account to one of: {accounts}"
            )
            .into())
        }
    }
}

fn close_target_from_trader(
    trader: &TraderView,
    authority: Pubkey,
    symbol: &str,
) -> Result<Option<CloseTarget>, Box<dyn std::error::Error>> {
    let trader_authority = parse_pubkey(&trader.authority, "trader authority")?;
    if trader_authority != authority {
        return Err(format!(
            "trader {} belongs to authority {}, not signing authority {}",
            trader.trader_key, trader_authority, authority
        )
        .into());
    }

    let Some(position) = trader.positions.iter().find(|position| {
        position.symbol.eq_ignore_ascii_case(symbol) && position.position_size.value != 0
    }) else {
        return Ok(None);
    };

    let size = position.position_size.value;
    let num_base_lots = size.checked_abs().ok_or_else(|| {
        format!(
            "position size for {} on trader {} is too large to close safely",
            position.symbol, trader.trader_key
        )
    })? as u64;
    let side = if size > 0 { Side::Ask } else { Side::Bid };

    Ok(Some(CloseTarget {
        trader_account: parse_pubkey(&trader.trader_key, "trader account")?,
        trader_pda_index: trader.trader_pda_index,
        trader_subaccount_index: trader.trader_subaccount_index,
        symbol: position.symbol.clone(),
        side,
        num_base_lots,
        position_ui: position.position_size.ui.clone(),
    }))
}

fn build_withdraw_all_instruction(
    config: &Config,
    accounts: &DemoAccounts,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let params = WithdrawAllCollateralParams {
        global_trader_index_count: account_count(accounts.global_trader_index.len())?,
        active_trader_buffer_count: account_count(accounts.active_trader_buffer.len())?,
    };
    let mut data = compute_discriminant("global:withdraw_all_collateral").to_vec();
    data.extend_from_slice(&to_vec(&params)?);
    Ok(Instruction {
        program_id: config.close_position_program_id,
        accounts: shared_account_metas(accounts, false)?,
        data,
    })
}

fn build_close_instruction(
    request: CloseBuildRequest<'_>,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let config = request.config;
    let accounts = request.accounts;
    let market = request
        .metadata
        .get_market(request.symbol)
        .ok_or_else(|| format!("unknown market symbol {}", request.symbol))?;
    let orderbook = parse_pubkey(&market.market_pubkey, "market orderbook")?;
    let spline_collection = parse_pubkey(&market.spline_pubkey, "spline collection")?;
    let params = ClosePositionAndWithdrawParams {
        withdraw_mode: config.withdraw_mode,
        side: request.side,
        price_in_ticks: config.price_in_ticks,
        num_base_lots: request.num_base_lots,
        num_quote_lots: None,
        min_base_lots_to_fill: config
            .min_base_lots_to_fill
            .unwrap_or(request.num_base_lots),
        min_quote_lots_to_fill: config.min_quote_lots_to_fill,
        self_trade_behavior: SelfTradeBehavior::CancelProvide,
        match_limit: config.match_limit,
        client_order_id: config.client_order_id,
        last_valid_slot: config.last_valid_slot,
        cancel_existing: config.cancel_existing,
        global_trader_index_count: account_count(accounts.global_trader_index.len())?,
        active_trader_buffer_count: account_count(accounts.active_trader_buffer.len())?,
    };

    let discriminant = if request.use_builders {
        "global:close_position_and_withdraw_with_builders"
    } else {
        "global:close_position_and_withdraw"
    };
    let mut data = compute_discriminant(discriminant).to_vec();
    data.extend_from_slice(&to_vec(&params)?);

    let mut metas = shared_account_metas(accounts, true)?;
    metas.push(AccountMeta::new(orderbook, false));
    metas.push(AccountMeta::new(spline_collection, false));
    if request.use_builders {
        let builder_authority = config
            .builder_authority
            .ok_or("--builder-authority is required when --use-builders is set")?;
        let builder_trader_account = config
            .builder_trader_account
            .ok_or("--builder-trader-account is required when --use-builders is set")?;
        metas.push(AccountMeta::new_readonly(
            ix::flight::FLIGHT_PROGRAM_ID,
            false,
        ));
        metas.push(AccountMeta::new_readonly(builder_authority, false));
        metas.push(AccountMeta::new(builder_trader_account, false));
        metas.push(AccountMeta::new_readonly(
            ix::flight::get_flight_global_state_address()?,
            false,
        ));
        metas.push(AccountMeta::new_readonly(
            ix::flight::get_flight_builder_state_address(&builder_authority)?,
            false,
        ));
    }
    append_dynamic_accounts(&mut metas, accounts);

    Ok(Instruction {
        program_id: config.close_position_program_id,
        accounts: metas,
        data,
    })
}

fn shared_account_metas(
    accounts: &DemoAccounts,
    omit_dynamic_tail: bool,
) -> Result<Vec<AccountMeta>, Box<dyn std::error::Error>> {
    let mut metas = vec![
        AccountMeta::new_readonly(accounts.authority, true),
        AccountMeta::new_readonly(PROD_PHOENIX_PROGRAM_ID, false),
        AccountMeta::new_readonly(ix::HAWKEYE_PROGRAM_ID, false),
        AccountMeta::new_readonly(EMBER_PROGRAM_ID, false),
        AccountMeta::new_readonly(PROD_PHOENIX_LOG_AUTHORITY, false),
        AccountMeta::new(PROD_PHOENIX_GLOBAL_CONFIGURATION, false),
        AccountMeta::new(accounts.trader_account, false),
        AccountMeta::new(accounts.perp_asset_map, false),
        AccountMeta::new(accounts.global_vault, false),
        AccountMeta::new(accounts.trader_phoenix_token_account, false),
        AccountMeta::new(accounts.withdraw_queue, false),
        AccountMeta::new_readonly(accounts.usdc_mint, false),
        AccountMeta::new(accounts.canonical_mint, false),
        AccountMeta::new(accounts.trader_usdc_token_account, false),
        AccountMeta::new_readonly(get_ember_state_address()?, false),
        AccountMeta::new(get_ember_vault_address()?, false),
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
    ];
    if !omit_dynamic_tail {
        append_dynamic_accounts(&mut metas, accounts);
    }
    Ok(metas)
}

fn append_dynamic_accounts(metas: &mut Vec<AccountMeta>, accounts: &DemoAccounts) {
    metas.extend(
        accounts
            .global_trader_index
            .iter()
            .copied()
            .map(|pubkey| AccountMeta::new(pubkey, false)),
    );
    metas.extend(
        accounts
            .active_trader_buffer
            .iter()
            .copied()
            .map(|pubkey| AccountMeta::new(pubkey, false)),
    );
}

fn assert_mainnet_keys(keys: &ExchangeKeysView) -> Result<(), Box<dyn std::error::Error>> {
    let api_program = keys
        .program_id
        .as_deref()
        .unwrap_or(&PROD_PHOENIX_PROGRAM_ID.to_string())
        .parse::<Pubkey>()?;
    if api_program != PROD_PHOENIX_PROGRAM_ID {
        return Err(
            format!("Phoenix API is not returning mainnet program id: {api_program}").into(),
        );
    }
    let api_global_config = keys.global_config.parse::<Pubkey>()?;
    if api_global_config != PROD_PHOENIX_GLOBAL_CONFIGURATION {
        return Err(format!(
            "Phoenix API is not returning mainnet global config: {api_global_config}"
        )
        .into());
    }
    Ok(())
}

fn parse_pubkey(value: &str, label: &str) -> Result<Pubkey, Box<dyn std::error::Error>> {
    Pubkey::from_str(value)
        .map_err(|error| format!("invalid {label} pubkey {value}: {error}").into())
}

fn parse_pubkeys(
    values: &[String],
    label: &str,
) -> Result<Vec<Pubkey>, Box<dyn std::error::Error>> {
    values
        .iter()
        .map(|value| parse_pubkey(value, label))
        .collect()
}

fn account_count(count: usize) -> Result<u8, Box<dyn std::error::Error>> {
    u8::try_from(count).map_err(|_| format!("dynamic account count {count} exceeds u8").into())
}

fn print_logs(logs: Option<&[String]>) {
    if let Some(logs) = logs {
        println!("Logs:");
        for log in logs {
            println!("{log}");
        }
    }
}

fn default_keypair_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{home}/.config/solana/id.json")
}
