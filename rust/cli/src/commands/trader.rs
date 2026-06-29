use std::error::Error;
use std::str::FromStr;

use clap::{Args, Subcommand};
use phoenix_rise::accounts::owned::Permission;
use phoenix_rise::accounts::permission::TRADER_ONBOARDING_PERMISSION;
use phoenix_rise::api::{
    ActivateReferralTxRequest, BuildRegisterIxsRequest, BuildRegisterIxsResponse,
    ReferralActivationPermissionResponse, SendRegisterIxsRequest,
    fetch_referral_activation_trader_status,
};
use phoenix_rise::core::{PhoenixMetadata, TraderKey};
use phoenix_rise::ix::delegate_trader::{DelegateTraderParams, create_delegate_trader_ix};
use phoenix_rise::ix::onboard_trader_delegated::{
    OnboardTraderDelegatedParams, create_onboard_trader_delegated_ix,
};
use phoenix_rise::ix::register_trader::{RegisterTraderParams, create_register_trader_ix};
use phoenix_rise::types::prelude::{
    CollateralHistoryQueryParams, FundingHistoryQueryParams, OrderHistoryQueryParams,
    PnlQueryParams, PnlResolution, TradeHistoryQueryParams, TraderPositionView, TraderView,
};
use serde::Serialize;
use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction::versioned::VersionedTransaction;

use crate::commands::api::ApiCommand;
use crate::commands::export::{
    FundingHistoryExportArgs, LiquidationHistoryExportArgs, OrderHistoryExportArgs,
    TradeHistoryExportArgs,
};
use crate::commands::trade::{
    CancelStopLossArgs, IsolatedLimitArgs, IsolatedMarketArgs, PlaceStopLossArgs, TradeCommand,
};
use crate::context::CommandCtx;
use crate::output::{InstructionOutput, print_human_json};
use crate::util::{
    api_instruction_to_solana, encode_transaction, parse_pubkey, read_wallet_keypair,
};

const DEFAULT_MAX_POSITIONS: u32 = 128;
const MIN_MAX_POSITIONS: u32 = 32;

#[derive(Debug, Subcommand)]
pub enum TraderCommand {
    /// Register or onboard a trader account.
    Register(RegisterArgs),
    /// Fetch one trader subaccount from the API.
    State(TraderStateArgs),
    /// Print a consolidated trader account summary.
    Summary {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: u8,
        #[arg(long, default_value_t = 0)]
        subaccount_index: u8,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    /// Fetch trader PnL history.
    Pnl(TraderPnlArgs),
    /// Fetch trader collateral history.
    CollateralHistory(TraderCollateralHistoryArgs),
    /// Fetch paginated trade history and print a table or JSON.
    TradeHistory(TradeHistoryExportArgs),
    /// Fetch paginated order history and print a table or JSON.
    OrderHistory(OrderHistoryExportArgs),
    /// Fetch paginated funding history and print a table or JSON.
    FundingHistory(FundingHistoryExportArgs),
    /// Fetch hourly funding history.
    FundingHourly(TraderFundingHourlyArgs),
    /// Fetch paginated liquidation/backstop/ADL history and print a table or
    /// JSON.
    LiquidationHistory(LiquidationHistoryExportArgs),
    /// Build an isolated market-order instruction bundle.
    #[command(alias = "isolated-market")]
    PlaceMarketOrder(IsolatedMarketArgs),
    /// Build an isolated limit-order instruction bundle.
    #[command(alias = "isolated-limit")]
    PlaceLimitOrder(IsolatedLimitArgs),
    /// Build a stop-loss instruction bundle.
    PlaceStopLoss(PlaceStopLossArgs),
    /// Build a cancel-stop-loss instruction bundle.
    CancelStopLoss(CancelStopLossArgs),
    /// Set a trader account's position authority.
    #[command(alias = "delegate-trader", alias = "delegate")]
    SetPositionAuthority(SetPositionAuthorityArgs),
    /// Reset a trader account's position authority back to its trader wallet.
    #[command(alias = "reset-delegation")]
    ResetPositionAuthority(ResetPositionAuthorityArgs),
}

#[derive(Debug, Args)]
pub struct RegisterArgs {
    /// Referral code to activate while registering/onboarding.
    #[arg(long)]
    referral_code: Option<String>,
    /// Trader authority pubkey. Without a referral code this can be any
    /// authority pubkey; with a referral code it must match global --keypair.
    #[arg(long)]
    authority: Option<String>,
    /// Max positions to use when creating a missing trader account.
    #[arg(long, default_value_t = DEFAULT_MAX_POSITIONS)]
    max_positions: u32,
    /// Trader PDA index. The public onboarding endpoints currently support 0.
    #[arg(long, default_value_t = 0, visible_alias = "pda-index")]
    trader_pda_index: u8,
    /// Trader subaccount index. The public onboarding endpoints currently
    /// support 0.
    #[arg(long, default_value_t = 0, visible_alias = "subaccount-index")]
    trader_subaccount_index: u8,
    /// Optional recent blockhash to use when signing the transaction.
    #[arg(long)]
    recent_blockhash: Option<String>,
}

#[derive(Debug, Args)]
pub struct TraderStateArgs {
    /// Trader authority pubkey.
    #[arg(long)]
    authority: String,
    /// Trader PDA index.
    #[arg(long, default_value_t = 0)]
    pda_index: u8,
    /// Trader subaccount index.
    #[arg(long, default_value_t = 0)]
    subaccount_index: u8,
}

#[derive(Debug, Args)]
pub struct TraderPnlArgs {
    /// Trader authority pubkey.
    #[arg(long)]
    authority: String,
    /// PnL aggregation resolution, for example 1d.
    #[arg(long, default_value = "1d")]
    resolution: String,
    /// Inclusive lower timestamp bound in milliseconds.
    #[arg(long)]
    start_time: Option<i64>,
    /// Inclusive upper timestamp bound in milliseconds.
    #[arg(long)]
    end_time: Option<i64>,
    /// Maximum records to return.
    #[arg(long)]
    limit: Option<i64>,
}

#[derive(Debug, Args)]
pub struct TraderCollateralHistoryArgs {
    /// Trader authority pubkey.
    #[arg(long)]
    authority: String,
    /// Trader PDA index.
    #[arg(long, default_value_t = 0)]
    pda_index: u8,
    /// Maximum records to return.
    #[arg(long, default_value_t = 10)]
    limit: i64,
    /// Next page cursor.
    #[arg(long)]
    next_cursor: Option<String>,
    /// Previous page cursor.
    #[arg(long)]
    prev_cursor: Option<String>,
    /// Explicit cursor.
    #[arg(long)]
    cursor: Option<String>,
}

#[derive(Debug, Args)]
pub struct TraderFundingHourlyArgs {
    /// Trader authority pubkey.
    #[arg(long)]
    authority: String,
    /// Market symbol, for example SOL.
    #[arg(long)]
    symbol: Option<String>,
    /// Trader PDA index.
    #[arg(long)]
    pda_index: Option<i32>,
    /// Maximum records to return.
    #[arg(long)]
    limit: Option<i64>,
    /// Page cursor.
    #[arg(long)]
    cursor: Option<String>,
}

#[derive(Debug, Args)]
pub struct SetPositionAuthorityArgs {
    /// Trader wallet authority pubkey. Defaults to global --keypair when
    /// omitted.
    #[arg(long)]
    pub(crate) authority: Option<String>,
    /// New position authority pubkey.
    #[arg(long, visible_alias = "position-authority")]
    pub(crate) new_position_authority: String,
    /// Trader PDA index.
    #[arg(long, default_value_t = 0, visible_alias = "pda-index")]
    pub(crate) trader_pda_index: u8,
    /// Trader subaccount index.
    #[arg(long, default_value_t = 0, visible_alias = "subaccount-index")]
    pub(crate) trader_subaccount_index: u8,
    /// Explicit trader account address. When omitted, the CLI derives it from
    /// authority, trader PDA index, and subaccount index.
    #[arg(long)]
    pub(crate) trader_account: Option<String>,
}

#[derive(Debug, Args)]
pub struct ResetPositionAuthorityArgs {
    /// Trader wallet authority pubkey. Defaults to global --keypair when
    /// omitted.
    #[arg(long)]
    pub(crate) authority: Option<String>,
    /// Trader PDA index.
    #[arg(long, default_value_t = 0, visible_alias = "pda-index")]
    pub(crate) trader_pda_index: u8,
    /// Trader subaccount index.
    #[arg(long, default_value_t = 0, visible_alias = "subaccount-index")]
    pub(crate) trader_subaccount_index: u8,
    /// Explicit trader account address. When omitted, the CLI derives it from
    /// authority, trader PDA index, and subaccount index.
    #[arg(long)]
    pub(crate) trader_account: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TraderSummary {
    authority: String,
    pda_index: u8,
    subaccount_index: u8,
    trader: Option<TraderView>,
    position_diagnostics: Vec<PositionDiagnostics>,
    recent_pnl: serde_json::Value,
    recent_orders: serde_json::Value,
    recent_collateral: serde_json::Value,
    recent_funding: serde_json::Value,
    recent_trades: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionDiagnostics {
    symbol: String,
    unrealized_pnl: String,
    accumulated_funding: String,
    unsettled_funding: String,
    liquidation_price: String,
    stop_loss_price: Option<String>,
    take_profit_price: Option<String>,
    stop_loss_distance_pct: Option<f64>,
    liquidation_distance_pct: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterTraderOutput<Response: Serialize> {
    flow: &'static str,
    trader_authority: String,
    tx_fee_payer: String,
    trader_pda: String,
    trader_onboarder: String,
    max_positions: u32,
    include_register_trader: bool,
    recent_blockhash: String,
    signed_transaction: String,
    instructions: Vec<InstructionOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    permission: Option<ReferralActivationPermissionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trader_status: Option<String>,
    response: Response,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionAuthorityOutput {
    action: &'static str,
    authority: String,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    trader_account: String,
    new_position_authority: String,
    instructions: Vec<InstructionOutput>,
}

pub async fn run(cmd: TraderCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        TraderCommand::Register(args) => register(args, ctx).await,
        TraderCommand::State(args) => {
            crate::commands::api::run(
                ApiCommand::Trader {
                    authority: args.authority,
                    pda_index: args.pda_index,
                    subaccount_index: args.subaccount_index,
                },
                ctx,
            )
            .await
        }
        TraderCommand::Summary {
            authority,
            pda_index,
            subaccount_index,
            limit,
        } => summary(&authority, pda_index, subaccount_index, limit, ctx).await,
        TraderCommand::Pnl(args) => {
            crate::commands::api::run(
                ApiCommand::Pnl {
                    authority: args.authority,
                    resolution: args.resolution,
                    start_time: args.start_time,
                    end_time: args.end_time,
                    limit: args.limit,
                },
                ctx,
            )
            .await
        }
        TraderCommand::CollateralHistory(args) => {
            crate::commands::api::run(
                ApiCommand::CollateralHistory {
                    authority: args.authority,
                    pda_index: args.pda_index,
                    limit: args.limit,
                    next_cursor: args.next_cursor,
                    prev_cursor: args.prev_cursor,
                    cursor: args.cursor,
                },
                ctx,
            )
            .await
        }
        TraderCommand::TradeHistory(args) => {
            crate::commands::export::show_trade_history(args, ctx).await
        }
        TraderCommand::OrderHistory(args) => {
            crate::commands::export::show_order_history(args, ctx).await
        }
        TraderCommand::FundingHistory(args) => {
            crate::commands::export::show_funding_history(args, ctx).await
        }
        TraderCommand::FundingHourly(args) => {
            crate::commands::api::run(
                ApiCommand::FundingHourly {
                    authority: args.authority,
                    symbol: args.symbol,
                    pda_index: args.pda_index,
                    limit: args.limit,
                    cursor: args.cursor,
                },
                ctx,
            )
            .await
        }
        TraderCommand::LiquidationHistory(args) => {
            crate::commands::export::show_liquidation_history(args, ctx).await
        }
        TraderCommand::PlaceMarketOrder(args) => {
            crate::commands::trade::run(TradeCommand::IsolatedMarket(args), ctx).await
        }
        TraderCommand::PlaceLimitOrder(args) => {
            crate::commands::trade::run(TradeCommand::IsolatedLimit(args), ctx).await
        }
        TraderCommand::PlaceStopLoss(args) => {
            crate::commands::trade::run(TradeCommand::PlaceStopLoss(args), ctx).await
        }
        TraderCommand::CancelStopLoss(args) => {
            crate::commands::trade::run(TradeCommand::CancelStopLoss(args), ctx).await
        }
        TraderCommand::SetPositionAuthority(args) => set_position_authority(args, ctx),
        TraderCommand::ResetPositionAuthority(args) => reset_position_authority(args, ctx),
    }
}

fn set_position_authority(
    args: SetPositionAuthorityArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let authority = resolve_authority_or_keypair(args.authority.as_deref(), ctx)?;
    let new_position_authority = parse_pubkey(&args.new_position_authority)?;
    build_and_print_position_authority_ix(
        "set_position_authority",
        authority,
        new_position_authority,
        args.trader_pda_index,
        args.trader_subaccount_index,
        args.trader_account.as_deref(),
        ctx,
    )
}

fn reset_position_authority(
    args: ResetPositionAuthorityArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let authority = resolve_authority_or_keypair(args.authority.as_deref(), ctx)?;
    build_and_print_position_authority_ix(
        "reset_position_authority",
        authority,
        authority,
        args.trader_pda_index,
        args.trader_subaccount_index,
        args.trader_account.as_deref(),
        ctx,
    )
}

fn build_and_print_position_authority_ix(
    action: &'static str,
    authority: Pubkey,
    new_position_authority: Pubkey,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    explicit_trader_account: Option<&str>,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let trader_account = match explicit_trader_account {
        Some(trader_account) => parse_pubkey(trader_account)?,
        None => TraderKey::new_with_idx(authority, trader_pda_index, trader_subaccount_index).pda(),
    };
    let params = DelegateTraderParams::builder()
        .trader_wallet(authority)
        .trader_account(trader_account)
        .new_position_authority(new_position_authority)
        .build()?;
    let ix: Instruction = create_delegate_trader_ix(params)?.into();

    print_human_json(
        &PositionAuthorityOutput {
            action,
            authority: authority.to_string(),
            trader_pda_index,
            trader_subaccount_index,
            trader_account: trader_account.to_string(),
            new_position_authority: new_position_authority.to_string(),
            instructions: vec![InstructionOutput::from(&ix)],
        },
        ctx.output,
    )
}

async fn register(args: RegisterArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    if ctx.ledger.is_some() {
        return Err(
            "ledger signing is not wired for trader registration yet; use global --keypair or \
             --payer-keypair"
                .into(),
        );
    }
    validate_register_args(&args)?;

    let client = ctx.http_client()?;
    let rpc = RpcClient::new_with_commitment(ctx.rpc_url.clone(), CommitmentConfig::confirmed());

    if args.referral_code.is_some() {
        register_with_referral(&client, &rpc, args, ctx).await
    } else {
        register_without_referral(&client, &rpc, args, ctx).await
    }
}

async fn register_without_referral(
    client: &phoenix_rise::api::PhoenixHttpClient,
    rpc: &RpcClient,
    args: RegisterArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let payer_keypair = read_payer_keypair(ctx)?;
    let tx_fee_payer = payer_keypair.pubkey();
    let trader_authority =
        resolve_no_referral_authority(args.authority.as_deref(), &payer_keypair, ctx)?;

    let request = BuildRegisterIxsRequest {
        trader_authority: trader_authority.to_string(),
        tx_fee_payer: tx_fee_payer.to_string(),
        max_positions: Some(args.max_positions),
    };
    let built = client.exchange().build_register_ixs(&request).await?;
    let instructions = built
        .instructions
        .iter()
        .map(api_instruction_to_solana)
        .collect::<Result<Vec<_>, _>>()?;
    let (signed_transaction, recent_blockhash) = sign_instructions(
        &instructions,
        &payer_keypair,
        &[],
        args.recent_blockhash.as_deref(),
        rpc,
    )
    .await?;

    let submitted = client
        .exchange()
        .send_register_ixs(&SendRegisterIxsRequest {
            transaction: signed_transaction.clone(),
            trader_authority: trader_authority.to_string(),
            tx_fee_payer: tx_fee_payer.to_string(),
            max_positions: Some(args.max_positions),
            trader_pda_index: Some(args.trader_pda_index),
            trader_subaccount_index: Some(args.trader_subaccount_index),
        })
        .await?;

    print_register_output(
        "builder_onboarding",
        trader_authority,
        tx_fee_payer,
        &built,
        recent_blockhash,
        signed_transaction,
        instructions,
        None,
        None,
        submitted,
        ctx,
    )
}

async fn register_with_referral(
    client: &phoenix_rise::api::PhoenixHttpClient,
    rpc: &RpcClient,
    args: RegisterArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let authority_keypair = read_wallet_keypair(ctx.keypair_path.clone())?;
    let trader_authority =
        resolve_referral_authority(args.authority.as_deref(), &authority_keypair)?;
    let payer_keypair = read_payer_keypair(ctx)?;
    let tx_fee_payer = payer_keypair.pubkey();
    let trader = TraderKey::new_with_idx(
        trader_authority,
        args.trader_pda_index,
        args.trader_subaccount_index,
    );
    let referral_code = args
        .referral_code
        .as_deref()
        .expect("register_with_referral is only called when referral_code exists");
    let trader_status = fetch_referral_activation_trader_status(rpc, &trader.pda()).await?;
    let permission_response = client.invite().get_referral_activation_permission().await?;
    let trader_onboarder = parse_pubkey(&permission_response.trader_onboarder)?;
    let risk_authority = parse_pubkey(&permission_response.risk_authority)?;
    let permission_account = parse_pubkey(&permission_response.permission_account)?;
    let permission = fetch_permission_account(rpc, &permission_account).await?;
    validate_permission(&permission, risk_authority, trader_onboarder)?;

    let exchange = client.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);
    let keys = metadata.keys();
    let global_trader_index = parse_pubkey_list(&keys.global_trader_index)?;
    let active_trader_buffer = parse_pubkey_list(&keys.active_trader_buffer)?;
    let include_register_trader = trader_status.should_include_register_trader();
    let instructions = build_referral_activation_instructions(
        &trader,
        include_register_trader,
        args.max_positions,
        tx_fee_payer,
        tx_fee_payer != trader_authority,
        trader_onboarder,
        permission_account,
        global_trader_index,
        active_trader_buffer,
    )?;
    let additional_signers: Vec<&Keypair> = if tx_fee_payer == trader_authority {
        Vec::new()
    } else {
        vec![&authority_keypair]
    };
    let (signed_transaction, recent_blockhash) = sign_instructions(
        &instructions,
        &payer_keypair,
        &additional_signers,
        args.recent_blockhash.as_deref(),
        rpc,
    )
    .await?;

    let response = client
        .invite()
        .activate_referral_tx(&ActivateReferralTxRequest {
            referral_code: referral_code.to_string(),
            trader_authority: trader.authority().to_string(),
            trader_pda_index: Some(trader.pda_index),
            trader_subaccount_index: Some(trader.subaccount_index),
            recent_blockhash: recent_blockhash.clone(),
            transaction: signed_transaction.clone(),
        })
        .await?;

    let build_response = BuildRegisterIxsResponse {
        instructions: Vec::new(),
        trader_pda: trader.pda().to_string(),
        trader_onboarder: trader_onboarder.to_string(),
        tx_fee_payer: tx_fee_payer.to_string(),
        max_positions: args.max_positions,
        include_register_trader,
    };
    print_register_output(
        "referral_activation",
        trader.authority(),
        tx_fee_payer,
        &build_response,
        recent_blockhash,
        signed_transaction,
        instructions,
        Some(permission_response),
        Some(format!("{trader_status:?}")),
        response,
        ctx,
    )
}

async fn sign_instructions(
    instructions: &[Instruction],
    fee_payer_keypair: &Keypair,
    additional_signers: &[&Keypair],
    recent_blockhash: Option<&str>,
    rpc: &RpcClient,
) -> Result<(String, String), Box<dyn Error>> {
    let recent_blockhash = match recent_blockhash {
        Some(blockhash) => blockhash.parse()?,
        None => rpc.get_latest_blockhash().await?,
    };
    let mut transaction =
        Transaction::new_with_payer(instructions, Some(&fee_payer_keypair.pubkey()));
    let mut signers = Vec::with_capacity(1 + additional_signers.len());
    signers.push(fee_payer_keypair);
    for signer in additional_signers {
        if !signers
            .iter()
            .any(|existing| existing.pubkey() == signer.pubkey())
        {
            signers.push(*signer);
        }
    }
    transaction.try_partial_sign(&signers, recent_blockhash)?;
    let versioned = VersionedTransaction::from(transaction);
    Ok((
        encode_transaction(&versioned)?,
        recent_blockhash.to_string(),
    ))
}

fn build_referral_activation_instructions(
    trader: &TraderKey,
    include_register_trader: bool,
    max_positions: u32,
    tx_fee_payer: Pubkey,
    include_trader_authority_signer: bool,
    trader_onboarder: Pubkey,
    permission_account: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
) -> Result<Vec<Instruction>, Box<dyn Error>> {
    let mut instructions = Vec::with_capacity(if include_register_trader { 2 } else { 1 });
    if include_register_trader {
        let register_params = RegisterTraderParams::builder()
            .payer(tx_fee_payer)
            .trader(trader.authority())
            .trader_account(trader.pda())
            .max_positions(u64::from(max_positions))
            .trader_pda_index(trader.pda_index)
            .subaccount_index(trader.subaccount_index)
            .build()?;
        instructions.push(create_register_trader_ix(register_params)?.into());
    }

    let onboard_params = OnboardTraderDelegatedParams::builder()
        .authority(trader_onboarder)
        .permission_account(permission_account)
        .trader_account(trader.pda())
        .global_trader_index(global_trader_index)
        .active_trader_buffer(active_trader_buffer)
        .build()?;
    let mut onboard_ix: Instruction = create_onboard_trader_delegated_ix(onboard_params)?.into();
    if include_trader_authority_signer {
        onboard_ix
            .accounts
            .push(AccountMeta::new_readonly(trader.authority(), true));
    }
    instructions.push(onboard_ix);
    Ok(instructions)
}

async fn fetch_permission_account(
    rpc: &RpcClient,
    permission_account: &Pubkey,
) -> Result<Permission, Box<dyn Error>> {
    let account = rpc
        .get_account_with_commitment(permission_account, CommitmentConfig::confirmed())
        .await?
        .value
        .ok_or_else(|| format!("permission account does not exist: {permission_account}"))?;
    Ok(Permission::try_from_account_bytes(&account.data)?)
}

fn validate_permission(
    permission: &Permission,
    expected_risk_authority: Pubkey,
    expected_onboarder: Pubkey,
) -> Result<(), Box<dyn Error>> {
    if permission.permission_authority != expected_risk_authority {
        return Err(format!(
            "permission authority mismatch: expected {expected_risk_authority}, got {}",
            permission.permission_authority
        )
        .into());
    }
    if permission.delegated_key != expected_onboarder {
        return Err(format!(
            "permission delegated key mismatch: expected {expected_onboarder}, got {}",
            permission.delegated_key
        )
        .into());
    }
    if permission.permission & TRADER_ONBOARDING_PERMISSION != TRADER_ONBOARDING_PERMISSION {
        return Err("permission account is missing trader-onboarding permission".into());
    }
    if permission.allowed_signer_actions == 0 {
        return Err("permission account has no remaining signer actions".into());
    }
    Ok(())
}

fn parse_pubkey_list(values: &[String]) -> Result<Vec<Pubkey>, Box<dyn Error>> {
    values.iter().map(|value| parse_pubkey(value)).collect()
}

fn read_payer_keypair(ctx: &CommandCtx) -> Result<Keypair, Box<dyn Error>> {
    read_wallet_keypair(
        ctx.payer_keypair_path
            .clone()
            .or_else(|| ctx.keypair_path.clone()),
    )
}

fn resolve_no_referral_authority(
    authority: Option<&str>,
    payer_keypair: &Keypair,
    ctx: &CommandCtx,
) -> Result<Pubkey, Box<dyn Error>> {
    if let Some(authority) = authority {
        return parse_pubkey(authority);
    }

    if ctx.keypair_path.is_some() {
        return Ok(read_wallet_keypair(ctx.keypair_path.clone())?.pubkey());
    }

    // Builder onboarding does not require the trader authority signature. If
    // no authority or explicit global keypair is provided, register the payer
    // authority by default.
    Ok(payer_keypair.pubkey())
}

fn resolve_authority_or_keypair(
    authority: Option<&str>,
    ctx: &CommandCtx,
) -> Result<Pubkey, Box<dyn Error>> {
    if let Some(authority) = authority {
        return parse_pubkey(authority);
    }
    if ctx.ledger.is_some() {
        return Err(
            "pass --authority when using --ledger for position-authority instructions".into(),
        );
    }
    Ok(read_wallet_keypair(ctx.keypair_path.clone())?.pubkey())
}

fn resolve_referral_authority(
    authority: Option<&str>,
    authority_keypair: &Keypair,
) -> Result<Pubkey, Box<dyn Error>> {
    let keypair_pubkey = authority_keypair.pubkey();
    let Some(authority) = authority else {
        return Ok(keypair_pubkey);
    };
    let authority = parse_pubkey(authority)?;
    if authority != keypair_pubkey {
        return Err(format!(
            "--authority ({authority}) must match global --keypair ({keypair_pubkey}) when \
             --referral-code is used"
        )
        .into());
    }
    Ok(authority)
}

fn validate_register_args(args: &RegisterArgs) -> Result<(), Box<dyn Error>> {
    if !(MIN_MAX_POSITIONS..=DEFAULT_MAX_POSITIONS).contains(&args.max_positions) {
        return Err(format!(
            "--max-positions must be between {MIN_MAX_POSITIONS} and {DEFAULT_MAX_POSITIONS}"
        )
        .into());
    }
    if args.trader_pda_index != 0 {
        return Err("--trader-pda-index must be 0 for the public onboarding endpoints".into());
    }
    if args.trader_subaccount_index != 0 {
        return Err(
            "--trader-subaccount-index must be 0 for the public onboarding endpoints".into(),
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn print_register_output<Response: Serialize>(
    flow: &'static str,
    trader_authority: Pubkey,
    tx_fee_payer: Pubkey,
    built: &BuildRegisterIxsResponse,
    recent_blockhash: String,
    signed_transaction: String,
    instructions: Vec<Instruction>,
    permission: Option<ReferralActivationPermissionResponse>,
    trader_status: Option<String>,
    response: Response,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let instructions = instructions.iter().map(InstructionOutput::from).collect();
    print_human_json(
        &RegisterTraderOutput {
            flow,
            trader_authority: trader_authority.to_string(),
            tx_fee_payer: tx_fee_payer.to_string(),
            trader_pda: built.trader_pda.clone(),
            trader_onboarder: built.trader_onboarder.clone(),
            max_positions: built.max_positions,
            include_register_trader: built.include_register_trader,
            recent_blockhash,
            signed_transaction,
            instructions,
            permission,
            trader_status,
            response,
        },
        ctx.output,
    )
}

async fn summary(
    authority: &str,
    pda_index: u8,
    subaccount_index: u8,
    limit: i64,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let client = ctx.http_client()?;
    let authority_key = parse_pubkey(authority)?;

    let trader = client
        .traders()
        .get_trader_subaccount(&authority_key, pda_index, subaccount_index)
        .await?;
    let recent_pnl = client
        .get_user_pnl(
            &authority_key,
            PnlQueryParams::new(PnlResolution::Day1).with_limit(limit),
        )
        .await
        .map(serde_json::to_value)??;
    let recent_orders = client
        .get_order_history(
            &authority_key,
            OrderHistoryQueryParams::new(limit).with_pda_index(pda_index),
        )
        .await
        .map(serde_json::to_value)??;
    let recent_collateral = client
        .get_collateral_history(
            &authority_key,
            CollateralHistoryQueryParams::new(limit).with_pda_index(pda_index),
        )
        .await
        .map(serde_json::to_value)??;
    let recent_funding = client
        .get_funding_history(
            &authority_key,
            FundingHistoryQueryParams::new()
                .with_pda_index(pda_index)
                .with_limit(limit),
        )
        .await
        .map(serde_json::to_value)??;
    let recent_trades = client
        .get_trade_history(
            &authority_key,
            TradeHistoryQueryParams::new()
                .with_pda_index(pda_index)
                .with_limit(limit),
        )
        .await
        .map(serde_json::to_value)??;

    let position_diagnostics = trader
        .as_ref()
        .map(|trader| {
            trader
                .positions
                .iter()
                .map(position_diagnostics)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    print_human_json(
        &TraderSummary {
            authority: authority.to_string(),
            pda_index,
            subaccount_index,
            trader,
            position_diagnostics,
            recent_pnl,
            recent_orders,
            recent_collateral,
            recent_funding,
            recent_trades,
        },
        ctx.output,
    )
}

fn position_diagnostics(position: &TraderPositionView) -> PositionDiagnostics {
    let entry = parse_decimal_ui(&position.entry_price.ui);
    let stop_loss = position
        .stop_loss_price
        .as_ref()
        .and_then(|price| parse_decimal_ui(&price.ui));
    let liquidation = parse_decimal_ui(&position.liquidation_price.ui);

    PositionDiagnostics {
        symbol: position.symbol.clone(),
        unrealized_pnl: position.unrealized_pnl.ui.clone(),
        accumulated_funding: position.accumulated_funding.ui.clone(),
        unsettled_funding: position.unsettled_funding.ui.clone(),
        liquidation_price: position.liquidation_price.ui.clone(),
        stop_loss_price: position
            .stop_loss_price
            .as_ref()
            .map(|price| price.ui.clone()),
        take_profit_price: position
            .take_profit_price
            .as_ref()
            .map(|price| price.ui.clone()),
        stop_loss_distance_pct: distance_pct(entry, stop_loss),
        liquidation_distance_pct: distance_pct(entry, liquidation),
    }
}

fn parse_decimal_ui(value: &str) -> Option<f64> {
    f64::from_str(value).ok().filter(|value| value.is_finite())
}

fn distance_pct(reference: Option<f64>, target: Option<f64>) -> Option<f64> {
    let reference = reference?;
    let target = target?;
    if reference == 0.0 {
        return None;
    }
    Some(((target - reference) / reference).abs() * 100.0)
}
