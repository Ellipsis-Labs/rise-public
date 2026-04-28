//! Example: Subscribe to orderbook updates via WebSocket.
//!
//! Run with:
//!   export PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws
//!   cargo run -p phoenix-rise --example subscribe_l2_book -- SOL
//!   cargo run -p phoenix-rise --example subscribe_l2_book -- SOL
//! --bypass-execution-band
//!
//! The symbol argument is required (e.g., "SOL", "BTC", "ETH").

use std::env;

use phoenix_rise::{L2Book, PhoenixWSClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter("phoenix_rise=debug,info")
        .init();

    // Get symbol from command line (required), with optional bypass flag.
    let args: Vec<String> = env::args().skip(1).collect();
    let symbol = args
        .first()
        .cloned()
        .expect("Usage: subscribe_l2_book <SYMBOL> [--bypass-execution-band]");
    let bypass_execution_band = args.iter().any(|arg| arg == "--bypass-execution-band");

    println!("Connecting to Phoenix WebSocket...");

    // Connect to the WebSocket server (uses PHOENIX_WS_URL env var)
    let client = PhoenixWSClient::new_from_env()?;

    println!(
        "Subscribing to orderbook for: {} (bypass_execution_band={})",
        symbol, bypass_execution_band
    );

    // Subscribe to orderbook updates
    let (mut rx, _handle) =
        client.subscribe_to_orderbook_with_options(symbol.clone(), bypass_execution_band)?;
    println!("Subscribed! Waiting for updates...\n");

    // Maintain an L2Book container
    let mut book = L2Book::new(symbol);

    // Process updates
    while let Some(msg) = rx.recv().await {
        book.apply_update(&msg);

        println!("=== {} Orderbook Update ===", book.symbol());

        if let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask()) {
            println!(
                "  Best Bid:   ${:.4} ({:.2})",
                bid,
                book.best_bid_quantity().unwrap_or(0.0)
            );
            println!(
                "  Best Ask:   ${:.4} ({:.2})",
                ask,
                book.best_ask_quantity().unwrap_or(0.0)
            );
        }

        if let Some(spread) = book.spread() {
            println!("  Spread:     ${:.6}", spread);
        }

        if let Some(mid) = book.mid_price() {
            println!("  Mid Price:  ${:.4}", mid);
        }

        if let Some(spread_pct) = book.spread_percent() {
            println!("  Spread %:   {:.4}%", spread_pct);
        }

        println!(
            "  Bid Depth:  {} levels ({:.2} total qty)",
            book.bid_depth(),
            book.total_bid_liquidity()
        );
        println!(
            "  Ask Depth:  {} levels ({:.2} total qty)",
            book.ask_depth(),
            book.total_ask_liquidity()
        );

        // Show top 3 levels
        let bids = book.bids();
        let asks = book.asks();

        if !bids.is_empty() {
            println!("\n  Top Bids:");
            for level in bids.iter().take(3) {
                println!("    ${:.4} x {:.2}", level.price, level.quantity);
            }
        }

        if !asks.is_empty() {
            println!("\n  Top Asks:");
            for level in asks.iter().take(3) {
                println!("    ${:.4} x {:.2}", level.price, level.quantity);
            }
        }

        println!();
    }

    println!("WebSocket connection closed");
    Ok(())
}
