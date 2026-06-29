//! Example: Naive inventory-aware market maker on Phoenix Perps.
//!
//! Quotes one tick inside the current spread on each market/trader update,
//! cancels existing orders before requoting, and adjusts bid/ask volume
//! based on current inventory (linear skew).
//!
//! Run with:
//!   export KEYPAIR_PATH=/path/to/keypair.json
//!   export PHOENIX_API_URL=https://perp-api.phoenix.trade
//!   cargo run -p phoenix-rise --example market_maker --features solana-keypair
//! -- SOL 100 1000

use std::env;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use phoenix_rise::api::{
    PhoenixClient, PhoenixClientEvent, PhoenixHttpClient, PhoenixMetadata, PhoenixSubscription,
    SubscriptionKey, Trader, TraderKey,
};
use phoenix_rise::core::{LimitOrderTicket, PhoenixTxBuilder};
use phoenix_rise::ix::types::{CancelId, Side};
use phoenix_rise::math::Ticks;
use phoenix_rise::types::market_state::Market;
use solana_commitment_config::CommitmentConfig;
use solana_keypair::{Keypair, read_keypair_file};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";
const REQUOTE_INTERVAL_MS: u64 = 5_000;

fn apply_market_event(event: PhoenixClientEvent, market_state: &mut Option<Market>) {
    match event {
        PhoenixClientEvent::MarketUpdate {
            symbol,
            prev_market,
            update,
        } => {
            let mut market = prev_market.unwrap_or_else(|| Market::from_symbol(symbol));
            market.apply_market_stats_update(&update);
            *market_state = Some(market);
        }
        PhoenixClientEvent::OrderbookUpdate {
            symbol,
            prev_market,
            update,
        } => {
            let mut market = prev_market.unwrap_or_else(|| Market::from_symbol(symbol));
            market.apply_l2_book_update(&update);
            *market_state = Some(market);
        }
        _ => {}
    }
}

fn apply_trader_event(
    event: PhoenixClientEvent,
    authority: Pubkey,
    trader_state: &mut Option<Trader>,
) {
    if let PhoenixClientEvent::TraderUpdate {
        prev_trader,
        update,
        ..
    } = event
    {
        let mut trader = prev_trader
            .unwrap_or_else(|| Trader::new(TraderKey::from_authority_with_idx(authority, 0, 0)));
        trader.apply_update(&update);
        *trader_state = Some(trader);
    }
}

async fn try_requote(
    symbol: &str,
    order_size_lots: u64,
    max_position_lots: i64,
    tick_size_usd: f64,
    last_quote: &Mutex<Instant>,
    market_state: &Option<Market>,
    trader_state: &Option<Trader>,
    builder: &PhoenixTxBuilder<'_>,
    rpc: &Arc<RpcClient>,
    keypair: &Arc<Keypair>,
    authority: Pubkey,
) {
    {
        let last = last_quote.lock();
        if last.elapsed().as_millis() < REQUOTE_INTERVAL_MS as u128 {
            return;
        }
    }

    let Some(market) = market_state.as_ref() else {
        return;
    };
    let Some(best_bid) = market.best_bid() else {
        return;
    };
    let Some(best_ask) = market.best_ask() else {
        return;
    };

    let mut cancel_ids = Vec::new();
    if let Some(ts) = trader_state.as_ref() {
        for order in ts.all_orders() {
            if order.symbol != symbol {
                continue;
            }
            cancel_ids.push(CancelId::new(
                order.price_ticks as u64,
                order.order_sequence_number,
            ));
        }
    }

    let mut instructions = Vec::new();

    if !cancel_ids.is_empty() {
        match builder.build_cancel_orders(
            authority,
            TraderKey::new(authority).pda(),
            symbol,
            cancel_ids,
        ) {
            Ok(ixs) => instructions.extend(ixs),
            Err(e) => {
                eprintln!("Failed to build cancel ix: {}", e);
                return;
            }
        }
    }

    let position_lots: i64 = trader_state
        .as_ref()
        .and_then(|ts| ts.subaccount(0))
        .and_then(|sub| sub.positions.get(symbol))
        .map(|pos| pos.base_position_lots)
        .unwrap_or(0);

    let skew = (position_lots as f64 / max_position_lots as f64).clamp(-1.0, 1.0);
    let bid_size = (order_size_lots as f64 * (1.0 - skew)) as u64;
    let ask_size = (order_size_lots as f64 * (1.0 + skew)) as u64;

    let bid_price = best_bid + tick_size_usd;
    let ask_price = best_ask - tick_size_usd;

    if bid_price >= ask_price {
        if instructions.is_empty() {
            return;
        }
    } else {
        let trader_pda = TraderKey::new(authority).pda();

        if bid_size > 0 {
            let ticket = LimitOrderTicket::builder()
                .authority(authority)
                .trader_account(trader_pda)
                .symbol(symbol)
                .side(Side::Bid)
                .price(bid_price)
                .num_base_lots(bid_size)
                .build();
            match ticket {
                Ok(ticket) => match builder.place_limit_order(ticket).await {
                    Ok(ixs) => instructions.extend(ixs),
                    Err(e) => eprintln!("Failed to build bid ix: {}", e),
                },
                Err(e) => eprintln!("Failed to build bid ticket: {}", e),
            }
        }

        if ask_size > 0 {
            let ticket = LimitOrderTicket::builder()
                .authority(authority)
                .trader_account(trader_pda)
                .symbol(symbol)
                .side(Side::Ask)
                .price(ask_price)
                .num_base_lots(ask_size)
                .build();
            match ticket {
                Ok(ticket) => match builder.place_limit_order(ticket).await {
                    Ok(ixs) => instructions.extend(ixs),
                    Err(e) => eprintln!("Failed to build ask ix: {}", e),
                },
                Err(e) => eprintln!("Failed to build ask ticket: {}", e),
            }
        }
    }

    if instructions.is_empty() {
        return;
    }

    *last_quote.lock() = Instant::now();

    let rpc = rpc.clone();
    let keypair = keypair.clone();
    let symbol = symbol.to_string();
    tokio::spawn(async move {
        let blockhash = match rpc.get_latest_blockhash().await {
            Ok(bh) => bh,
            Err(e) => {
                eprintln!("Failed to get blockhash: {}", e);
                return;
            }
        };

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&authority),
            &[&*keypair],
            blockhash,
        );

        match rpc.send_and_confirm_transaction(&tx).await {
            Ok(sig) => {
                println!(
                    "Requoted {}: bid {:.4} x {} | ask {:.4} x {} | pos={} | sig={}",
                    symbol, bid_price, bid_size, ask_price, ask_size, position_lots, sig
                );
            }
            Err(e) => eprintln!("Transaction failed: {}", e),
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: market_maker <SYMBOL> <ORDER_SIZE_LOTS> <MAX_POSITION_LOTS>");
        eprintln!("Example: market_maker SOL 100 1000");
        std::process::exit(1);
    }

    let symbol = args[1].to_ascii_uppercase();
    let order_size_lots: u64 = args[2].parse()?;
    let max_position_lots: i64 = args[3].parse()?;

    let keypair_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME environment variable not set");
        format!("{}/.config/solana/id.json", home)
    });
    println!("Loading keypair from: {}", keypair_path);

    let keypair = Arc::new(
        read_keypair_file(&keypair_path).map_err(|e| format!("Failed to read keypair: {}", e))?,
    );
    let trader = TraderKey::new(keypair.pubkey());
    let authority = trader.authority();
    let pda = trader.pda();

    println!("Authority: {}", authority);
    println!("PDA:       {}", pda);

    println!("\nFetching exchange metadata...");
    let http = PhoenixHttpClient::new_from_env()?;
    let exchange = http.get_exchange().await?.into();
    let metadata: &'static PhoenixMetadata = Box::leak(Box::new(PhoenixMetadata::new(exchange)));

    let calc = metadata
        .get_market_calculator(&symbol)
        .ok_or_else(|| format!("Unknown symbol: {}", symbol))?;
    let tick_size_usd = calc.ticks_to_price(Ticks::new(1));
    println!("Tick size for {}: ${:.6}", symbol, tick_size_usd);

    let builder = PhoenixTxBuilder::new(metadata);
    let rpc = Arc::new(RpcClient::new_with_commitment(
        RPC_ENDPOINT.to_string(),
        CommitmentConfig::confirmed(),
    ));

    println!("\nConnecting PhoenixClient...");
    let client = PhoenixClient::new_from_env().await?;

    let (mut market_rx, _market_handle) = client
        .subscribe(PhoenixSubscription::market(symbol.clone()))
        .await?;
    let (mut trader_rx, _trader_handle) = client
        .subscribe(PhoenixSubscription::Key(SubscriptionKey::trader(
            &authority, 0,
        )))
        .await?;

    let mut market_state: Option<Market> = None;
    let mut trader_state: Option<Trader> = None;
    let last_quote = Mutex::new(
        Instant::now()
            .checked_sub(std::time::Duration::from_millis(REQUOTE_INTERVAL_MS))
            .unwrap_or_else(Instant::now),
    );

    println!(
        "\nMarket maker running: {} | size={} lots | max_pos={} lots | requote={}ms",
        symbol, order_size_lots, max_position_lots, REQUOTE_INTERVAL_MS
    );
    println!("Press Ctrl+C to stop\n");

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            Some(event) = market_rx.recv() => {
                apply_market_event(event, &mut market_state);
                try_requote(
                    &symbol,
                    order_size_lots,
                    max_position_lots,
                    tick_size_usd,
                    &last_quote,
                    &market_state,
                    &trader_state,
                    &builder,
                    &rpc,
                    &keypair,
                    authority,
                ).await;
            }
            Some(event) = trader_rx.recv() => {
                apply_trader_event(event, authority, &mut trader_state);
                try_requote(
                    &symbol,
                    order_size_lots,
                    max_position_lots,
                    tick_size_usd,
                    &last_quote,
                    &market_state,
                    &trader_state,
                    &builder,
                    &rpc,
                    &keypair,
                    authority,
                ).await;
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
