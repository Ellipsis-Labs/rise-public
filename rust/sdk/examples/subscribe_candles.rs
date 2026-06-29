//! Example: Subscribe to candle updates via WebSocket.
//!
//! Run with:
//!   export PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws
//!   cargo run -p phoenix-rise --example subscribe_candles -- SOL 1m
//!
//! Arguments:
//!   <symbol>     Market symbol (e.g., "SOL")
//!   <timeframe>  Candle timeframe (1s, 5s, 1m, 5m, 15m, 30m, 1h, 4h, 1d)

use std::env;

use phoenix_rise::api::PhoenixWSClient;
use phoenix_rise::types::candles::Timeframe;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter("phoenix_rise=debug,info")
        .init();

    // Get symbol and timeframe from command line
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <symbol> <timeframe>", args[0]);
        eprintln!("Example: {} SOL 1m", args[0]);
        eprintln!("Timeframes: 1s, 5s, 1m, 5m, 15m, 30m, 1h, 4h, 1d");
        std::process::exit(1);
    }

    let symbol = args[1].clone();
    let timeframe: Timeframe = args[2]
        .parse()
        .map_err(|e| format!("Invalid timeframe: {}", e))?;

    println!("Connecting to Phoenix WebSocket...");

    // Connect to the WebSocket server (uses PHOENIX_WS_URL if set)
    let client = PhoenixWSClient::new_from_env()?;

    println!("Subscribing to {} candles for {}", timeframe, symbol);

    // Subscribe to candle updates
    let (mut rx, _handle) = client.subscribe_to_candles(symbol.clone(), timeframe)?;
    println!("Subscribed! Waiting for updates...\n");

    // Process updates
    while let Some(msg) = rx.recv().await {
        println!("=== {} {} Candle ===", msg.symbol, msg.timeframe);
        println!("  Time:        {}", msg.candle.time);
        println!("  Open:        {:.4}", msg.candle.open);
        println!("  High:        {:.4}", msg.candle.high);
        println!("  Low:         {:.4}", msg.candle.low);
        println!("  Close:       {:.4}", msg.candle.close);
        if let Some(volume) = msg.candle.volume {
            println!("  Volume:      {:.4}", volume);
        }
        if let Some(trade_count) = msg.candle.trade_count {
            println!("  Trade Count: {}", trade_count);
        }
        println!();
    }

    println!("WebSocket connection closed");
    Ok(())
}
