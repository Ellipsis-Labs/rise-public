use std::error::Error;

use clap::{Args, Subcommand, ValueEnum};
use phoenix_rise::ix::types::Side;
use phoenix_rise::types::prelude::{
    CancelStopLossOrderRequest, PlaceIsolatedLimitOrderRequest, PlaceIsolatedMarketOrderRequest,
    PlaceStopLossOrderRequest, StopLossExecutionDirection, TpSlOrderConfig,
};
use serde::Serialize;
use solana_signer::Signer;

use crate::context::CommandCtx;
use crate::output::{InstructionOutput, print_json};
use crate::util::{parse_pubkey, read_wallet_keypair};

#[derive(Debug, Subcommand)]
pub enum TradeCommand {
    IsolatedMarket(IsolatedMarketArgs),
    IsolatedLimit(IsolatedLimitArgs),
    PlaceStopLoss(PlaceStopLossArgs),
    CancelStopLoss(CancelStopLossArgs),
}

#[derive(Debug, Args)]
pub struct IsolatedMarketArgs {
    #[arg(long)]
    pub(crate) authority: String,
    #[arg(long)]
    pub(crate) symbol: String,
    #[arg(long, value_enum)]
    pub(crate) side: SideArg,
    #[arg(long)]
    pub(crate) num_base_lots: Option<u64>,
    #[arg(long)]
    pub(crate) quantity: Option<f64>,
    #[arg(long, default_value_t = 0)]
    pub(crate) transfer_amount: u64,
    #[arg(long)]
    pub(crate) max_price_in_ticks: Option<u64>,
    #[arg(long)]
    pub(crate) pda_index: Option<u8>,
    #[arg(long)]
    pub(crate) fee_payer: Option<String>,
    #[arg(long)]
    pub(crate) position_authority: Option<String>,
    #[arg(long)]
    pub(crate) reduce_only: Option<bool>,
    #[arg(long)]
    pub(crate) skip_transfer_to_parent: Option<bool>,
    #[arg(long, default_value_t = true)]
    pub(crate) allow_cross_and_isolated: bool,
    #[arg(long)]
    pub(crate) enhanced: bool,
}

#[derive(Debug, Args)]
pub struct IsolatedLimitArgs {
    #[arg(long)]
    authority: String,
    #[arg(long)]
    symbol: String,
    #[arg(long, value_enum)]
    side: SideArg,
    #[arg(long)]
    price: Option<f64>,
    #[arg(long)]
    price_in_ticks: Option<u64>,
    #[arg(long)]
    num_base_lots: Option<u64>,
    #[arg(long)]
    quantity: Option<f64>,
    #[arg(long, default_value_t = 0)]
    transfer_amount: u64,
    #[arg(long)]
    pda_index: Option<u8>,
    #[arg(long)]
    fee_payer: Option<String>,
    #[arg(long)]
    position_authority: Option<String>,
    #[arg(long)]
    reduce_only: Option<bool>,
    #[arg(long)]
    post_only: Option<bool>,
    #[arg(long)]
    slide: Option<bool>,
    #[arg(long)]
    skip_transfer_to_parent: Option<bool>,
    #[arg(long, default_value_t = true)]
    allow_cross_and_isolated: bool,
    #[arg(long)]
    enhanced: bool,
}

#[derive(Debug, Args)]
pub struct PlaceStopLossArgs {
    #[arg(long)]
    authority: String,
    #[arg(long)]
    symbol: String,
    #[arg(long, value_enum)]
    side: SideArg,
    #[arg(long, default_value_t = 0)]
    trader_pda_index: u8,
    #[arg(long)]
    trader_subaccount_index: Option<u8>,
    #[arg(long)]
    isolated: bool,
    #[arg(long)]
    position_authority: Option<String>,
    #[arg(long)]
    fee_payer: Option<String>,
    #[arg(long)]
    stop_loss_trigger_price: Option<f64>,
    #[arg(long)]
    stop_loss_trigger_price_in_ticks: Option<u64>,
    #[arg(long)]
    stop_loss_execution_price: Option<f64>,
    #[arg(long)]
    stop_loss_execution_price_in_ticks: Option<u64>,
    #[arg(long)]
    take_profit_trigger_price: Option<f64>,
    #[arg(long)]
    take_profit_execution_price: Option<f64>,
    #[arg(long)]
    order_kind: Option<String>,
    #[arg(long)]
    num_base_lots: Option<u64>,
    #[arg(long)]
    quantity: Option<f64>,
}

#[derive(Debug, Args)]
pub struct CancelStopLossArgs {
    #[arg(long)]
    authority: String,
    #[arg(long)]
    symbol: String,
    #[arg(long, default_value_t = 0)]
    trader_pda_index: u8,
    #[arg(long)]
    trader_subaccount_index: Option<u8>,
    #[arg(long)]
    isolated: bool,
    #[arg(long)]
    position_authority: Option<String>,
    #[arg(long, value_enum)]
    execution_direction: StopLossDirectionArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SideArg {
    Buy,
    Sell,
    Bid,
    Ask,
}

impl SideArg {
    fn to_ix_side(self) -> Side {
        match self {
            Self::Buy | Self::Bid => Side::Bid,
            Self::Sell | Self::Ask => Side::Ask,
        }
    }

    fn to_api_string(self) -> &'static str {
        self.to_ix_side().to_api_string()
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StopLossDirectionArg {
    GreaterThan,
    LessThan,
}

impl From<StopLossDirectionArg> for StopLossExecutionDirection {
    fn from(value: StopLossDirectionArg) -> Self {
        match value {
            StopLossDirectionArg::GreaterThan => Self::GreaterThan,
            StopLossDirectionArg::LessThan => Self::LessThan,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstructionBundle<T: Serialize> {
    request: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_liquidation_price_usd: Option<f64>,
    instructions: Vec<InstructionOutput>,
}

pub async fn run(cmd: TradeCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        TradeCommand::IsolatedMarket(args) => isolated_market(args, ctx).await,
        TradeCommand::IsolatedLimit(args) => isolated_limit(args, ctx).await,
        TradeCommand::PlaceStopLoss(args) => place_stop_loss(args, ctx).await,
        TradeCommand::CancelStopLoss(args) => cancel_stop_loss(args, ctx).await,
    }
}

async fn isolated_market(args: IsolatedMarketArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    validate_size(args.num_base_lots, args.quantity)?;
    let client = ctx.http_client()?;
    let request = PlaceIsolatedMarketOrderRequest {
        authority: parse_pubkey(&args.authority)?.to_string(),
        position_authority: default_position_authority(
            ctx,
            &args.authority,
            args.position_authority,
        )?,
        symbol: args.symbol.to_ascii_uppercase(),
        side: args.side.to_api_string().to_string(),
        num_base_lots: args.num_base_lots,
        quantity: args.quantity,
        transfer_amount: args.transfer_amount,
        max_price_in_ticks: args.max_price_in_ticks,
        pda_index: args.pda_index,
        allow_cross_and_isolated_for_asset: Some(args.allow_cross_and_isolated),
        fee_payer: default_fee_payer(ctx, args.fee_payer)?,
        is_reduce_only: args.reduce_only,
        skip_transfer_to_parent: args.skip_transfer_to_parent,
        ..Default::default()
    };
    if args.enhanced {
        let (ixs, estimated_liquidation_price_usd) = client
            .build_isolated_market_order_tx_enhanced_with_request(request.clone())
            .await?;
        print_instruction_bundle(request, Some(estimated_liquidation_price_usd), ixs, ctx)
    } else {
        let ixs = client
            .build_isolated_market_order_tx_with_request(request.clone())
            .await?;
        print_instruction_bundle(request, None, ixs, ctx)
    }
}

async fn isolated_limit(args: IsolatedLimitArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    validate_size(args.num_base_lots, args.quantity)?;
    if args.price.is_none() && args.price_in_ticks.is_none() {
        return Err("one of --price or --price-in-ticks is required".into());
    }

    let client = ctx.http_client()?;
    let request = PlaceIsolatedLimitOrderRequest {
        authority: parse_pubkey(&args.authority)?.to_string(),
        position_authority: default_position_authority(
            ctx,
            &args.authority,
            args.position_authority,
        )?,
        symbol: args.symbol.to_ascii_uppercase(),
        side: args.side.to_api_string().to_string(),
        price: args.price,
        price_in_ticks: args.price_in_ticks,
        num_base_lots: args.num_base_lots,
        quantity: args.quantity,
        transfer_amount: args.transfer_amount,
        pda_index: args.pda_index,
        allow_cross_and_isolated_for_asset: Some(args.allow_cross_and_isolated),
        fee_payer: default_fee_payer(ctx, args.fee_payer)?,
        is_reduce_only: args.reduce_only,
        is_post_only: args.post_only,
        slide: args.slide,
        skip_transfer_to_parent: args.skip_transfer_to_parent,
        ..Default::default()
    };
    if args.enhanced {
        let (ixs, estimated_liquidation_price_usd) = client
            .build_isolated_limit_order_tx_enhanced_with_request(request.clone())
            .await?;
        print_instruction_bundle(request, Some(estimated_liquidation_price_usd), ixs, ctx)
    } else {
        let ixs = client
            .build_isolated_limit_order_tx_with_request(request.clone())
            .await?;
        print_instruction_bundle(request, None, ixs, ctx)
    }
}

async fn place_stop_loss(args: PlaceStopLossArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let client = ctx.http_client()?;
    let request = PlaceStopLossOrderRequest {
        authority: parse_pubkey(&args.authority)?.to_string(),
        position_authority: default_position_authority(
            ctx,
            &args.authority,
            args.position_authority,
        )?,
        trader_pda_index: args.trader_pda_index,
        trader_subaccount_index: args.trader_subaccount_index,
        is_isolated: args.isolated,
        symbol: args.symbol.to_ascii_uppercase(),
        side: args.side.to_api_string().to_string(),
        fee_payer: default_fee_payer(ctx, args.fee_payer)?,
        tp_sl: TpSlOrderConfig {
            take_profit_trigger_price: args.take_profit_trigger_price,
            take_profit_execution_price: args.take_profit_execution_price,
            stop_loss_trigger_price: args.stop_loss_trigger_price,
            stop_loss_trigger_price_in_ticks: args.stop_loss_trigger_price_in_ticks,
            stop_loss_execution_price: args.stop_loss_execution_price,
            stop_loss_execution_price_in_ticks: args.stop_loss_execution_price_in_ticks,
            order_kind: args.order_kind,
            num_base_lots: args.num_base_lots,
            quantity: args.quantity,
            ..Default::default()
        },
        ..Default::default()
    };
    let ixs = client.place_stop_loss_order(request.clone()).await?;
    print_instruction_bundle(request, None, ixs, ctx)
}

async fn cancel_stop_loss(
    args: CancelStopLossArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let client = ctx.http_client()?;
    let request = CancelStopLossOrderRequest {
        authority: parse_pubkey(&args.authority)?.to_string(),
        position_authority: default_position_authority(
            ctx,
            &args.authority,
            args.position_authority,
        )?,
        trader_pda_index: args.trader_pda_index,
        trader_subaccount_index: args.trader_subaccount_index,
        is_isolated: args.isolated,
        symbol: args.symbol.to_ascii_uppercase(),
        execution_direction: args.execution_direction.into(),
    };
    let ixs = client.cancel_stop_loss_order(request.clone()).await?;
    print_instruction_bundle(request, None, ixs, ctx)
}

fn validate_size(num_base_lots: Option<u64>, quantity: Option<f64>) -> Result<(), Box<dyn Error>> {
    if num_base_lots.is_none() && quantity.is_none() {
        return Err("one of --num-base-lots or --quantity is required".into());
    }
    Ok(())
}

fn default_fee_payer(
    ctx: &CommandCtx,
    explicit_fee_payer: Option<String>,
) -> Result<Option<String>, Box<dyn Error>> {
    if explicit_fee_payer.is_some() {
        return Ok(explicit_fee_payer);
    }
    let Some(path) = ctx
        .payer_keypair_path
        .clone()
        .or_else(|| ctx.keypair_path.clone())
    else {
        return Ok(None);
    };
    Ok(Some(read_wallet_keypair(Some(path))?.pubkey().to_string()))
}

fn default_position_authority(
    ctx: &CommandCtx,
    authority: &str,
    explicit_position_authority: Option<String>,
) -> Result<Option<String>, Box<dyn Error>> {
    if explicit_position_authority.is_some() {
        return Ok(explicit_position_authority);
    }
    let Some(path) = ctx.keypair_path.clone() else {
        return Ok(None);
    };
    let keypair_pubkey = read_wallet_keypair(Some(path))?.pubkey();
    if keypair_pubkey == parse_pubkey(authority)? {
        return Ok(None);
    }
    // TODO: add delegated signer handling by reading trader state/capabilities
    // so the CLI can distinguish delegated keys from position-authority keys.
    Ok(Some(keypair_pubkey.to_string()))
}

fn print_instruction_bundle<T: Serialize>(
    request: T,
    estimated_liquidation_price_usd: Option<Option<f64>>,
    instructions: Vec<solana_instruction::Instruction>,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    print_json(
        &InstructionBundle {
            request,
            estimated_liquidation_price_usd: estimated_liquidation_price_usd.flatten(),
            instructions: instructions.iter().map(InstructionOutput::from).collect(),
        },
        ctx.output,
    )
}
