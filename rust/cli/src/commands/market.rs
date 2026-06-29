use std::error::Error;

use clap::{Args, Subcommand};

use crate::commands::api::ApiCommand;
use crate::commands::export::CandlesExportArgs;
use crate::context::CommandCtx;

#[derive(Debug, Subcommand)]
pub enum MarketCommand {
    /// List all Phoenix Rise markets.
    List,
    /// Fetch metadata for one market symbol.
    Show {
        /// Market symbol, for example SOL.
        #[arg(long)]
        symbol: String,
    },
    /// Fetch L2 orderbook data for one market.
    Orderbook {
        /// Market symbol, for example SOL.
        #[arg(long)]
        symbol: String,
        /// Include spline liquidity in the response.
        #[arg(long, default_value_t = false)]
        include_splines: bool,
    },
    /// Fetch the current mark price for one market.
    MarkPrice {
        /// Market symbol, for example SOL.
        #[arg(long)]
        symbol: String,
    },
    /// Fetch one market's regular market calendar.
    Calendar {
        /// Market symbol, for example SOL.
        #[arg(long)]
        symbol: String,
    },
    /// Fetch the commodity market calendar.
    CommodityCalendar,
    /// Fetch the next market-calendar transition for one symbol.
    NextTransition {
        /// Market symbol, for example SOL.
        #[arg(long)]
        symbol: String,
    },
    /// Fetch the next commodity market transition.
    NextCommodityTransition,
    /// Fetch funding-rate history for one market.
    FundingRates(FundingRatesArgs),
    /// Fetch paginated market candles and print a table or JSON.
    Candles(CandlesExportArgs),
}

#[derive(Debug, Args)]
pub struct FundingRatesArgs {
    /// Market symbol, for example SOL.
    #[arg(long)]
    symbol: String,
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

pub async fn run(cmd: MarketCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        MarketCommand::List => crate::commands::api::run(ApiCommand::Markets, ctx).await,
        MarketCommand::Show { symbol } => {
            crate::commands::api::run(ApiCommand::Market { symbol }, ctx).await
        }
        MarketCommand::Orderbook {
            symbol,
            include_splines,
        } => {
            crate::commands::api::run(
                ApiCommand::Orderbook {
                    symbol,
                    include_splines,
                },
                ctx,
            )
            .await
        }
        MarketCommand::MarkPrice { symbol } => {
            crate::commands::api::run(ApiCommand::MarkPrice { symbol }, ctx).await
        }
        MarketCommand::Calendar { symbol } => {
            crate::commands::api::run(ApiCommand::MarketCalendar { symbol }, ctx).await
        }
        MarketCommand::CommodityCalendar => {
            crate::commands::api::run(ApiCommand::CommodityMarketCalendar, ctx).await
        }
        MarketCommand::NextTransition { symbol } => {
            crate::commands::api::run(ApiCommand::NextMarketCalendarTransition { symbol }, ctx)
                .await
        }
        MarketCommand::NextCommodityTransition => {
            crate::commands::api::run(ApiCommand::NextCommodityMarketTransition, ctx).await
        }
        MarketCommand::FundingRates(args) => {
            crate::commands::api::run(
                ApiCommand::FundingRates {
                    symbol: args.symbol,
                    start_time: args.start_time,
                    end_time: args.end_time,
                    limit: args.limit,
                },
                ctx,
            )
            .await
        }
        MarketCommand::Candles(args) => crate::commands::export::show_candles(args, ctx).await,
    }
}
