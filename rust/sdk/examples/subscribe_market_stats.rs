//! Example: Subscribe to market updates via WebSocket.
//!
//! This example demonstrates:
//! - Connection status tracking
//! - Multiple subscribers to the same channel
//! - Explicit handle drops for unsubscription
//!
//! Run with:
//!   export PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws
//!   cargo run -p phoenix-rise --example subscribe_market_stats -- SOL

use std::env;

use phoenix_rise::api::{PhoenixWSClient, WsConnectionStatus};
use phoenix_rise::types::market_stats::MarketStats;
use tokio::select;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter("phoenix_rise=debug,info")
        .init();

    // Get symbol from command line (required)
    let symbol = env::args()
        .nth(1)
        .expect("Usage: subscribe_market_stats <SYMBOL> (e.g., SOL, BTC)");

    println!("Connecting to Phoenix WebSocket...");

    // Connect with connection status tracking
    let mut client = PhoenixWSClient::new_from_env_with_connection_status()?;

    // Get connection status receiver
    let mut status_rx = client.connection_status_receiver().unwrap();

    println!("Subscribing to market updates for: {}", symbol);

    // Subscribe twice to demonstrate multiple subscribers
    let (mut rx1, handle1) = client.subscribe_to_market(symbol.clone())?;
    let (mut rx2, handle2) = client.subscribe_to_market(symbol.clone())?;
    println!("Subscribed with 2 receivers! Waiting for updates...\n");

    // Maintain a MarketStats container
    let mut market = MarketStats::new(symbol.clone());

    // Process updates (limit to 5 for demo purposes)
    let mut update_count = 0;
    loop {
        select! {
            Some(status) = status_rx.recv() => {
                match status {
                    WsConnectionStatus::Connecting => println!("[status] Connecting..."),
                    WsConnectionStatus::Connected => println!("[status] Connected!"),
                    WsConnectionStatus::ConnectionFailed => println!("[status] Connection failed"),
                    WsConnectionStatus::Disconnected(reason) => println!("[status] Disconnected: {}", reason),
                }
            }
            Some(msg) = rx1.recv() => {
                println!("[rx1] === {} Stats Update ===", msg.symbol);
                println!("  Mark Price:    ${:.4}", msg.mark_price);
                println!("  Mid Price:     ${:.4}", msg.mid_price);
                println!("  Oracle Price:  ${:.4}", msg.oracle_price);
                println!("  Funding Rate:  {:.6}%", msg.funding_rate * 100.0);

                // Apply the update to our market stats container
                market.apply_update(&msg);
                if let Some(change) = market.price_change_24h_percent() {
                    println!("  24h Change:    {:.2}%", change);
                }
                println!();

                update_count += 1;
                if update_count >= 5 {
                    break;
                }
            }
            Some(msg) = rx2.recv() => {
                println!("[rx2] === {} Stats Update ===", msg.symbol);
                println!("  Mark Price:    ${:.4}", msg.mark_price);
                println!();
            }
        }
    }

    // Explicitly drop handles to unsubscribe
    println!("Dropping handle1...");
    drop(handle1);
    println!("Dropped handle1 (one subscriber remains)");

    println!("Dropping handle2...");
    drop(handle2);
    println!("Dropped handle2 (server unsubscribe sent)\n");

    println!("WebSocket connection closed");
    Ok(())
}
