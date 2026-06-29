use std::error::Error;
use std::str::FromStr;

use clap::Subcommand;
use phoenix_rise::api::BuildRegisterIxsRequest;
use phoenix_rise::types::prelude::{
    CandlesQueryParams, CollateralHistoryQueryParams, FundingHistoryQueryParams,
    FundingHourlyQuery, FundingRateHistoryQuery, OrderHistoryQueryParams, PnlQueryParams,
    PnlResolution, Timeframe, TradeHistoryQueryParams, UserLiquidationHistoryQueryParams,
};

use crate::auth::AuthCommand;
use crate::context::CommandCtx;
use crate::output::print_json;
use crate::util::parse_pubkey;

#[derive(Debug, Subcommand)]
pub enum ApiCommand {
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    ExchangeKeys,
    Exchange,
    ExchangeSnapshot,
    ExchangeStatus,
    Markets,
    Market {
        #[arg(long)]
        symbol: String,
    },
    Orderbook {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value_t = false)]
        include_splines: bool,
    },
    MarkPrice {
        #[arg(long)]
        symbol: String,
    },
    MarketCalendar {
        #[arg(long)]
        symbol: String,
    },
    CommodityMarketCalendar,
    NextMarketCalendarTransition {
        #[arg(long)]
        symbol: String,
    },
    NextCommodityMarketTransition,
    Trader {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: u8,
        #[arg(long, default_value_t = 0)]
        subaccount_index: u8,
    },
    Pnl {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value = "1d")]
        resolution: String,
        #[arg(long)]
        start_time: Option<i64>,
        #[arg(long)]
        end_time: Option<i64>,
        #[arg(long)]
        limit: Option<i64>,
    },
    CollateralHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: u8,
        #[arg(long, default_value_t = 10)]
        limit: i64,
        #[arg(long)]
        next_cursor: Option<String>,
        #[arg(long)]
        prev_cursor: Option<String>,
        #[arg(long)]
        cursor: Option<String>,
    },
    FundingHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: u8,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        start_time: Option<i64>,
        #[arg(long)]
        end_time: Option<i64>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        cursor: Option<String>,
    },
    FundingHourly {
        #[arg(long)]
        authority: String,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        pda_index: Option<i32>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        cursor: Option<String>,
    },
    FundingRates {
        #[arg(long)]
        symbol: String,
        #[arg(long)]
        start_time: Option<i64>,
        #[arg(long)]
        end_time: Option<i64>,
        #[arg(long)]
        limit: Option<i64>,
    },
    OrderHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 10)]
        limit: i64,
        #[arg(long)]
        trader_pda_index: Option<u8>,
        #[arg(long)]
        market_symbol: Option<String>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        privy_id: Option<String>,
    },
    Candles {
        #[arg(long)]
        symbol: String,
        #[arg(long, default_value = "1m")]
        timeframe: String,
        #[arg(long)]
        start_time: Option<i64>,
        #[arg(long)]
        end_time: Option<i64>,
        #[arg(long)]
        limit: Option<u32>,
    },
    TradeHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: u8,
        #[arg(long)]
        market_symbol: Option<String>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        cursor: Option<String>,
    },
    LiquidationHistory {
        #[arg(long)]
        authority: String,
        #[arg(long, default_value_t = 0)]
        pda_index: i32,
        #[arg(long)]
        subaccount_index: Option<i32>,
        #[arg(long)]
        symbol: Option<String>,
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        cursor: Option<String>,
    },
    ReferralActivationPermission,
    ActivateInvite {
        #[arg(long)]
        authority: String,
        #[arg(long)]
        code: String,
    },
    ActivateReferral {
        #[arg(long)]
        authority: String,
        #[arg(long)]
        referral_code: String,
    },
    BuildRegisterIxs {
        #[arg(long)]
        trader_authority: String,
        #[arg(long)]
        tx_fee_payer: String,
        #[arg(long)]
        max_positions: Option<u32>,
    },
}

pub async fn run(cmd: ApiCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        ApiCommand::Auth { command } => crate::auth::run(command, ctx).await,
        command => run_request(command, ctx).await,
    }
}

async fn run_request(cmd: ApiCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    let client = ctx.http_client()?;

    match cmd {
        ApiCommand::Auth { .. } => unreachable!("auth subcommands are handled separately"),
        ApiCommand::ExchangeKeys => print_json(&client.get_exchange_keys().await?, ctx.output)?,
        ApiCommand::Exchange => print_json(&client.get_exchange().await?, ctx.output)?,
        ApiCommand::ExchangeSnapshot => {
            print_json(&client.get_exchange_snapshot().await?, ctx.output)?;
        }
        ApiCommand::ExchangeStatus => {
            print_json(&client.exchange().get_status().await?, ctx.output)?;
        }
        ApiCommand::Markets => print_json(&client.get_markets().await?, ctx.output)?,
        ApiCommand::Market { symbol } => {
            print_json(
                &client.get_market(&symbol.to_ascii_uppercase()).await?,
                ctx.output,
            )?;
        }
        ApiCommand::Orderbook {
            symbol,
            include_splines,
        } => {
            print_json(
                &client
                    .markets()
                    .get_orderbook(&symbol.to_ascii_uppercase(), include_splines)
                    .await?,
                ctx.output,
            )?;
        }
        ApiCommand::MarkPrice { symbol } => {
            print_json(
                &client
                    .markets()
                    .get_mark_price(&symbol.to_ascii_uppercase())
                    .await?,
                ctx.output,
            )?;
        }
        ApiCommand::MarketCalendar { symbol } => {
            print_json(&client.get_market_calendar(&symbol).await?, ctx.output)?;
        }
        ApiCommand::CommodityMarketCalendar => {
            print_json(&client.get_commodity_market_calendar().await?, ctx.output)?;
        }
        ApiCommand::NextMarketCalendarTransition { symbol } => {
            print_json(
                &client.get_next_market_calendar_transition(&symbol).await?,
                ctx.output,
            )?;
        }
        ApiCommand::NextCommodityMarketTransition => {
            print_json(
                &client.get_next_commodity_market_transition().await?,
                ctx.output,
            )?;
        }
        ApiCommand::Trader {
            authority,
            pda_index,
            subaccount_index,
        } => {
            let authority = parse_pubkey(&authority)?;
            let response = client
                .traders()
                .get_trader_subaccount(&authority, pda_index, subaccount_index)
                .await?;
            print_json(&response, ctx.output)?;
        }
        ApiCommand::Pnl {
            authority,
            resolution,
            start_time,
            end_time,
            limit,
        } => {
            let authority = parse_pubkey(&authority)?;
            let resolution = PnlResolution::from_str(&resolution)
                .map_err(|e| format!("invalid PnL resolution '{resolution}': {e}"))?;
            let mut params = PnlQueryParams::new(resolution);
            if let Some(value) = start_time {
                params = params.with_start_time(value);
            }
            if let Some(value) = end_time {
                params = params.with_end_time(value);
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            print_json(&client.get_user_pnl(&authority, params).await?, ctx.output)?;
        }
        ApiCommand::CollateralHistory {
            authority,
            pda_index,
            limit,
            next_cursor,
            prev_cursor,
            cursor,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = CollateralHistoryQueryParams::new(limit).with_pda_index(pda_index);
            if let Some(value) = next_cursor {
                params = params.with_next_cursor(value);
            }
            if let Some(value) = prev_cursor {
                params = params.with_prev_cursor(value);
            }
            if let Some(value) = cursor {
                params.request.cursor = Some(value);
            }
            print_json(
                &client.get_collateral_history(&authority, params).await?,
                ctx.output,
            )?;
        }
        ApiCommand::FundingHistory {
            authority,
            pda_index,
            symbol,
            start_time,
            end_time,
            limit,
            cursor,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = FundingHistoryQueryParams::new().with_pda_index(pda_index);
            if let Some(value) = symbol {
                params = params.with_symbol(value.to_ascii_uppercase());
            }
            if let Some(value) = start_time {
                params = params.with_start_time(value);
            }
            if let Some(value) = end_time {
                params = params.with_end_time(value);
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            if let Some(value) = cursor {
                params = params.with_cursor(value);
            }
            print_json(
                &client.get_funding_history(&authority, params).await?,
                ctx.output,
            )?;
        }
        ApiCommand::FundingHourly {
            authority,
            symbol,
            pda_index,
            limit,
            cursor,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = FundingHourlyQuery::new();
            if let Some(value) = symbol {
                params = params.with_symbol(value.to_ascii_uppercase());
            }
            if let Some(value) = pda_index {
                params = params.with_trader_pda_index(value);
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            if let Some(value) = cursor {
                params = params.with_cursor(value);
            }
            print_json(
                &client
                    .get_user_hourly_funding_history(&authority, params)
                    .await?,
                ctx.output,
            )?;
        }
        ApiCommand::FundingRates {
            symbol,
            start_time,
            end_time,
            limit,
        } => {
            let mut params = FundingRateHistoryQuery::new();
            if let Some(value) = start_time {
                params = params.with_start_time(value);
            }
            if let Some(value) = end_time {
                params = params.with_end_time(value);
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            print_json(
                &client
                    .get_market_funding_rate_history(&symbol.to_ascii_uppercase(), params)
                    .await?,
                ctx.output,
            )?;
        }
        ApiCommand::OrderHistory {
            authority,
            limit,
            trader_pda_index,
            market_symbol,
            cursor,
            privy_id,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = OrderHistoryQueryParams::new(limit);
            if let Some(value) = trader_pda_index {
                params = params.with_pda_index(value);
            }
            if let Some(value) = market_symbol {
                params = params.with_market_symbol(value.to_ascii_uppercase());
            }
            if let Some(value) = cursor {
                params = params.with_cursor(value);
            }
            if let Some(value) = privy_id {
                params = params.with_privy_id(value);
            }
            print_json(
                &client.get_order_history(&authority, params).await?,
                ctx.output,
            )?;
        }
        ApiCommand::Candles {
            symbol,
            timeframe,
            start_time,
            end_time,
            limit,
        } => {
            let timeframe = Timeframe::from_str(&timeframe)
                .map_err(|e| format!("invalid timeframe '{timeframe}': {e}"))?;
            let mut params = CandlesQueryParams::new(symbol.to_ascii_uppercase(), timeframe);
            if let Some(value) = start_time {
                params = params.with_start_time(value);
            }
            if let Some(value) = end_time {
                params = params.with_end_time(value);
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            print_json(&client.get_candles(params).await?, ctx.output)?;
        }
        ApiCommand::TradeHistory {
            authority,
            pda_index,
            market_symbol,
            limit,
            cursor,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = TradeHistoryQueryParams::new().with_pda_index(pda_index);
            if let Some(value) = market_symbol {
                params = params.with_market_symbol(value.to_ascii_uppercase());
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            if let Some(value) = cursor {
                params = params.with_cursor(value);
            }
            print_json(
                &client.get_trade_history(&authority, params).await?,
                ctx.output,
            )?;
        }
        ApiCommand::LiquidationHistory {
            authority,
            pda_index,
            subaccount_index,
            symbol,
            limit,
            cursor,
        } => {
            let authority = parse_pubkey(&authority)?;
            let mut params = UserLiquidationHistoryQueryParams::new().with_pda_index(pda_index);
            if let Some(value) = subaccount_index {
                params = params.with_subaccount_index(value);
            }
            if let Some(value) = symbol {
                params = params.with_symbol(value.to_ascii_uppercase());
            }
            if let Some(value) = limit {
                params = params.with_limit(value);
            }
            if let Some(value) = cursor {
                params = params.with_cursor(value);
            }
            print_json(
                &client
                    .get_user_liquidation_history(&authority, params)
                    .await?,
                ctx.output,
            )?;
        }
        ApiCommand::ReferralActivationPermission => {
            print_json(
                &client.invite().get_referral_activation_permission().await?,
                ctx.output,
            )?;
        }
        ApiCommand::ActivateInvite { authority, code } => {
            let authority = parse_pubkey(&authority)?;
            print_json(
                &client.invite().activate_invite(&authority, &code).await?,
                ctx.output,
            )?;
        }
        ApiCommand::ActivateReferral {
            authority,
            referral_code,
        } => {
            let authority = parse_pubkey(&authority)?;
            print_json(
                &client
                    .invite()
                    .activate_referral(&authority, &referral_code)
                    .await?,
                ctx.output,
            )?;
        }
        ApiCommand::BuildRegisterIxs {
            trader_authority,
            tx_fee_payer,
            max_positions,
        } => {
            let request = BuildRegisterIxsRequest {
                trader_authority,
                tx_fee_payer,
                max_positions,
            };
            print_json(
                &client.exchange().build_register_ixs(&request).await?,
                ctx.output,
            )?;
        }
    }

    Ok(())
}
