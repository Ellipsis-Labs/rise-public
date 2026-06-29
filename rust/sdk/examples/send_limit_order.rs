//! Example: Send a limit order using PhoenixTxBuilder.
//!
//! This example demonstrates how to use PhoenixTxBuilder with a raw Solana
//! RPC client for order placement.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example send_limit_order --features
//! solana-keypair -- SOL 150.50 50000

use std::env;

use phoenix_rise::api::PhoenixHttpClient;
use phoenix_rise::core::{LimitOrderTicket, PhoenixMetadata, PhoenixTxBuilder, TraderKey};
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
    if args.len() < 4 {
        eprintln!("Usage: send_limit_order <SYMBOL> <PRICE_USD> <NUM_BASE_LOTS>");
        eprintln!("Example: send_limit_order SOL 150.50 50000");
        std::process::exit(1);
    }
    let symbol = &args[1];
    let price: f64 = args[2].parse()?;
    let num_base_lots: u64 = args[3].parse()?;

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

    // Build limit order instructions
    let builder = PhoenixTxBuilder::new(&metadata);
    println!(
        "\nPlacing limit order: {} {} base lots @ ${:.2}",
        symbol, num_base_lots, price
    );

    let ticket = LimitOrderTicket::builder()
        .authority(trader.authority())
        .trader_account(trader.pda())
        .symbol(symbol)
        .side(Side::Bid)
        .price(price)
        .num_base_lots(num_base_lots)
        .build()?;

    let instructions = builder.place_limit_order(ticket).await?;

    // Send transaction via raw Solana RPC client
    let rpc =
        RpcClient::new_with_commitment(RPC_ENDPOINT.to_string(), CommitmentConfig::confirmed());
    let blockhash = rpc.get_latest_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &[&keypair],
        blockhash,
    );

    let signature = rpc.send_and_confirm_transaction(&tx).await?;

    println!("Transaction confirmed!");
    println!("Signature: {}", signature);
    println!("Explorer: https://explorer.solana.com/tx/{}", signature);

    Ok(())
}
