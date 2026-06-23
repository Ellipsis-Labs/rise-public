//! Example: Subscribe to trader state updates via WebSocket.
//!
//! Run with:
//!   export PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws
//!   export KEYPAIR_PATH=~/.config/solana/id.json
//!   cargo run -p phoenix-rise --example subscribe_trader_state --features
//! solana-keypair

use phoenix_rise::{PhoenixWSClient, Trader, TraderKey};
use solana_keypair::read_keypair_file;
use solana_signer::Signer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter("phoenix_rise=debug,info")
        .init();

    println!("Connecting to Phoenix WebSocket...");

    // Connect to the WebSocket server (uses PHOENIX_WS_URL if set)
    let client = PhoenixWSClient::new_from_env()?;

    // Load keypair from file specified by KEYPAIR_PATH env var
    let keypair_path =
        std::env::var("KEYPAIR_PATH").map_err(|_| "KEYPAIR_PATH environment variable not set")?;
    let keypair = read_keypair_file(&keypair_path)
        .map_err(|e| format!("Failed to read keypair from {}: {}", keypair_path, e))?;

    // Create trader key from keypair
    let key = TraderKey::new(keypair.pubkey());
    let authority = key.authority();
    println!("Subscribing to trader state for authority: {}", authority);
    println!("Trader PDA: {}", key.pda());

    // Create a trader state container
    let mut trader = Trader::new(key.clone());

    // Subscribe to trader state updates
    let (mut rx, _handle) = client.subscribe_to_trader_state(&authority)?;
    println!("Subscribed! Waiting for updates...\n");

    // Process updates
    while let Some(msg) = rx.recv().await {
        println!("=== Received update at slot {} ===", msg.slot);

        // Apply the update to our local state
        trader.apply_update(&msg);

        // Print summary
        println!("Total Collateral: {}", trader.total_collateral());

        let positions = trader.all_positions();
        if positions.is_empty() {
            println!("Positions: (none)");
        } else {
            println!("Positions:");
            for pos in positions {
                println!(
                    "  {} | Size: {} lots | Entry: {}",
                    pos.symbol, pos.base_position_lots, pos.entry_price_usd
                );
            }
        }

        let orders = trader.all_orders();
        if orders.is_empty() {
            println!("Orders: (none)");
        } else {
            println!("Orders:");
            for order in orders {
                println!(
                    "  {} | {} {} @ {} | Remaining: {} lots",
                    order.symbol,
                    order.side,
                    order.order_type,
                    order.price_usd,
                    order.size_remaining_lots
                );
            }
        }

        println!();
    }

    println!("WebSocket connection closed");
    Ok(())
}
