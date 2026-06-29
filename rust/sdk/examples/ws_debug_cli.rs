//! Tiny WebSocket debug CLI driven by a TOML config file.
//!
//! Usage:
//!   cargo run -p phoenix-rise --example ws_debug_cli -- --config
//! sdk/examples/ws_debug_config.toml
//!   cargo run -p phoenix-rise --example ws_debug_cli -- --config
//! sdk/examples/ws_debug_config.toml --stats

use std::error::Error;
use std::io::{self, Write};
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::{env, fs};

use phoenix_rise::api::{PhoenixEnv, PhoenixWSClient, SubscriptionHandle, WsConnectionStatus};
use phoenix_rise::types::prelude::{
    AllMidsData, CandleData, FundingRateMessage, L2BookUpdate, MarketStatsUpdate, ServerMessage,
    Timeframe, TraderStatePayload, TraderStateServerMessage, TradesMessage,
};
use serde::Deserialize;
use serde_json::json;
use solana_pubkey::Pubkey;
use tokio::sync::mpsc;
use tokio::time::{self, MissedTickBehavior};

#[derive(Debug)]
struct CliArgs {
    config_path: String,
    stats: bool,
}

#[derive(Debug, Default, Deserialize)]
struct WsDebugConfig {
    #[serde(default)]
    connection: ConnectionConfig,
    #[serde(default)]
    subscriptions: SubscriptionsConfig,
}

#[derive(Debug, Default, Deserialize)]
struct ConnectionConfig {
    ws_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SubscriptionsConfig {
    #[serde(default)]
    all_mids: bool,
    #[serde(default)]
    funding_rate: Vec<String>,
    #[serde(default)]
    orderbook: Vec<String>,
    #[serde(default)]
    market: Vec<String>,
    #[serde(default)]
    trades: Vec<String>,
    #[serde(default)]
    candles: Vec<CandlesSubscriptionConfig>,
    #[serde(default)]
    trader_state: Vec<TraderStateSubscriptionConfig>,
}

#[derive(Debug, Deserialize)]
struct CandlesSubscriptionConfig {
    symbol: String,
    timeframe: String,
}

#[derive(Debug, Deserialize)]
struct TraderStateSubscriptionConfig {
    authority: String,
    #[serde(default)]
    trader_pda_index: u8,
}

enum IncomingEvent {
    Status(WsConnectionStatus),
    AllMids(AllMidsData),
    FundingRate(FundingRateMessage),
    Orderbook(L2BookUpdate),
    Market(MarketStatsUpdate),
    Trades(TradesMessage),
    Candles(CandleData),
    TraderState(TraderStateServerMessage),
}

#[derive(Default)]
struct Metrics {
    total: u64,
    status: u64,
    all_mids: u64,
    funding_rate: u64,
    orderbook: u64,
    market: u64,
    trades: u64,
    candles: u64,
    trader_state: u64,
    trader_snapshots: u64,
    trader_deltas: u64,
    connection_open_since: Option<Instant>,
    last_connection_open: Option<Duration>,
}

impl Metrics {
    fn record(&mut self, event: &IncomingEvent, now: Instant) {
        self.total += 1;
        match event {
            IncomingEvent::Status(status) => {
                self.status += 1;
                match status {
                    WsConnectionStatus::Connected => {
                        self.connection_open_since = Some(now);
                    }
                    WsConnectionStatus::Disconnected(_) | WsConnectionStatus::ConnectionFailed => {
                        if let Some(since) = self.connection_open_since.take() {
                            self.last_connection_open = Some(now.saturating_duration_since(since));
                        }
                    }
                    WsConnectionStatus::Connecting => {}
                }
            }
            IncomingEvent::AllMids(_) => self.all_mids += 1,
            IncomingEvent::FundingRate(_) => self.funding_rate += 1,
            IncomingEvent::Orderbook(_) => self.orderbook += 1,
            IncomingEvent::Market(_) => self.market += 1,
            IncomingEvent::Trades(_) => self.trades += 1,
            IncomingEvent::Candles(_) => self.candles += 1,
            IncomingEvent::TraderState(msg) => {
                self.trader_state += 1;
                match &msg.content {
                    TraderStatePayload::Snapshot(_) => self.trader_snapshots += 1,
                    TraderStatePayload::Delta(_) => self.trader_deltas += 1,
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args().map_err(|e| format!("{e}\n\n{}", usage()))?;
    let config = load_config(&args.config_path)?;

    let mut env_cfg = PhoenixEnv::load();
    if let Some(ws_url) = config.connection.ws_url {
        env_cfg.ws_url = ws_url;
    }

    let mut client = PhoenixWSClient::from_env_with_connection_status(env_cfg)?;
    let status_rx = client
        .connection_status_receiver()
        .ok_or("connection status receiver unavailable")?;

    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    spawn_forwarder(status_rx, event_tx.clone(), IncomingEvent::Status);

    let mut _handles: Vec<SubscriptionHandle> = Vec::new();
    let mut configured_streams = 0usize;

    if config.subscriptions.all_mids {
        let (rx, handle) = client.subscribe_to_all_mids()?;
        _handles.push(handle);
        configured_streams += 1;
        spawn_forwarder(rx, event_tx.clone(), IncomingEvent::AllMids);
    }

    for symbol in config.subscriptions.funding_rate {
        let symbol = symbol.to_ascii_uppercase();
        let (rx, handle) = client.subscribe_to_funding_rate(symbol)?;
        _handles.push(handle);
        configured_streams += 1;
        spawn_forwarder(rx, event_tx.clone(), IncomingEvent::FundingRate);
    }

    for symbol in config.subscriptions.orderbook {
        let symbol = symbol.to_ascii_uppercase();
        let (rx, handle) = client.subscribe_to_orderbook(symbol)?;
        _handles.push(handle);
        configured_streams += 1;
        spawn_forwarder(rx, event_tx.clone(), IncomingEvent::Orderbook);
    }

    for symbol in config.subscriptions.market {
        let symbol = symbol.to_ascii_uppercase();
        let (rx, handle) = client.subscribe_to_market(symbol)?;
        _handles.push(handle);
        configured_streams += 1;
        spawn_forwarder(rx, event_tx.clone(), IncomingEvent::Market);
    }

    for symbol in config.subscriptions.trades {
        let symbol = symbol.to_ascii_uppercase();
        let (rx, handle) = client.subscribe_to_trades(symbol)?;
        _handles.push(handle);
        configured_streams += 1;
        spawn_forwarder(rx, event_tx.clone(), IncomingEvent::Trades);
    }

    for sub in config.subscriptions.candles {
        let symbol = sub.symbol.to_ascii_uppercase();
        let timeframe: Timeframe = sub.timeframe.parse().map_err(|e| {
            format!(
                "invalid timeframe '{}' for {}: {}",
                sub.timeframe, symbol, e
            )
        })?;
        let (rx, handle) = client.subscribe_to_candles(symbol, timeframe)?;
        _handles.push(handle);
        configured_streams += 1;
        spawn_forwarder(rx, event_tx.clone(), IncomingEvent::Candles);
    }

    for sub in config.subscriptions.trader_state {
        let authority = Pubkey::from_str(&sub.authority)
            .map_err(|e| format!("invalid trader authority '{}': {}", sub.authority, e))?;
        let (rx, handle) =
            client.subscribe_to_trader_state_with_pda(&authority, sub.trader_pda_index)?;
        _handles.push(handle);
        configured_streams += 1;
        spawn_forwarder(rx, event_tx.clone(), IncomingEvent::TraderState);
    }

    if configured_streams == 0 {
        return Err("no subscriptions enabled in config file".into());
    }

    println!(
        "Connected and subscribed to {} stream(s). Press Ctrl+C to exit.",
        configured_streams
    );

    drop(event_tx);

    let started = Instant::now();
    let mut metrics = Metrics::default();
    let mut ticker = time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    if args.stats {
        print_stats(&metrics, started);
    }

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
            _ = ticker.tick(), if args.stats => {
                print_stats(&metrics, started);
            }
            maybe_event = event_rx.recv() => {
                match maybe_event {
                    Some(event) => {
                        metrics.record(&event, Instant::now());
                        if args.stats {
                            continue;
                        }
                        print_event_json(event)?;
                    }
                    None => {
                        println!("All subscription channels closed.");
                        break;
                    }
                }
            }
        }
    }

    if args.stats {
        print_stats(&metrics, started);
        println!();
    }

    Ok(())
}

fn spawn_forwarder<T, F>(
    mut rx: mpsc::UnboundedReceiver<T>,
    tx: mpsc::UnboundedSender<IncomingEvent>,
    map: F,
) where
    T: Send + 'static,
    F: Fn(T) -> IncomingEvent + Send + 'static,
{
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if tx.send(map(message)).is_err() {
                break;
            }
        }
    });
}

fn print_event_json(event: IncomingEvent) -> Result<(), serde_json::Error> {
    match event {
        IncomingEvent::Status(status) => {
            let payload = match status {
                WsConnectionStatus::Connecting => {
                    json!({"type":"connectionStatus","status":"connecting"})
                }
                WsConnectionStatus::Connected => {
                    json!({"type":"connectionStatus","status":"connected"})
                }
                WsConnectionStatus::ConnectionFailed => {
                    json!({"type":"connectionStatus","status":"connectionFailed"})
                }
                WsConnectionStatus::Disconnected(reason) => {
                    json!({"type":"connectionStatus","status":"disconnected","reason":reason})
                }
            };
            println!("🔌 {}", serde_json::to_string(&payload)?);
        }
        IncomingEvent::AllMids(msg) => {
            println!(
                "📈 {}",
                serde_json::to_string(&ServerMessage::AllMids(msg))?
            );
        }
        IncomingEvent::FundingRate(msg) => {
            println!(
                "💸 {}",
                serde_json::to_string(&ServerMessage::FundingRate(msg))?
            );
        }
        IncomingEvent::Orderbook(msg) => {
            println!(
                "📚 {}",
                serde_json::to_string(&ServerMessage::Orderbook(msg))?
            );
        }
        IncomingEvent::Market(msg) => {
            println!("📊 {}", serde_json::to_string(&ServerMessage::Market(msg))?);
        }
        IncomingEvent::Trades(msg) => {
            println!("💱 {}", serde_json::to_string(&ServerMessage::Trades(msg))?);
        }
        IncomingEvent::Candles(msg) => {
            println!(
                "🕯️ {}",
                serde_json::to_string(&ServerMessage::Candles(msg))?
            );
        }
        IncomingEvent::TraderState(msg) => {
            println!(
                "👤 {}",
                serde_json::to_string(&ServerMessage::TraderState(msg))?
            );
        }
    }
    Ok(())
}

fn print_stats(metrics: &Metrics, started: Instant) {
    let elapsed = started.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 {
        metrics.total as f64 / elapsed
    } else {
        0.0
    };

    print!("\x1B[2J\x1B[H");
    println!("Phoenix WS Debug Stats");
    match metrics.connection_open_since {
        Some(since) => {
            println!("🔗 connected_for {:.1}s", since.elapsed().as_secs_f64());
        }
        None => {
            if let Some(last) = metrics.last_connection_open {
                println!(
                    "🔗 connected_for disconnected (last {:.1}s)",
                    last.as_secs_f64()
                );
            } else {
                println!("🔗 connected_for disconnected");
            }
        }
    }
    println!("Elapsed: {:.1}s", elapsed);
    println!("Total messages: {} ({:.2}/s)", metrics.total, rate);
    println!();

    let print_metric = |emoji: &str, label: &str, count: u64| {
        let per_sec = if elapsed > 0.0 {
            count as f64 / elapsed
        } else {
            0.0
        };
        println!("{} {:<12} {:>6} ({:>6.2}/s)", emoji, label, count, per_sec);
    };

    print_metric("🔌", "status", metrics.status);
    print_metric("📈", "all_mids", metrics.all_mids);
    print_metric("💸", "funding_rate", metrics.funding_rate);
    print_metric("📚", "orderbook", metrics.orderbook);
    print_metric("📊", "market", metrics.market);
    print_metric("💱", "trades", metrics.trades);
    print_metric("🕯️", "candles", metrics.candles);
    print_metric("👤", "trader_state", metrics.trader_state);
    print_metric("📸", "snapshots", metrics.trader_snapshots);
    print_metric("🧩", "deltas", metrics.trader_deltas);
    let _ = io::stdout().flush();
}

fn load_config(path: &str) -> Result<WsDebugConfig, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let config: WsDebugConfig = toml::from_str(&raw)?;
    Ok(config)
}

fn parse_args() -> Result<CliArgs, String> {
    let mut args = env::args().skip(1);
    let mut config_path: Option<String> = None;
    let mut stats = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                let path = args
                    .next()
                    .ok_or("--config requires a path argument".to_string())?;
                config_path = Some(path);
            }
            "--stats" => {
                stats = true;
            }
            "-h" | "--help" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            other => {
                return Err(format!("unknown argument: {}", other));
            }
        }
    }

    let config_path = config_path.ok_or("--config is required".to_string())?;
    Ok(CliArgs { config_path, stats })
}

fn usage() -> &'static str {
    "Usage: ws_debug_cli --config <path> [--stats]

Examples:
  cargo run -p phoenix-rise --example ws_debug_cli -- --config sdk/examples/ws_debug_config.toml
  cargo run -p phoenix-rise --example ws_debug_cli -- --config sdk/examples/ws_debug_config.toml \
     --stats"
}
