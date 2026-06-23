//! Example: Cancel a stop loss (or take profit) order using PhoenixTxBuilder.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example cancel_stop_loss --features
//! solana-keypair -- SOL less_than

use std::env;

use phoenix_rise::{Direction, PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, TraderKey};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cancel_stop_loss <SYMBOL> <DIRECTION>");
        eprintln!("  DIRECTION: less_than | greater_than");
        eprintln!("Example: cancel_stop_loss SOL less_than");
        std::process::exit(1);
    }
    let symbol = &args[1];
    let direction = match args[2].as_str() {
        "less_than" => Direction::LessThan,
        "greater_than" => Direction::GreaterThan,
        other => {
            eprintln!(
                "Invalid direction '{}': use 'less_than' or 'greater_than'",
                other
            );
            std::process::exit(1);
        }
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

    // Build cancel stop loss instruction
    let builder = PhoenixTxBuilder::new(&metadata);
    println!(
        "\nCancelling stop loss: {} direction={:?}",
        symbol, direction
    );

    let instructions =
        builder.build_cancel_bracket_leg(trader.authority(), trader.pda(), symbol, direction)?;

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
