//! Example: Real-time trader margin monitoring using live WebSocket data
//!
//! This example demonstrates a production-ready margin monitoring system:
//! 1. Bootstrap market configuration via HTTP API
//! 2. Subscribe to market-stats WebSocket for real-time prices
//! 3. Subscribe to trader-state WebSocket for position updates
//! 4. Recompute margin instantly (<1ms) on every update
//! 5. Monitor risk tier changes in real-time
//!
//! # Usage
//!
//! ```bash
//! PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws \
//! cargo run -p phoenix-rise --example compute_trader_margin -- <AUTHORITY_PUBKEY>
//! ```
//!
//! # What this demonstrates
//!
//! - **Real-time margin monitoring**: Margin recalculated on every
//!   price/position update
//! - **Zero-latency calculations**: <1ms margin computation vs 50-200ms API
//!   latency
//! - **Risk monitoring**: Instant alerts when crossing risk tier thresholds
//! - **Production-ready**: Handles errors and state synchronization

use std::env;

use phoenix_rise::api::{
    PhoenixHttpClient, PhoenixMetadata, PhoenixWSClient, SubaccountState, Trader, TraderKey,
};
use phoenix_rise::math::{TraderPortfolio, TraderPortfolioMargin};
use phoenix_rise::types::exchange::ExchangeView;
use solana_pubkey::Pubkey;
use tokio::select;

/// Print detailed margin information for the portfolio
fn print_margin_summary(margin: &TraderPortfolioMargin, subaccount: &SubaccountState) {
    println!("\n========== MARGIN SUMMARY ==========");

    println!("\nTrader State:");
    println!("  Positions: {}", subaccount.positions.len());
    println!("  Orders:    {}", subaccount.orders.len());

    // Aggregate portfolio margin
    let portfolio_value = margin.portfolio_value().as_inner() as f64 / 1_000_000_000_000.0;
    let effective_coll = margin.effective_collateral().as_inner() as f64 / 1_000_000_000_000.0;
    let initial_margin = margin.margin.initial_margin.as_inner() as f64 / 1_000_000.0;
    let maintenance_margin = margin.margin.maintenance_margin.as_inner() as f64 / 1_000_000.0;
    let unrealized_pnl = margin.margin.unrealized_pnl.as_inner() as f64 / 1_000_000.0;

    println!("\nAggregate Portfolio:");
    println!(
        "  Collateral:          ${:.2}",
        margin.quote_lot_collateral.as_inner() as f64 / 1_000_000_000_000.0
    );
    println!("  Portfolio Value:     ${:.2}", portfolio_value);
    println!("  Effective Collateral:${:.2}", effective_coll);
    println!("  Initial Margin:      ${:.2}", initial_margin);
    println!(
        "  Maintenance Margin:  ${:.2} (liquidation threshold)",
        maintenance_margin
    );
    println!("  Unrealized PnL:      ${:+.2}", unrealized_pnl);
    println!("  Risk Tier:           {:?}", margin.risk_tier().ok());

    // SOL market margin (if position exists in computed margin)
    if let Some(sol_margin) = margin.positions.get("SOL") {
        let sol_initial = sol_margin.margin.initial_margin.as_inner() as f64 / 1_000_000.0;
        let sol_maint = sol_margin.margin.maintenance_margin.as_inner() as f64 / 1_000_000.0;
        let sol_pnl = sol_margin.margin.unrealized_pnl.as_inner() as f64 / 1_000_000.0;
        let sol_limit_order = sol_margin.margin.limit_order_margin.as_inner() as f64 / 1_000_000.0;

        println!("\nSOL Market:");
        if let Some(pos) = sol_margin.position {
            println!(
                "  Position:            {} lots",
                pos.base_lot_position.as_inner()
            );
        }
        println!("  Initial Margin:      ${:.2}", sol_initial);
        println!("  Maintenance Margin:  ${:.2}", sol_maint);
        println!("  Limit Order Margin:  ${:.2}", sol_limit_order);
        println!("  Unrealized PnL:      ${:+.2}", sol_pnl);
    } else {
        println!("\nSOL Market: No position");
    }

    // BTC market margin (if position exists in computed margin)
    if let Some(btc_margin) = margin.positions.get("BTC") {
        let btc_initial = btc_margin.margin.initial_margin.as_inner() as f64 / 1_000_000.0;
        let btc_maint = btc_margin.margin.maintenance_margin.as_inner() as f64 / 1_000_000.0;
        let btc_pnl = btc_margin.margin.unrealized_pnl.as_inner() as f64 / 1_000_000.0;
        let btc_limit_order = btc_margin.margin.limit_order_margin.as_inner() as f64 / 1_000_000.0;

        println!("\nBTC Market:");
        if let Some(pos) = btc_margin.position {
            println!(
                "  Position:            {} lots",
                pos.base_lot_position.as_inner()
            );
        }
        println!("  Initial Margin:      ${:.2}", btc_initial);
        println!("  Maintenance Margin:  ${:.2}", btc_maint);
        println!("  Limit Order Margin:  ${:.2}", btc_limit_order);
        println!("  Unrealized PnL:      ${:+.2}", btc_pnl);
    } else {
        println!("\nBTC Market: No position");
    }

    println!("====================================\n");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse authority pubkey from command line
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <AUTHORITY_PUBKEY>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} 3z9vL1zjN6qyAFHhHQdWYRTFAcy69pJydkZmSFBKHg1R", args[0]);
        std::process::exit(1);
    }

    let authority_str = &args[1];
    let authority = authority_str
        .parse::<Pubkey>()
        .map_err(|e| format!("Invalid pubkey: {}", e))?;

    println!("Phoenix Margin Monitor");
    println!("Authority: {}\n", authority_str);

    // ========================================================================
    // STEP 1: Bootstrap - Fetch static exchange config via HTTP
    // ========================================================================

    println!("[1/5] Bootstrapping exchange configuration...");
    let http_client = PhoenixHttpClient::new_from_env()?;
    let exchange_response = http_client.get_exchange().await?;
    let exchange: ExchangeView = exchange_response.into();
    println!("  Loaded {} markets\n", exchange.markets.len());

    // ========================================================================
    // STEP 2: Prepare market symbols from exchange config
    // ========================================================================

    println!("[2/5] Preparing market metadata...");

    // Build PhoenixMetadata with cached calculators for all markets
    // Note: PerpAssetMetadata will be populated once we receive mark prices from
    // WebSocket
    let mut metadata = PhoenixMetadata::new(exchange);

    println!(
        "  Prepared {} markets with cached calculators\n",
        metadata.exchange().markets.len()
    );

    // ========================================================================
    // STEP 3: Initialize trader state container
    // ========================================================================

    println!("[3/5] Initializing trader state...");
    let trader_key = TraderKey::from_authority(authority);
    let mut trader = Trader::new(trader_key.clone());

    println!("  Trader state will populate from WebSocket\n");

    // Portfolio will be built from trader state updates
    let mut portfolio = TraderPortfolio::default();

    // ========================================================================
    // STEP 4: Connect to WebSocket and subscribe to channels
    // ========================================================================

    println!("[4/5] Connecting to WebSocket...");
    let ws_url = env::var("PHOENIX_WS_URL")
        .unwrap_or_else(|_| "wss://perp-api.phoenix.trade/ws".to_string());

    let ws_client = PhoenixWSClient::new_from_env()?;
    println!("  Connected to {}\n", ws_url);

    // Subscribe to market updates for each market (to get mark prices)
    // The public API requires subscribing to each market individually
    println!("[5/5] Subscribing to data streams...");

    // Get market symbols from exchange config and subscribe to each
    let market_symbols: Vec<String> = metadata.exchange().markets.keys().cloned().collect();

    // Create a channel to merge all market updates
    let (market_tx, mut market_stats_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut _market_handles = Vec::new();

    for symbol in &market_symbols {
        let (rx, handle) = ws_client.subscribe_to_market(symbol.clone())?;
        _market_handles.push(handle);

        let tx = market_tx.clone();
        let symbol_clone = symbol.clone();
        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(msg) = rx.recv().await {
                if tx.send(msg).is_err() {
                    break;
                }
            }
            tracing::debug!("Market subscription for {} ended", symbol_clone);
        });
    }
    drop(market_tx); // Drop the original sender so the channel closes when all spawned tasks end

    println!(
        "  Subscribed to market updates for {} markets",
        market_symbols.len()
    );

    // Subscribe to trader-state for position updates
    let (mut trader_state_rx, _trader_state_handle) =
        ws_client.subscribe_to_trader_state(&trader_key.authority())?;
    println!("  Subscribed to trader-state for {}\n", authority_str);

    // Track if we've initialized metadata (need at least one price update)
    let mut initialized_markets = std::collections::HashSet::new();
    let mut last_risk_tier = None;

    println!("----------------------------------------------------");
    println!("Live Margin Monitor Started");
    println!("----------------------------------------------------\n");

    // ========================================================================
    // STEP 5: Main event loop - Process WebSocket updates
    // ========================================================================

    loop {
        select! {
            // Handle market stats updates (mark price changes)
            Some(stats) = market_stats_rx.recv() => {
                // Initialize or update metadata for this market
                let is_new = !initialized_markets.contains(&stats.symbol);
                match metadata.apply_market_stats(&stats) {
                    Ok(()) => {
                        if is_new {
                            initialized_markets.insert(stats.symbol.clone());
                            println!("Initialized {}: mark=${:.2}", stats.symbol, stats.mark_price);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to apply stats for {}: {}", stats.symbol, e);
                    }
                }

                // Recompute margin if we have initialized markets and positions
                if let Some(subaccount) = trader.primary_subaccount() {
                    if metadata.initialized_market_count() > 0 && !portfolio.positions.is_empty() {
                        match portfolio.compute_margin(metadata.all_perp_asset_metadata()) {
                            Ok(margin) => {
                                let risk_tier = margin.risk_tier().ok();

                                if risk_tier != last_risk_tier {
                                    println!("\nRISK TIER CHANGE: {:?} -> {:?}", last_risk_tier, risk_tier);
                                    last_risk_tier = risk_tier;
                                }

                                println!("Price update: {} @ ${:.2}", stats.symbol, stats.mark_price);
                                print_margin_summary(&margin, subaccount);
                            }
                            Err(e) => {
                                eprintln!("Margin calculation error: {}", e);
                            }
                        }
                    }
                }
            }

            // Handle trader state updates (position changes)
            Some(msg) = trader_state_rx.recv() => {
                // Apply update to Trader state container
                trader.apply_update(&msg);

                // Rebuild portfolio from trader state using new conversion methods
                if let Some(subaccount) = trader.primary_subaccount() {
                    portfolio = subaccount.to_trader_portfolio();

                    println!("\nTrader State Updated (slot {})", trader.last_slot);
                    println!("  Collateral: ${:.2}", subaccount.collateral);

                    for (symbol, pos) in &subaccount.positions {
                        println!("  {} position: {} lots @ ${:.2}",
                            symbol,
                            pos.base_position_lots,
                            pos.entry_price_usd
                        );
                    }

                    // Recompute and print margin after trader state update
                    if metadata.initialized_market_count() > 0 && !portfolio.positions.is_empty() {
                        if let Ok(margin) = portfolio.compute_margin(metadata.all_perp_asset_metadata()) {
                            print_margin_summary(&margin, subaccount);
                            last_risk_tier = margin.risk_tier().ok();
                        }
                    }
                }
            }

            else => {
                println!("All channels closed, exiting...");
                break;
            }
        }
    }

    Ok(())
}
