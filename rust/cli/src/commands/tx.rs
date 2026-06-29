use std::error::Error;

use clap::Subcommand;

use crate::commands::rpc::ParseTxArgs;
use crate::context::CommandCtx;

#[derive(Debug, Subcommand)]
pub enum TxCommand {
    /// Fetch a transaction and parse Phoenix instructions and events.
    Parse(ParseTxArgs),
}

pub fn run(cmd: TxCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        TxCommand::Parse(args) => crate::commands::rpc::parse_tx(args, ctx),
    }
}
