//! Example: Send a Flight-routed market order using `PhoenixFlightClient`.
//!
//! Builds a Phoenix market-order instruction with `PhoenixTxBuilder`, wraps it
//! with `PhoenixFlightClient::try_wrap_order_instruction` so builder fees are
//! credited to the configured builder trader account, and submits the
//! transaction to RPC.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example send_flight_market_order \
//!     --features solana-keypair -- \
//!     <BUILDER_AUTHORITY> <BUILDER_PDA_INDEX> <BUILDER_SUBACCOUNT_INDEX> \
//!     <SYMBOL> <bid|ask> <NUM_BASE_LOTS>
//!
//! Example:
//!   cargo run -p phoenix-rise --example send_flight_market_order \
//!     --features solana-keypair -- \
//!     Builder1111111111111111111111111111111111 0 0 SOL bid 67
//!
//! Environment:
//!   KEYPAIR_PATH       Trader keypair (default: ~/.config/solana/id.json)
//!   PHOENIX_API_URL    Phoenix API URL (default: https://perp-api.phoenix.trade)
//!   SOLANA_RPC_URL     Solana RPC URL (default: https://api.mainnet-beta.solana.com)
//!   PHOENIX_DRY_RUN=1  Simulate the transaction instead of submitting it.

use std::env;
use std::str::FromStr;

use phoenix_rise::{
    MarketOrderTicket, PhoenixFlightClient, PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder,
    Side, TraderKey,
};
use solana_commitment_config::CommitmentConfig;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_keypair::read_keypair_file;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DEFAULT_RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";
const COMPUTE_UNIT_LIMIT: u32 = 1_400_000;

fn parse_side(value: &str) -> Result<Side, String> {
    match value.to_ascii_lowercase().as_str() {
        "bid" | "buy" => Ok(Side::Bid),
        "ask" | "sell" => Ok(Side::Ask),
        other => Err(format!("invalid side '{other}', expected bid|ask")),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 7 {
        eprintln!(
            "Usage: send_flight_market_order <BUILDER_AUTHORITY> <BUILDER_PDA_INDEX> \
             <BUILDER_SUBACCOUNT_INDEX> <SYMBOL> <bid|ask> <NUM_BASE_LOTS>"
        );
        std::process::exit(1);
    }

    let builder_authority = Pubkey::from_str(&args[1])?;
    let builder_pda_index: u8 = args[2].parse()?;
    let builder_subaccount_index: u8 = args[3].parse()?;
    let symbol = &args[4];
    let side = parse_side(&args[5])?;
    let num_base_lots: u64 = args[6].parse()?;

    let keypair_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME environment variable not set");
        format!("{}/.config/solana/id.json", home)
    });
    println!("Loading trader keypair from: {}", keypair_path);
    let keypair = read_keypair_file(&keypair_path)
        .map_err(|err| format!("Failed to read keypair: {}", err))?;
    let trader = TraderKey::new(keypair.pubkey());

    let flight = PhoenixFlightClient::new(
        builder_authority,
        builder_pda_index,
        builder_subaccount_index,
    );

    println!("Trader authority:       {}", trader.authority());
    println!("Trader account (PDA):   {}", trader.pda());
    println!("Builder authority:      {}", flight.builder_authority);
    println!(
        "Builder trader account: {}",
        flight.builder_trader_account()
    );

    println!("\nFetching exchange metadata...");
    let http = PhoenixHttpClient::new_from_env()?;
    let exchange = http.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);
    if let Some(market) = metadata.get_market(symbol) {
        println!("\n=== {} Market ===", market.symbol);
        println!("  Market Key:      {}", market.market_pubkey);
        println!("  Taker Fee:       {:.4}%", market.taker_fee * 100.0);
    }

    let tx_builder = PhoenixTxBuilder::new(&metadata);
    let ticket = MarketOrderTicket::builder()
        .authority(trader.authority())
        .trader_account(trader.pda())
        .symbol(symbol)
        .side(side)
        .num_base_lots(num_base_lots)
        .build()?;

    println!(
        "\nPlacing Flight-routed market order: {} {:?} {} base lots",
        symbol, side, num_base_lots
    );
    let native_ixs = tx_builder.place_market_order(ticket).await?;
    let mut instructions = vec![ComputeBudgetInstruction::set_compute_unit_limit(
        COMPUTE_UNIT_LIMIT,
    )];
    for ix in native_ixs {
        instructions.push(flight.try_wrap_order_instruction(ix, trader.authority())?);
    }

    let rpc = RpcClient::new_with_commitment(
        env::var("SOLANA_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_ENDPOINT.to_string()),
        CommitmentConfig::confirmed(),
    );
    let blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &[&keypair],
        blockhash,
    );

    if env::var_os("PHOENIX_DRY_RUN").is_some() {
        println!("\nDry run: simulating transaction without sending...");
        let simulation = rpc.simulate_transaction(&tx).await?;
        println!("Simulation result: {:?}", simulation.value.err);
        if let Some(logs) = simulation.value.logs {
            for log in logs {
                println!("{}", log);
            }
        }
        return Ok(());
    }

    let signature = rpc.send_and_confirm_transaction(&tx).await?;
    println!("\nTransaction confirmed!");
    println!("Signature: {}", signature);
    println!("Explorer:  https://explorer.solana.com/tx/{}", signature);

    Ok(())
}
