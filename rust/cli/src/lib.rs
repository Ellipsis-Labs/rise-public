mod app;
mod auth;
mod commands;
mod context;
mod output;
mod util;

use std::error::Error;

use app::{Cli, RootCommand};
use clap::{Command, CommandFactory, Parser};
use context::CommandCtx;

/// Return the clap command tree used by the `phoenix-rise` binary.
pub fn command() -> Command {
    Cli::command()
}

/// Parse process arguments and run the CLI.
pub async fn run_from_args() -> Result<(), Box<dyn Error>> {
    run(Cli::parse()).await
}

async fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let ctx = CommandCtx::try_from_cli(&cli)?;

    match cli.command {
        RootCommand::Api { command } => commands::api::run(command, &ctx).await,
        RootCommand::Auth { command } => auth::run(command, &ctx).await,
        RootCommand::Exchange { command } => commands::exchange::run(command, &ctx).await,
        RootCommand::Market { command } => commands::market::run(command, &ctx).await,
        RootCommand::Rpc { command } => commands::rpc::run(command, &ctx),
        RootCommand::ParseTx(command) => commands::rpc::parse_tx(command, &ctx),
        RootCommand::Tx { command } => commands::tx::run(command, &ctx),
        RootCommand::Trade { command } => commands::trade::run(command, &ctx).await,
        RootCommand::Flight { command } => commands::flight::run(command, &ctx).await,
        RootCommand::Trader { command } => commands::trader::run(command, &ctx).await,
        RootCommand::Export { command } => commands::export::run(command, &ctx).await,
        RootCommand::Ws { command } => commands::ws::run(command, &ctx).await,
        RootCommand::TradeHistory(command) => {
            commands::export::show_trade_history(command, &ctx).await
        }
        RootCommand::OrderHistory(command) => {
            commands::export::show_order_history(command, &ctx).await
        }
        RootCommand::FundingHistory(command) => {
            commands::export::show_funding_history(command, &ctx).await
        }
        RootCommand::LiquidationHistory(command) => {
            commands::export::show_liquidation_history(command, &ctx).await
        }
        RootCommand::Candles(command) => commands::export::show_candles(command, &ctx).await,
    }
}
