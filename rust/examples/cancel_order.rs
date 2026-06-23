//! Example: Cancel an order using PhoenixTxBuilder.
//!
//! This example demonstrates how to use PhoenixTxBuilder with a raw Solana
//! RPC client for order cancellation.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example cancel_order --features solana-keypair
//! -- SOL 50000 12345

use std::env;

use phoenix_rise::{CancelId, PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, TraderKey};
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
        eprintln!("Usage: cancel_order <SYMBOL> <PRICE_IN_TICKS> <ORDER_SEQUENCE_NUMBER>");
        eprintln!("Example: cancel_order SOL 50000 12345");
        std::process::exit(1);
    }
    let symbol = &args[1];
    let price_in_ticks: u64 = args[2].parse()?;
    let order_sequence_number: u64 = args[3].parse()?;

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
    }

    // Build cancel order instructions
    let builder = PhoenixTxBuilder::new(&metadata);
    let cancel_id = CancelId::new(price_in_ticks, order_sequence_number);
    println!(
        "\nCancelling order: {} price_in_ticks={} seq={}",
        symbol, price_in_ticks, order_sequence_number
    );

    let instructions =
        builder.build_cancel_orders(trader.authority(), trader.pda(), symbol, vec![cancel_id])?;

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
