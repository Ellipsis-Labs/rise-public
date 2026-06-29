//! Example: Subscribe to trades updates via WebSocket.
//!
//! Run with:
//!   export PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws
//!   cargo run -p phoenix-rise --example subscribe_trades -- SOL

use std::env;

use phoenix_rise::api::PhoenixWSClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter("phoenix_rise=debug,info")
        .init();

    // Get market symbol from command line
    let Some(market_symbol) = env::args().nth(1) else {
        eprintln!("Usage: cargo run -p phoenix-rise --example subscribe_trades -- <SYMBOL>");
        eprintln!("Example: cargo run -p phoenix-rise --example subscribe_trades -- SOL");
        return Ok(());
    };

    println!("Connecting to Phoenix WebSocket...");

    // Connect to the WebSocket server (uses PHOENIX_WS_URL if set)
    let client = PhoenixWSClient::new_from_env()?;

    println!("Subscribing to trades for: {}", market_symbol);

    // Subscribe to trades updates
    let (mut rx, _handle) = client.subscribe_to_trades(market_symbol)?;
    println!("Subscribed! Waiting for trades...\n");

    // Process trades
    while let Some(msg) = rx.recv().await {
        for trade in &msg.trades {
            println!("=== {} Trade ===", msg.symbol);
            println!("  Side:          {:?}", trade.side);
            println!("  Base Amount:   {}", trade.base_amount);
            println!("  Quote Amount:  {}", trade.quote_amount);
            println!("  Taker:         {}", trade.taker);
            println!(
                "  Timestamp:     {}",
                chrono::DateTime::<chrono::Utc>::from(trade.timestamp)
            );
            println!("  Seq Number:    {}", trade.trade_sequence_number);
            println!("  Num Fills:     {}", trade.num_fills);
            println!();
        }
    }

    println!("WebSocket connection closed");
    Ok(())
}
