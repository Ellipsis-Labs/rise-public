use std::error::Error;

use clap::{Args, Subcommand};
use phoenix_rise::core::{PhoenixMetadata, PhoenixTxBuilder, TraderKey};
use phoenix_rise::ix::claim_fees::{ClaimFeesParams, create_claim_fees_ix};
use phoenix_rise::ix::constants::{get_associated_token_address, usdc_mint};
use phoenix_rise::ix::discriminants::FlightAccount;
use phoenix_rise::ix::flight::{
    RegisterBuilderParams, UpdateFeeParams, create_register_builder_ix, create_update_fee_ix,
    get_flight_builder_state_address,
};
use serde::Serialize;
use solana_commitment_config::CommitmentConfig;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::context::CommandCtx;
use crate::output::{InstructionOutput, print_json};
use crate::util::parse_pubkey;

const USDC_BASE_UNITS_PER_TOKEN: f64 = 1_000_000.0;

#[derive(Debug, Subcommand)]
pub enum FlightCommand {
    /// Show a Flight builder's config and current withdrawable fee collateral.
    View(ViewArgs),
    /// Build a Flight register-builder instruction.
    RegisterBuilder(RegisterBuilderArgs),
    /// Build a Flight update-fee instruction.
    UpdateFee(UpdateFeeArgs),
    /// Build the Phoenix/Ember withdrawal flow for a Flight builder's
    /// collateralized fees.
    #[command(alias = "withdraw-fees", alias = "withdraw-builder-fees")]
    WithdrawCollateral(WithdrawCollateralArgs),
    /// Build a Phoenix risk-authority protocol fee claim instruction.
    #[command(alias = "claim-protocol-fees")]
    ClaimFees(ClaimFeesArgs),
}

#[derive(Debug, Args)]
pub struct ViewArgs {
    /// Builder authority pubkey.
    #[arg(long)]
    pub(crate) authority: String,
}

#[derive(Debug, Args)]
pub struct RegisterBuilderArgs {
    /// Builder authority pubkey. This account signs the transaction.
    #[arg(long)]
    pub(crate) authority: String,
    /// Builder trader account. Defaults to the trader PDA derived from
    /// authority, --trader-pda-index, and --subaccount-index.
    #[arg(long)]
    pub(crate) trader_account: Option<String>,
    /// Phoenix trader PDA index.
    #[arg(long, default_value_t = 0, visible_alias = "pda-index")]
    pub(crate) trader_pda_index: u8,
    /// Phoenix subaccount index.
    #[arg(long, default_value_t = 0)]
    pub(crate) subaccount_index: u8,
    /// Builder fee in basis points.
    #[arg(long)]
    pub(crate) fee_bps: u64,
}

#[derive(Debug, Args)]
pub struct UpdateFeeArgs {
    /// Builder authority pubkey. This account signs the transaction.
    #[arg(long)]
    pub(crate) authority: String,
    /// New builder fee in basis points.
    #[arg(long)]
    pub(crate) fee_bps: u64,
}

#[derive(Debug, Args)]
pub struct WithdrawCollateralArgs {
    /// Builder authority pubkey. This account signs the withdrawal flow.
    #[arg(long)]
    pub(crate) authority: String,
    /// Builder trader account. Defaults to the trader PDA derived from
    /// authority, --trader-pda-index, and --subaccount-index.
    #[arg(long)]
    pub(crate) trader_account: Option<String>,
    /// Phoenix trader PDA index.
    #[arg(long, default_value_t = 0, visible_alias = "pda-index")]
    pub(crate) trader_pda_index: u8,
    /// Phoenix subaccount index.
    #[arg(long, default_value_t = 0)]
    pub(crate) subaccount_index: u8,
    /// Amount to withdraw in canonical token base units.
    #[arg(long, conflicts_with = "usdc_amount")]
    pub(crate) amount: Option<u64>,
    /// Amount to withdraw in USDC units. Prefer --amount for exact scripts.
    #[arg(long)]
    pub(crate) usdc_amount: Option<f64>,
}

#[derive(Debug, Args)]
pub struct ClaimFeesArgs {
    /// Risk authority pubkey. This account signs the claim-fees instruction.
    #[arg(long)]
    pub(crate) authority: String,
    /// Phoenix global vault. Defaults to /v1/exchange/keys.
    #[arg(long)]
    pub(crate) global_vault: Option<String>,
    /// Destination canonical token account. Defaults to the authority ATA.
    #[arg(long)]
    pub(crate) destination: Option<String>,
    /// Amount to claim in canonical token base units.
    #[arg(long, conflicts_with = "usdc_amount")]
    pub(crate) amount: Option<u64>,
    /// Amount to claim in USDC units. Prefer --amount for exact scripts.
    #[arg(long)]
    pub(crate) usdc_amount: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FlightInstructionBundle<T: Serialize> {
    command: &'static str,
    request: T,
    required_signers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ledger: Option<String>,
    instructions: Vec<InstructionOutput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterBuilderPlan {
    authority: String,
    trader_account: String,
    trader_pda_index: u8,
    subaccount_index: u8,
    builder_state: String,
    fee_bps: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateFeePlan {
    authority: String,
    builder_state: String,
    fee_bps: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WithdrawCollateralPlan {
    authority: String,
    trader_account: String,
    trader_pda_index: u8,
    subaccount_index: u8,
    amount: u64,
    destination_usdc_account: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimFeesPlan {
    authority: String,
    global_vault: String,
    destination: String,
    amount: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FlightBuilderView {
    authority: String,
    builder_state_account: String,
    configured_authority: String,
    trader_account: String,
    active: bool,
    status_bits: u64,
    fee_bps: u64,
    trader_authority: String,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    quote_lot_collateral: i64,
    quote_lot_collateral_usdc: String,
    withdrawable_fee_quote_lots: u64,
    withdrawable_fee_usdc: String,
    trader_can_withdraw: bool,
    position_count: u64,
    note: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct FlightBuilderState {
    authority_key: Pubkey,
    trader_key: Pubkey,
    status: u64,
    fee_bps: u64,
}

pub async fn run(cmd: FlightCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        FlightCommand::View(args) => view(args, ctx).await,
        FlightCommand::RegisterBuilder(args) => register_builder(args, ctx),
        FlightCommand::UpdateFee(args) => update_fee(args, ctx),
        FlightCommand::WithdrawCollateral(args) => withdraw_collateral(args, ctx).await,
        FlightCommand::ClaimFees(args) => claim_fees(args, ctx).await,
    }
}

async fn view(args: ViewArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let builder_state_account = get_flight_builder_state_address(&authority)?;
    let rpc = RpcClient::new_with_commitment(ctx.rpc_url.clone(), CommitmentConfig::confirmed());
    let builder_account = rpc.get_account(&builder_state_account).await?;
    let builder_state = FlightBuilderState::try_from_account_bytes(&builder_account.data)?;
    let trader_account = rpc.get_account(&builder_state.trader_key).await?;
    let trader =
        phoenix_rise::accounts::trader::Trader::try_from_account_bytes(&trader_account.data)?;
    let trader_header = trader.header();
    let trader_state = trader_header.state();
    let quote_lot_collateral = trader_state.quote_lot_collateral.as_inner();
    let withdrawable_fee_quote_lots = quote_lot_collateral.max(0).unsigned_abs();
    let configured_authority = builder_state.authority_key;
    let output = FlightBuilderView {
        authority: authority.to_string(),
        builder_state_account: builder_state_account.to_string(),
        configured_authority: configured_authority.to_string(),
        trader_account: builder_state.trader_key.to_string(),
        active: builder_state.is_active(),
        status_bits: builder_state.status,
        fee_bps: builder_state.fee_bps,
        trader_authority: Pubkey::new_from_array(*trader_header.authority()).to_string(),
        trader_pda_index: trader_header.trader_pda_index(),
        trader_subaccount_index: trader_header.trader_subaccount_index(),
        quote_lot_collateral,
        quote_lot_collateral_usdc: format_signed_base_units(quote_lot_collateral),
        withdrawable_fee_quote_lots,
        withdrawable_fee_usdc: format_base_units(withdrawable_fee_quote_lots),
        trader_can_withdraw: trader_state.capability_flags().allows(
            phoenix_rise::accounts::trader::capabilities::TraderCapabilityKind::WithdrawCollateral,
        ),
        position_count: trader.raw_len(),
        note: "withdrawableFeeQuoteLots is the positive collateral in the builder trader account; \
               open positions, orders, queues, or risk checks can reduce the amount a withdrawal \
               transaction may settle",
    };

    if ctx.output.json || ctx.output.pretty {
        print_json(&output, ctx.output)
    } else {
        print_builder_view(&output);
        Ok(())
    }
}

fn register_builder(args: RegisterBuilderArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let trader_account = resolve_trader_account(
        authority,
        args.trader_account.as_deref(),
        args.trader_pda_index,
        args.subaccount_index,
    )?;
    let builder_state = get_flight_builder_state_address(&authority)?;
    let params = RegisterBuilderParams::builder()
        .trader_authority(authority)
        .trader_account(trader_account)
        .trader_pda_index(args.trader_pda_index)
        .subaccount_index(args.subaccount_index)
        .fee_bps(args.fee_bps)
        .build()?;
    let ix: Instruction = create_register_builder_ix(params)?.into();

    print_flight_bundle(
        "registerBuilder",
        RegisterBuilderPlan {
            authority: authority.to_string(),
            trader_account: trader_account.to_string(),
            trader_pda_index: args.trader_pda_index,
            subaccount_index: args.subaccount_index,
            builder_state: builder_state.to_string(),
            fee_bps: args.fee_bps,
        },
        vec![authority],
        vec![ix],
        ctx,
    )
}

fn update_fee(args: UpdateFeeArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let builder_state = get_flight_builder_state_address(&authority)?;
    let params = UpdateFeeParams::builder()
        .trader_authority(authority)
        .fee_bps(args.fee_bps)
        .build()?;
    let ix: Instruction = create_update_fee_ix(params)?.into();

    print_flight_bundle(
        "updateFee",
        UpdateFeePlan {
            authority: authority.to_string(),
            builder_state: builder_state.to_string(),
            fee_bps: args.fee_bps,
        },
        vec![authority],
        vec![ix],
        ctx,
    )
}

async fn withdraw_collateral(
    args: WithdrawCollateralArgs,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let trader_account = resolve_trader_account(
        authority,
        args.trader_account.as_deref(),
        args.trader_pda_index,
        args.subaccount_index,
    )?;
    let amount = resolve_base_units(args.amount, args.usdc_amount, "withdraw")?;
    let exchange = ctx.http_client()?.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);
    let builder = PhoenixTxBuilder::new(&metadata);
    let instructions =
        builder.build_withdraw_funds_base_units(authority, trader_account, amount)?;
    let destination_usdc_account = get_associated_token_address(&authority, &usdc_mint())?;

    print_flight_bundle(
        "withdrawCollateral",
        WithdrawCollateralPlan {
            authority: authority.to_string(),
            trader_account: trader_account.to_string(),
            trader_pda_index: args.trader_pda_index,
            subaccount_index: args.subaccount_index,
            amount,
            destination_usdc_account: destination_usdc_account.to_string(),
        },
        vec![authority],
        instructions,
        ctx,
    )
}

async fn claim_fees(args: ClaimFeesArgs, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let authority = parse_pubkey(&args.authority)?;
    let amount = resolve_base_units(args.amount, args.usdc_amount, "claim")?;
    let keys = if args.global_vault.is_none() || args.destination.is_none() {
        Some(ctx.http_client()?.get_exchange_keys().await?)
    } else {
        None
    };
    let global_vault = match args.global_vault {
        Some(value) => parse_pubkey(&value)?,
        None => parse_pubkey(
            &keys
                .as_ref()
                .expect("exchange keys were fetched when global_vault is missing")
                .global_vault,
        )?,
    };
    let destination = match args.destination {
        Some(value) => parse_pubkey(&value)?,
        None => {
            let canonical_mint = parse_pubkey(
                &keys
                    .as_ref()
                    .expect("exchange keys were fetched when destination is missing")
                    .canonical_mint,
            )?;
            get_associated_token_address(&authority, &canonical_mint)?
        }
    };
    let params = ClaimFeesParams::builder()
        .authority(authority)
        .global_vault(global_vault)
        .destination(destination)
        .amount(amount)
        .build()?;
    let ix: Instruction = create_claim_fees_ix(params)?.into();

    print_flight_bundle(
        "claimFees",
        ClaimFeesPlan {
            authority: authority.to_string(),
            global_vault: global_vault.to_string(),
            destination: destination.to_string(),
            amount,
        },
        vec![authority],
        vec![ix],
        ctx,
    )
}

fn resolve_trader_account(
    authority: Pubkey,
    explicit_trader_account: Option<&str>,
    pda_index: u8,
    subaccount_index: u8,
) -> Result<Pubkey, Box<dyn Error>> {
    match explicit_trader_account {
        Some(trader_account) => parse_pubkey(trader_account),
        None => Ok(TraderKey::derive_pda(
            &authority,
            pda_index,
            subaccount_index,
        )),
    }
}

fn resolve_base_units(
    amount: Option<u64>,
    usdc_amount: Option<f64>,
    action: &str,
) -> Result<u64, Box<dyn Error>> {
    match (amount, usdc_amount) {
        (Some(amount), None) if amount > 0 => Ok(amount),
        (Some(_), None) => Err(format!("--amount for {action} must be greater than zero").into()),
        (None, Some(usdc_amount)) if usdc_amount.is_finite() && usdc_amount > 0.0 => {
            let base_units = (usdc_amount * USDC_BASE_UNITS_PER_TOKEN).round();
            if base_units > u64::MAX as f64 {
                return Err(format!("--usdc-amount for {action} is too large").into());
            }
            Ok(base_units as u64)
        }
        (None, Some(_)) => {
            Err(format!("--usdc-amount for {action} must be a positive finite number").into())
        }
        (None, None) => {
            Err(format!("one of --amount or --usdc-amount is required to {action}").into())
        }
        (Some(_), Some(_)) => unreachable!("clap enforces amount/usdc_amount conflicts"),
    }
}

fn print_flight_bundle<T: Serialize>(
    command: &'static str,
    request: T,
    required_signers: Vec<Pubkey>,
    instructions: Vec<Instruction>,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    print_json(
        &FlightInstructionBundle {
            command,
            request,
            required_signers: required_signers
                .into_iter()
                .map(|signer| signer.to_string())
                .collect(),
            ledger: ctx.ledger.clone(),
            instructions: instructions.iter().map(InstructionOutput::from).collect(),
        },
        ctx.output,
    )
}

impl FlightBuilderState {
    const ACTIVE_FLAG: u64 = 1;
    const LEN: usize = 216;

    fn try_from_account_bytes(data: &[u8]) -> Result<Self, Box<dyn Error>> {
        if data.len() < Self::LEN {
            return Err(format!(
                "flight builder state account is too small: expected at least {} bytes, got {}",
                Self::LEN,
                data.len()
            )
            .into());
        }
        let discriminant = data
            .get(0..8)
            .ok_or("missing Flight builder state discriminant")?;
        if discriminant != FlightAccount::BuilderState.discriminant() {
            return Err("account discriminant is not Flight BuilderState".into());
        }

        Ok(Self {
            authority_key: Pubkey::new_from_array(read_pubkey(data, 8)?),
            trader_key: Pubkey::new_from_array(read_pubkey(data, 40)?),
            status: read_u64(data, 72)?,
            fee_bps: read_u64(data, 80)?,
        })
    }

    fn is_active(self) -> bool {
        self.status & Self::ACTIVE_FLAG != 0
    }
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<[u8; 32], Box<dyn Error>> {
    Ok(data
        .get(offset..offset.saturating_add(32))
        .ok_or("account data is missing pubkey field")?
        .try_into()?)
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, Box<dyn Error>> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset.saturating_add(8))
            .ok_or("account data is missing u64 field")?
            .try_into()?,
    ))
}

fn format_base_units(amount: u64) -> String {
    let whole = amount / 1_000_000;
    let fractional = amount % 1_000_000;
    format!("{whole}.{fractional:06}")
}

fn format_signed_base_units(amount: i64) -> String {
    if amount < 0 {
        format!("-{}", format_base_units(amount.unsigned_abs()))
    } else {
        format_base_units(amount as u64)
    }
}

fn print_builder_view(output: &FlightBuilderView) {
    println!("Flight builder {}", output.authority);
    println!("  Builder state: {}", output.builder_state_account);
    println!(
        "  Active: {} | fee bps: {} | status bits: {}",
        output.active, output.fee_bps, output.status_bits
    );
    println!("  Trader account: {}", output.trader_account);
    println!(
        "  Trader authority: {} | pda index: {} | subaccount: {}",
        output.trader_authority, output.trader_pda_index, output.trader_subaccount_index
    );
    println!(
        "  Collateral: {} quote lots ({} USDC)",
        output.quote_lot_collateral, output.quote_lot_collateral_usdc
    );
    println!(
        "  Withdrawable fee estimate: {} quote lots ({} USDC)",
        output.withdrawable_fee_quote_lots, output.withdrawable_fee_usdc
    );
    println!(
        "  Can withdraw: {} | open positions: {}",
        output.trader_can_withdraw, output.position_count
    );
    println!("  Note: {}", output.note);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_withdraw_amount() {
        let error = resolve_base_units(None, None, "withdraw").unwrap_err();
        assert!(error.to_string().contains("--amount"));
    }

    #[test]
    fn converts_usdc_amount_to_base_units() {
        assert_eq!(
            resolve_base_units(None, Some(12.345678), "withdraw").unwrap(),
            12_345_678
        );
    }

    #[test]
    fn decodes_flight_builder_state() {
        let authority = Pubkey::new_unique();
        let trader = Pubkey::new_unique();
        let mut bytes = vec![0; FlightBuilderState::LEN];
        bytes[..8].copy_from_slice(&FlightAccount::BuilderState.discriminant());
        bytes[8..40].copy_from_slice(authority.as_ref());
        bytes[40..72].copy_from_slice(trader.as_ref());
        bytes[72..80].copy_from_slice(&FlightBuilderState::ACTIVE_FLAG.to_le_bytes());
        bytes[80..88].copy_from_slice(&25u64.to_le_bytes());

        let state = FlightBuilderState::try_from_account_bytes(&bytes).unwrap();

        assert_eq!(state.authority_key, authority);
        assert_eq!(state.trader_key, trader);
        assert_eq!(state.fee_bps, 25);
        assert!(state.is_active());
    }
}
