//! Example: Query exchange keys, SOL market, and trader info via HTTP API.
//!
//! Demonstrates the resource-based sub-client API:
//!   client.markets().get_markets()
//!   client.exchange().get_keys()
//!   client.traders().get_trader(&authority)
//!   etc.
//!
//! Run with:
//!   export TRADER_PUBKEY=your_trader_pubkey  # optional
//!   cargo run -p phoenix-rise --example http_client

use std::str::FromStr;

use phoenix_rise::{
    CandlesQueryParams, CollateralHistoryQueryParams, FundingHistoryQueryParams,
    OrderHistoryQueryParams, PhoenixHttpClient, PnlQueryParams, PnlResolution, Timeframe,
    TradeHistoryQueryParams,
};
use solana_pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Phoenix HTTP API...\n");

    // Create HTTP client (uses PHOENIX_API_URL if set)
    let client = PhoenixHttpClient::new_from_env()?;

    // Fetch exchange keys
    println!("=== Exchange Keys ===");
    let exchange_keys = client.exchange().get_keys().await?;
    println!("  Global Config:     {}", exchange_keys.global_config);
    println!("  Canonical Mint:    {}", exchange_keys.canonical_mint);
    println!("  Global Vault:      {}", exchange_keys.global_vault);
    println!("  Perp Asset Map:    {}", exchange_keys.perp_asset_map);
    println!("  Withdraw Queue:    {}", exchange_keys.withdraw_queue);
    println!(
        "  Trader Index Accs: {}",
        exchange_keys.global_trader_index.len()
    );
    println!(
        "  Active Trader Buf: {}",
        exchange_keys.active_trader_buffer.len()
    );
    println!("\n  Current Authorities:");
    println!(
        "    Root:   {}",
        exchange_keys.current_authorities.root_authority
    );
    println!(
        "    Risk:   {}",
        exchange_keys.current_authorities.risk_authority
    );
    println!(
        "    Market: {}",
        exchange_keys.current_authorities.market_authority
    );
    println!(
        "    Oracle: {}",
        exchange_keys.current_authorities.oracle_authority
    );
    println!();

    // Fetch SOL market config (static configuration, not live data)
    println!("=== SOL Market Config ===");
    let market = client.markets().get_market("SOL").await?;
    println!("  Symbol:            {}", market.symbol);
    println!("  Asset ID:          {}", market.asset_id);
    println!("  Status:            {:?}", market.market_status);
    println!("  Market Pubkey:     {}", market.market_pubkey);
    println!("  Spline Pubkey:     {}", market.spline_pubkey);
    println!("  Tick Size:         {}", market.tick_size);
    println!("  Base Lot Decimals: {}", market.base_lots_decimals);
    println!("  Isolated Only:     {}", market.isolated_only);

    println!("\n  Fees:");
    println!("    Taker Fee:       {:.4}%", market.taker_fee * 100.0);
    println!("    Maker Fee:       {:.4}%", market.maker_fee * 100.0);

    println!("\n  Funding:");
    println!(
        "    Interval:        {} seconds",
        market.funding_interval_seconds
    );
    println!(
        "    Period:          {} seconds",
        market.funding_period_seconds
    );
    println!(
        "    Max Rate/Int:    {:.4}%",
        market.max_funding_rate_per_interval * 100.0
    );

    println!("\n  Leverage Tiers:");
    for (i, tier) in market.leverage_tiers.iter().enumerate() {
        println!(
            "    Tier {}: max {:.1}x leverage, max {} base lots",
            i + 1,
            tier.max_leverage,
            tier.max_size_base_lots
        );
    }

    println!("\n  Risk Factors:");
    println!(
        "    Maintenance:     {:.2}%",
        market.risk_factors.maintenance / 100.0
    );
    println!(
        "    Backstop:        {:.2}%",
        market.risk_factors.backstop / 100.0
    );
    println!(
        "    High Risk:       {:.2}%",
        market.risk_factors.high_risk / 100.0
    );

    println!("\n  Caps:");
    println!(
        "    OI Cap:          {} base lots",
        market.open_interest_cap_base_lots
    );
    println!(
        "    Max Liq Size:    {} base lots",
        market.max_liquidation_size_base_lots
    );
    println!();

    // Fetch SOL candles
    println!("=== SOL Candles (1m) ===");
    let params = CandlesQueryParams::new("SOL", Timeframe::Minute1).with_limit(5);
    let candles = client.candles().get_candles(params).await?;
    println!("  Latest {} candles:", candles.len());
    for candle in &candles {
        println!(
            "    {} | O: ${:.2} H: ${:.2} L: ${:.2} C: ${:.2} V: {:.1}",
            candle.time,
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume.unwrap_or(0.0)
        );
    }
    println!();

    // Fetch all markets (static configuration)
    println!("=== All Markets ===");
    let markets = client.markets().get_markets().await?;
    println!("  Markets ({} total):", markets.len());
    for m in &markets {
        println!(
            "    {}: {:?} (max leverage: {}x, isolated: {})",
            m.symbol,
            m.market_status,
            m.leverage_tiers
                .first()
                .map(|t| t.max_leverage)
                .unwrap_or(0.0),
            m.isolated_only
        );
    }
    println!();

    // Fetch trader info (if TRADER_PUBKEY env var is set)
    if let Ok(pubkey_str) = std::env::var("TRADER_PUBKEY") {
        println!("=== Trader Subaccounts ===");
        let authority = Pubkey::from_str(&pubkey_str)?;
        let traders = client.traders().get_trader(&authority).await?;
        println!("  Found {} subaccount(s)\n", traders.len());

        for trader in &traders {
            println!(
                "  --- Subaccount {} (PDA index: {}) ---",
                trader.trader_subaccount_index, trader.trader_pda_index
            );
            println!("  Trader Key:        {}", trader.trader_key);
            println!("  State:             {:?}", trader.state);
            println!("  Collateral:        {}", trader.collateral_balance.ui);
            println!("  Portfolio Value:   {}", trader.portfolio_value.ui);
            println!("  Unrealized PnL:    {}", trader.unrealized_pnl.ui);
            println!("  Risk State:        {:?}", trader.risk_state);
            println!("  Risk Tier:         {:?}", trader.risk_tier);

            if !trader.positions.is_empty() {
                println!("\n  Positions:");
                for pos in &trader.positions {
                    println!(
                        "    {}: {} @ {} (uPnL: {})",
                        pos.symbol, pos.position_size.ui, pos.entry_price.ui, pos.unrealized_pnl.ui
                    );
                }
            }

            let order_count: usize = trader.limit_orders.values().map(|v| v.len()).sum();
            if order_count > 0 {
                println!("\n  Limit Orders ({} total):", order_count);
                for (symbol, orders) in &trader.limit_orders {
                    for order in orders {
                        println!(
                            "    {}: {:?} {} @ {}",
                            symbol, order.side, order.trade_size_remaining.ui, order.price.ui
                        );
                    }
                }
            }
            println!();
        }

        // Fetch trade history (fills) for this trader
        println!("=== Trade History ===");
        let params = TradeHistoryQueryParams::new().with_limit(10);
        let trades = client
            .trades()
            .get_trader_trade_history(&authority, params)
            .await?;
        println!("  Latest {} trades:", trades.data.len());
        for fill in &trades.data {
            println!(
                "    {} | {} {} {:?} @ {} | PnL: {} | Fees: {} | {}",
                fill.timestamp,
                fill.market_symbol,
                fill.base_lots_delta,
                fill.liquidity,
                fill.price,
                fill.realized_pnl,
                fill.fees,
                fill.signature.clone().unwrap_or("N/A".to_string())
            );
        }
        if trades.has_more {
            println!("  (more trades available via pagination)");
        }

        // Fetch collateral history for this trader
        println!("=== Collateral History ===");
        let params = CollateralHistoryQueryParams::new(10);
        let history = client
            .collateral()
            .get_user_collateral_history(&authority, params)
            .await?;
        println!("  Latest {} events:", history.data.len());
        for event in &history.data {
            println!(
                "    {} | {} {} (balance after: {})",
                event.timestamp, event.event_type, event.amount, event.collateral_after
            );
        }
        if history.has_more {
            println!("  (more events available via pagination)");
        }
        println!();

        // Fetch funding history for this trader
        println!("=== Funding History ===");
        let params = FundingHistoryQueryParams::new().with_limit(10);
        let funding = client
            .funding()
            .get_user_funding_history(&authority, params)
            .await?;
        println!("  Latest {} events:", funding.events.len());
        for event in &funding.events {
            println!(
                "    {} | {} {} USDC",
                event.timestamp, event.symbol, event.funding_payment
            );
        }
        if funding.has_more {
            println!("  (more events available via pagination)");
        }
        println!();

        // Fetch PnL time-series for the last 6 months
        println!("=== PnL (last 6 months, daily) ===");
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let six_months_ago_ms = now_ms - 180 * 24 * 60 * 60 * 1000;
        let params = PnlQueryParams::new(PnlResolution::Day1)
            .with_start_time(six_months_ago_ms)
            .with_end_time(now_ms);
        let pnl = client.traders().get_trader_pnl(&authority, params).await?;
        println!("  {} data points:", pnl.len());
        for point in &pnl {
            println!(
                "    {} | cumPnL: {:.2} | unrealized: {:.2} | funding: {:.2} | fees: {:.2}",
                point.timestamp,
                point.cumulative_pnl,
                point.unrealized_pnl,
                point.cumulative_funding_payment,
                point.cumulative_taker_fee,
            );
        }
        println!();

        // Fetch order history for this trader
        println!("=== Order History ===");
        let params = OrderHistoryQueryParams::new(10);
        let orders = client
            .orders()
            .get_trader_order_history(&authority, params)
            .await?;
        println!("  Latest {} orders:", orders.data.len());
        for order in &orders.data {
            println!(
                "    {} | {:?} {:?} {} @ {} ({:?})",
                order.market_symbol,
                order.side,
                order.status,
                order.base_qty,
                order.price,
                order
                    .placed_at
                    .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            );
        }

        // Demonstrate pagination: fetch next page if available
        if orders.has_more {
            if let Some(cursor) = &orders.next_cursor {
                println!("\n  Fetching next page...");
                let next_params = OrderHistoryQueryParams::new(5).with_cursor(cursor);
                let next_page = client
                    .orders()
                    .get_trader_order_history(&authority, next_params)
                    .await?;
                println!("  Next {} orders:", next_page.data.len());
                for order in &next_page.data {
                    println!(
                        "    {} | {:?} {:?} {} @ {}",
                        order.market_symbol, order.side, order.status, order.base_qty, order.price
                    );
                }
            }
        }

        // Demonstrate market filter
        println!("\n  Filtering by SOL market:");
        let sol_params = OrderHistoryQueryParams::new(5).with_market_symbol("SOL");
        let sol_orders = client
            .orders()
            .get_trader_order_history(&authority, sol_params)
            .await?;
        println!("  Found {} SOL orders", sol_orders.data.len());
        println!();

        // Demonstrate market filter for trades
        println!("\n  Filtering trades by SOL market:");
        let sol_params = TradeHistoryQueryParams::new()
            .with_market_symbol("SOL")
            .with_limit(5);
        let sol_trades = client
            .trades()
            .get_trader_trade_history(&authority, sol_params)
            .await?;
        println!("  Found {} SOL trades", sol_trades.data.len());
        println!();
    }

    // Fetch exchange config (static market parameters)
    println!("=== Exchange Config ===");
    let exchange = client.exchange().get_exchange().await?;
    println!("  Markets ({} total):", exchange.markets.len());
    for market in &exchange.markets {
        println!("\n  {} Market Config:", market.symbol);
        println!("    Spline Collection: {}", market.spline_pubkey);
        println!(
            "    Fees: taker {:.4}%, maker {:.4}%",
            market.taker_fee * 100.0,
            market.maker_fee * 100.0
        );
        println!(
            "    Funding: {} sec interval, {} sec period",
            market.funding_interval_seconds, market.funding_period_seconds
        );
        println!(
            "    Max Funding Rate/Interval: {:.4}%",
            market.max_funding_rate_per_interval * 100.0
        );
        println!(
            "    OI Cap: {} base lots",
            market.open_interest_cap_base_lots
        );
        println!(
            "    Max Liquidation Size: {} base lots",
            market.max_liquidation_size_base_lots
        );
        println!("    Isolated Only: {}", market.isolated_only);
        println!("    Risk Factors:");
        println!(
            "      Maintenance: {:.2}%",
            market.risk_factors.maintenance / 100.0
        );
        println!(
            "      Backstop: {:.2}%",
            market.risk_factors.backstop / 100.0
        );
        println!(
            "      High Risk: {:.2}%",
            market.risk_factors.high_risk / 100.0
        );
    }
    println!();

    Ok(())
}
