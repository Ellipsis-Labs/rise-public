//! Example: Using PhoenixClient with receiver-based subscriptions.
//!
//! Demonstrates:
//! - Unified `subscribe(...)` API
//! - Market bundle subscriptions
//! - Trader margin subscriptions with trigger messages
//! - No callbacks or shared-state getters
//!
//! Run with:
//!   export PHOENIX_API_URL=https://perp-api.phoenix.trade
//!   cargo run -p phoenix-rise --example phoenix_client --
//! <AUTHORITY_PUBKEY>

use std::str::FromStr;

use phoenix_rise::math::{TraderPortfolioMargin, WrapperNum};
use phoenix_rise::{
    MarginTrigger, PhoenixClient, PhoenixClientEvent, PhoenixSubscription, Timeframe,
};
use solana_pubkey::Pubkey;

fn print_margin_summary(margin: &TraderPortfolioMargin) {
    let collateral = margin.quote_lot_collateral.as_inner() as f64 / 1_000_000.0;
    let portfolio_value = margin.portfolio_value().as_inner() as f64 / 1_000_000.0;
    let effective_coll = margin.effective_collateral().as_inner() as f64 / 1_000_000.0;
    let initial_margin = margin.margin.initial_margin.as_inner() as f64 / 1_000_000.0;
    let maintenance_margin = margin.margin.maintenance_margin.as_inner() as f64 / 1_000_000.0;
    let unrealized_pnl = margin.margin.unrealized_pnl.as_inner() as f64 / 1_000_000.0;

    println!("  Collateral:           ${:.2}", collateral);
    println!("  Portfolio Value:      ${:.2}", portfolio_value);
    println!("  Effective Collateral: ${:.2}", effective_coll);
    println!("  Initial Margin:       ${:.2}", initial_margin);
    println!("  Maintenance Margin:   ${:.2}", maintenance_margin);
    println!("  Unrealized PnL:       ${:+.2}", unrealized_pnl);
    println!("  Risk Tier:            {:?}", margin.risk_tier().ok());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        // .with_env_filter("phoenix_rise=debug,info")
        .init();

    let authority_str = std::env::args()
        .nth(1)
        .expect("Usage: phoenix_client <AUTHORITY_PUBKEY>");
    let authority = Pubkey::from_str(&authority_str)?;

    println!("Creating PhoenixClient...");
    let client = PhoenixClient::new_from_env().await?;

    let (mut market_rx, _market_handle) = client
        .subscribe(PhoenixSubscription::Market {
            symbol: "SOL".to_string(),
            candle_timeframes: vec![Timeframe::Minute1],
            include_trades: false,
        })
        .await?;

    let (mut margin_rx, _margin_handle) = client
        .subscribe(PhoenixSubscription::trader_margin(authority, 0))
        .await?;

    println!("Running... Press Ctrl+C to stop\n");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            Some(event) = market_rx.recv() => {
                match event {
                    PhoenixClientEvent::MarketUpdate { symbol, update, .. } => {
                        println!("{} mark={:.4} funding={:.8}", symbol, update.mark_price, update.funding_rate);
                    }
                    PhoenixClientEvent::OrderbookUpdate { symbol, update, .. } => {
                        let best_bid = update.orderbook.bids.first().map(|(px, _)| *px);
                        let best_ask = update.orderbook.asks.first().map(|(px, _)| *px);
                        println!("{} book bid={:?} ask={:?}", symbol, best_bid, best_ask);
                    }
                    PhoenixClientEvent::FundingRateUpdate { symbol, update, .. } => {
                        println!(
                            "{} funding={:.8}",
                            symbol,
                            update.funding,
                        );
                    }
                    PhoenixClientEvent::CandleUpdate { symbol, timeframe, update, .. } => {
                        println!(
                            "{} {} candle o={:.4} h={:.4} l={:.4} c={:.4}",
                            symbol,
                            timeframe,
                            update.candle.open,
                            update.candle.high,
                            update.candle.low,
                            update.candle.close,
                        );
                    }
                    _ => {}
                }
            }
            Some(event) = margin_rx.recv() => {
                if let PhoenixClientEvent::MarginUpdate { trigger, margin, .. } = event {
                    match trigger {
                        MarginTrigger::Trader(msg) => {
                            println!("Margin trigger: trader slot={}", msg.slot);
                        }
                        MarginTrigger::Market(msg) => {
                            println!("Margin trigger: market {} mark={:.4}", msg.symbol, msg.mark_price);
                        }
                    }

                    if let Some(margin) = margin {
                        print_margin_summary(&margin);
                    } else {
                        println!("  margin unavailable (waiting for trader/market initialization)");
                    }
                }
            }
            else => {
                break;
            }
        }
    }

    println!("\nShutting down...");
    client.shutdown();
    client.run().await;

    Ok(())
}
