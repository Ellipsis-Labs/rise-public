//! Example: Send a market order using PhoenixTxBuilder.
//!
//! This example demonstrates how to use PhoenixTxBuilder with a raw Solana
//! RPC client for order placement.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example send_market_order --features
//! solana-keypair -- SOL [SL_PRICE|x] [TP_PRICE|x]   PHOENIX_DRY_RUN=1 cargo
//! run -p phoenix-rise --example send_market_order --features solana-keypair --
//! SOL 50 100
//!
//! Use "x" in place of a price to skip stop-loss or take-profit.
//! Examples:
//!   send_market_order SOL              # no bracket legs
//!   send_market_order SOL 120.5 150.0  # SL at 120.5, TP at 150.0
//!   send_market_order SOL x 150.0      # no SL, TP at 150.0
//!   send_market_order SOL 120.5 x      # SL at 120.5, no TP

use std::env;
use std::sync::Arc;

use phoenix_rise::api::PhoenixHttpClient;
use phoenix_rise::core::{
    BracketLeg, BracketLegOrders, BracketLegTicket, DEFAULT_BRACKET_LEG_SLIPPAGE_BPS,
    MarketOrderTicket, PhoenixMetadata, PhoenixTxBuilder, TraderKey,
};
use phoenix_rise::ix::types::Side;
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line args
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: send_market_order <SYMBOL> [SL_PRICE|x] [TP_PRICE|x]");
        eprintln!("Example: send_market_order SOL 120.5 150.0");
        std::process::exit(1);
    }
    let symbol = &args[1];

    fn parse_price(arg: &str) -> Option<f64> {
        if arg.eq_ignore_ascii_case("x") {
            return None;
        }
        Some(arg.parse::<f64>().expect("invalid price"))
    }

    let sl_price = args.get(2).and_then(|s| parse_price(s));
    let tp_price = args.get(3).and_then(|s| parse_price(s));
    let bracket = if sl_price.is_some() || tp_price.is_some() {
        Some(BracketLegOrders {
            stop_loss: sl_price.map(|price| {
                BracketLeg::new(price).with_slippage_bps(DEFAULT_BRACKET_LEG_SLIPPAGE_BPS)
            }),
            take_profit: tp_price.map(|price| BracketLeg::new(price).with_limit_order()),
        })
    } else {
        None
    };

    // Load keypair
    let keypair_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME environment variable not set");
        format!("{}/.config/solana/id.json", home)
    });

    println!("Loading keypair from: {}", keypair_path);
    let keypair =
        read_keypair_file(&keypair_path).map_err(|e| format!("Failed to read keypair: {}", e))?;
    let trader = TraderKey::new(keypair.pubkey());

    println!("Trader authority: {}", trader.authority());
    println!("Trader PDA: {}", trader.pda());

    // Fetch exchange metadata via HTTP
    println!("\nFetching exchange metadata...");
    let http = PhoenixHttpClient::new_from_env()?;
    let exchange = http.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);

    // Show cached market info
    if let Some(market) = metadata.get_market(symbol) {
        println!("\n=== {} Market ===", market.symbol);
        println!("  Market Key: {}", market.market_pubkey);
        println!("  Spline Collection: {}", market.spline_pubkey);
        println!("  Taker Fee: {:.4}%", market.taker_fee * 100.0);
        println!("  Maker Fee: {:.4}%", market.maker_fee * 100.0);
    }

    let rpc = Arc::new(RpcClient::new_with_commitment(
        RPC_ENDPOINT.to_string(),
        CommitmentConfig::confirmed(),
    ));

    // Build market order instructions
    let builder = PhoenixTxBuilder::new(&metadata);
    let num_base_lots = 67;
    println!(
        "\nPlacing market order: {} {} base lots",
        symbol, num_base_lots
    );
    if let Some(ref b) = bracket {
        if let Some(sl) = &b.stop_loss {
            println!(
                "  Stop-loss: {} (IOC, {} bps slippage)",
                sl.price, DEFAULT_BRACKET_LEG_SLIPPAGE_BPS
            );
        }
        if let Some(tp) = &b.take_profit {
            println!("  Take-profit: {} (limit at trigger price)", tp.price);
        }
    }

    let mut ticket = MarketOrderTicket::builder()
        .authority(trader.authority())
        .trader_account(trader.pda())
        .symbol(symbol)
        .side(Side::Bid)
        .num_base_lots(num_base_lots);
    if let Some(bracket) = bracket.clone() {
        ticket = ticket.bracket_leg_ticket(BracketLegTicket::new(rpc.clone(), bracket));
    }
    let ticket = ticket.build()?;

    let instructions = builder.place_market_order(ticket).await?;

    // Send transaction via raw Solana RPC client
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

    println!("Transaction confirmed!");
    println!("Signature: {}", signature);
    println!("Explorer: https://explorer.solana.com/tx/{}", signature);

    Ok(())
}
