//! Example: Withdraw and Deposit USDC with Phoenix protocol.
//!
//! This example demonstrates both the withdraw and deposit flows:
//!
//! Withdraw flow (5 instructions):
//! 1. Create ATA for Phoenix tokens (if needed)
//! 2. Approve Ember to spend Phoenix tokens
//! 3. Create ATA for USDC (if needed)
//! 4. Withdraw Phoenix tokens from Phoenix protocol
//! 5. Convert Phoenix tokens to USDC via Ember
//!
//! Deposit flow (3 instructions):
//! 1. Create ATA for Phoenix tokens (if needed)
//! 2. Convert USDC to Phoenix tokens via Ember
//! 3. Deposit Phoenix tokens into Phoenix protocol
//!
//! Run with:
//!   cargo run -p phoenix-rise --example deposit_funds --features
//! solana-keypair -- 100.0

use std::env;

use phoenix_rise::{PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, TraderKey};
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
        eprintln!("Usage: deposit_funds <USDC_AMOUNT>");
        eprintln!("Example: deposit_funds 100.0");
        std::process::exit(1);
    }
    let usdc_amount: f64 = args[1]
        .parse()
        .map_err(|_| "Invalid USDC amount - must be a number")?;

    if usdc_amount <= 0.0 {
        eprintln!("Error: USDC amount must be greater than 0");
        std::process::exit(1);
    }

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

    // Show exchange keys info
    let keys = metadata.keys();
    println!("\n=== Exchange Keys ===");
    println!("  Canonical Mint: {}", keys.canonical_mint);
    println!("  Global Vault: {}", keys.global_vault);

    // Create Solana RPC client
    let rpc =
        RpcClient::new_with_commitment(RPC_ENDPOINT.to_string(), CommitmentConfig::confirmed());
    let builder = PhoenixTxBuilder::new(&metadata);

    // Withdraw funds first
    println!("\nWithdrawing ${:.2} USDC...", usdc_amount);
    println!("  This will:");
    println!("  1. Create Phoenix token ATA (if needed)");
    println!("  2. Approve Ember to spend Phoenix tokens");
    println!("  3. Create USDC ATA (if needed)");
    println!("  4. Withdraw Phoenix tokens from the protocol");
    println!("  5. Convert Phoenix tokens to USDC via Ember");

    let withdraw_instructions =
        builder.build_withdraw_funds(trader.authority(), trader.pda(), usdc_amount)?;

    let blockhash = rpc.get_latest_blockhash().await?;
    let withdraw_tx = Transaction::new_signed_with_payer(
        &withdraw_instructions,
        Some(&trader.authority()),
        &[&keypair],
        blockhash,
    );

    let withdraw_signature = rpc.send_and_confirm_transaction(&withdraw_tx).await?;

    println!("\nWithdraw transaction confirmed!");
    println!("Signature: {}", withdraw_signature);
    println!(
        "Explorer: https://explorer.solana.com/tx/{}",
        withdraw_signature
    );

    // Deposit funds
    println!("\nDepositing ${:.2} USDC...", usdc_amount);
    println!("  This will:");
    println!("  1. Create Phoenix token ATA (if needed)");
    println!("  2. Convert USDC to Phoenix tokens via Ember");
    println!("  3. Deposit Phoenix tokens into the protocol");

    let deposit_instructions =
        builder.build_deposit_funds(trader.authority(), trader.pda(), usdc_amount)?;

    let blockhash = rpc.get_latest_blockhash().await?;
    let deposit_tx = Transaction::new_signed_with_payer(
        &deposit_instructions,
        Some(&keypair.pubkey()),
        &[&keypair],
        blockhash,
    );

    let deposit_signature = rpc.send_and_confirm_transaction(&deposit_tx).await?;

    println!("\nDeposit transaction confirmed!");
    println!("Signature: {}", deposit_signature);
    println!(
        "Explorer: https://explorer.solana.com/tx/{}",
        deposit_signature
    );

    Ok(())
}
