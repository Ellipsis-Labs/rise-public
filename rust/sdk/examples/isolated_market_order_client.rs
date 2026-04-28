//! Example: Isolated margin market order via client-side construction.
//!
//! Demonstrates:
//! 1. Initialize a `PhoenixClient`
//! 2. Subscribe to trader state (explicit) and trader margin events
//! 3. Cache `Trader` state and `TraderPortfolioMargin` from their respective
//!    receivers
//! 4. Wait 10 seconds for state to settle
//! 5. Compute transferable collateral via `calculate_transferable_collateral`
//! 6. Build isolated market order via `PhoenixTxBuilder`
//!
//! Run with:
//!   export PHOENIX_API_URL=https://perp-api.phoenix.trade
//!   export PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws
//!   cargo run -p phoenix-rise --example isolated_market_order_client
//! --features solana-keypair -- SOL 10 5000000 [SL_PRICE|x] [TP_PRICE|x]
//!
//! Use "x" in place of a price to skip stop-loss or take-profit.

use std::env;

use phoenix_rise::math::{TraderPortfolioMargin, WrapperNum};
use phoenix_rise::{
    BracketLeg, BracketLegOrders, IsolatedCollateralFlow, MarginTrigger, PhoenixClient,
    PhoenixClientEvent, PhoenixMetadata, PhoenixSubscription, PhoenixTxBuilder, Side,
    SubscriptionKey, Trader, TraderKey,
};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";

fn print_margin(margin: &TraderPortfolioMargin) {
    println!(
        "  collateral:             {} signed_quote_lots",
        margin.quote_lot_collateral.as_inner()
    );
    println!(
        "  portfolio value:        {} signed_quote_lots",
        margin.portfolio_value().as_inner()
    );
    println!(
        "  effective collateral:   {} signed_quote_lots",
        margin.effective_collateral().as_inner()
    );
    println!(
        "  initial margin:         {} quote_lots",
        margin.margin.initial_margin.as_inner()
    );
    println!(
        "  initial margin (wdraw): {} quote_lots",
        margin.margin.initial_margin_for_withdrawals.as_inner()
    );
    println!(
        "  maintenance margin:     {} quote_lots",
        margin.margin.maintenance_margin.as_inner()
    );
    println!(
        "  limit order margin:     {} quote_lots",
        margin.margin.limit_order_margin.as_inner()
    );
    println!(
        "  unrealized pnl:         {} signed_quote_lots",
        margin.margin.unrealized_pnl.as_inner()
    );
    println!(
        "  discounted pnl:         {} signed_quote_lots",
        margin.margin.discounted_unrealized_pnl.as_inner()
    );
    println!(
        "  unsettled funding:      {} signed_quote_lots",
        margin.margin.unsettled_funding.as_inner()
    );
    println!(
        "  position value:         {} signed_quote_lots",
        margin.margin.position_value.as_inner()
    );
    println!("  risk tier:              {:?}", margin.risk_tier().ok());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!(
            "Usage: isolated_market_order_client <SYMBOL> <NUM_BASE_LOTS> <COLLATERAL> \
             [SL_PRICE|x] [TP_PRICE|x]"
        );
        eprintln!("  NUM_BASE_LOTS: positive for bid, negative for ask");
        eprintln!("Example: isolated_market_order_client SOL 10 5.0 120.5 150.0");
        std::process::exit(1);
    }

    let symbol = args[1].clone();
    let num_base_lots_signed: i64 = args[2].parse().expect("NUM_BASE_LOTS must be an integer");
    let collateral: u64 = args[3]
        .parse()
        .expect("COLLATERAL must be quote lots (u64)");

    fn parse_price(arg: &str) -> Option<f64> {
        if arg.eq_ignore_ascii_case("x") {
            return None;
        }
        Some(arg.parse::<f64>().expect("invalid price"))
    }

    let sl_price = args.get(4).and_then(|s| parse_price(s));
    let tp_price = args.get(5).and_then(|s| parse_price(s));
    let bracket = if sl_price.is_some() || tp_price.is_some() {
        Some(BracketLegOrders {
            stop_loss: sl_price.map(BracketLeg::new),
            take_profit: tp_price.map(BracketLeg::new),
        })
    } else {
        None
    };

    let (side, num_base_lots) = if num_base_lots_signed >= 0 {
        (Side::Bid, num_base_lots_signed as u64)
    } else {
        (Side::Ask, num_base_lots_signed.unsigned_abs())
    };

    // Load keypair
    let keypair_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME environment variable not set");
        format!("{}/.config/solana/id.json", home)
    });
    println!("Loading keypair from: {}", keypair_path);
    let keypair =
        read_keypair_file(&keypair_path).map_err(|e| format!("Failed to read keypair: {}", e))?;
    let authority = keypair.pubkey();
    println!("Trader authority: {}\n", authority);

    // Initialize PhoenixClient
    println!("Creating PhoenixClient...");
    let client = PhoenixClient::new_from_env().await?;

    // Subscribe to trader state (explicit receiver for Trader updates)
    let (mut trader_rx, _trader_handle) = client
        .subscribe(PhoenixSubscription::Key(SubscriptionKey::trader(
            &authority, 0,
        )))
        .await?;
    println!("Subscribed to trader state");

    // Subscribe to trader margin events
    let (mut margin_rx, _margin_handle) = client
        .subscribe(PhoenixSubscription::trader_margin(authority, 0))
        .await?;
    println!("Subscribed to trader margin events\n");

    // Cached state from receivers
    let mut cached_trader = Trader::new(TraderKey::new(authority));
    let mut trader_initialized = false;
    let mut cached_margin: Option<TraderPortfolioMargin> = None;
    let mut cached_metadata: Option<PhoenixMetadata> = None;
    let mut trader_updates: u32 = 0;
    let mut margin_updates: u32 = 0;

    // Collect updates for 10 seconds to let state settle
    println!("Waiting 10s for state to settle...");
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);

    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                println!(
                    "\nWait complete ({} trader updates, {} margin updates).\n",
                    trader_updates, margin_updates
                );
                break;
            }
            Some(event) = trader_rx.recv() => {
                if let PhoenixClientEvent::TraderUpdate { update, .. } = event {
                    cached_trader.apply_update(&update);
                    trader_initialized = true;
                    trader_updates += 1;

                    println!(
                        "[trader {:>3}] slot={} subaccounts={} collateral={}",
                        trader_updates,
                        update.slot,
                        cached_trader.subaccounts.len(),
                        cached_trader.total_collateral(),
                    );

                    if let Some(sub) = cached_trader.primary_subaccount() {
                        for (sym, pos) in &sub.positions {
                            println!(
                                "  {} position: {} lots @ ${:.2}",
                                sym, pos.base_position_lots, pos.entry_price_usd
                            );
                        }
                    }
                }
            }
            Some(event) = margin_rx.recv() => {
                if let PhoenixClientEvent::MarginUpdate {
                    trigger,
                    margin,
                    metadata,
                    ..
                } = event
                {
                    margin_updates += 1;

                    match &trigger {
                        MarginTrigger::Trader(msg) => {
                            println!("[margin {:>3}] trader trigger (slot={})", margin_updates, msg.slot);
                        }
                        MarginTrigger::Market(msg) => {
                            println!(
                                "[margin {:>3}] market trigger ({} mark={:.4})",
                                margin_updates, msg.symbol, msg.mark_price
                            );
                        }
                    }

                    if let Some(ref m) = margin {
                        print_margin(m);
                    } else {
                        println!("  margin unavailable (waiting for initialization)");
                    }

                    cached_margin = margin;
                    cached_metadata = Some(metadata);
                }
            }
            else => {
                return Err("Subscription channels closed unexpectedly".into());
            }
        }
    }

    // Validate cached state
    if !trader_initialized {
        return Err("No trader state received during wait period".into());
    }
    let margin = cached_margin
        .as_ref()
        .ok_or("No margin data received during wait period")?;
    let metadata = cached_metadata
        .as_ref()
        .ok_or("No metadata received during wait period")?;

    // Dump cached Trader state
    println!("========== CACHED TRADER ==========");
    println!("  authority:    {}", cached_trader.key.authority());
    println!("  pda_index:    {}", cached_trader.key.pda_index);
    println!("  last_slot:    {}", cached_trader.last_slot);
    println!("  subaccounts:  {}", cached_trader.subaccounts.len());
    println!(
        "  total_collateral: {} (Decimal, USD)",
        cached_trader.total_collateral()
    );
    if let Some(sub) = cached_trader.primary_subaccount() {
        println!("  [subaccount 0]");
        println!("    collateral: {} (Decimal, USD)", sub.collateral);
        for (sym, pos) in &sub.positions {
            println!(
                "    {} position: {} base_lots, entry={} ticks ({} USD), vquote={} quote_lots, \
                 unsettled_funding={} quote_lots, accum_funding={} quote_lots",
                sym,
                pos.base_position_lots,
                pos.entry_price_ticks,
                pos.entry_price_usd,
                pos.virtual_quote_position_lots,
                pos.unsettled_funding_quote_lots,
                pos.accumulated_funding_quote_lots,
            );
        }
        for ((sym, seq), order) in &sub.orders {
            println!("    order: {}#{} {:?}", sym, seq, order);
        }
    }

    // Dump TraderPortfolioMargin
    println!("\n========== TRADER PORTFOLIO MARGIN ==========");
    println!(
        "  quote_lot_collateral: {} signed_quote_lots",
        margin.quote_lot_collateral.as_inner()
    );
    println!(
        "  portfolio value:      {} signed_quote_lots",
        margin.portfolio_value().as_inner()
    );
    println!(
        "  effective collateral: {} signed_quote_lots",
        margin.effective_collateral().as_inner()
    );
    println!("  [aggregate margin]");
    println!(
        "    initial_margin:              {} quote_lots",
        margin.margin.initial_margin.as_inner()
    );
    println!(
        "    initial_margin_for_wdraw:    {} quote_lots",
        margin.margin.initial_margin_for_withdrawals.as_inner()
    );
    println!(
        "    maintenance_margin:          {} quote_lots",
        margin.margin.maintenance_margin.as_inner()
    );
    println!(
        "    limit_order_margin:          {} quote_lots",
        margin.margin.limit_order_margin.as_inner()
    );
    println!(
        "    unrealized_pnl:              {} signed_quote_lots",
        margin.margin.unrealized_pnl.as_inner()
    );
    println!(
        "    discounted_unrealized_pnl:   {} signed_quote_lots",
        margin.margin.discounted_unrealized_pnl.as_inner()
    );
    println!(
        "    discounted_pnl_for_wdraw:    {} signed_quote_lots",
        margin.margin.discounted_pnl_for_withdrawals.as_inner()
    );
    println!(
        "    unsettled_funding:           {} signed_quote_lots",
        margin.margin.unsettled_funding.as_inner()
    );
    println!(
        "    accumulated_funding:         {} signed_quote_lots",
        margin.margin.accumulated_funding.as_inner()
    );
    println!(
        "    position_value:              {} signed_quote_lots",
        margin.margin.position_value.as_inner()
    );
    println!(
        "    backstop_requirement:        {} quote_lots",
        margin.margin.backstop_requirement.as_inner()
    );
    println!("  risk tier: {:?}", margin.risk_tier().ok());

    for (sym, mm) in &margin.positions {
        println!("  [{}]", sym);
        if let Some(pos) = &mm.position {
            println!(
                "    base_lot_position:       {} signed_base_lots",
                pos.base_lot_position.as_inner()
            );
            println!(
                "    virtual_quote_position:  {} signed_quote_lots",
                pos.virtual_quote_lot_position.as_inner()
            );
        }
        println!(
            "    initial_margin:          {} quote_lots",
            mm.margin.initial_margin.as_inner()
        );
        println!(
            "    maintenance_margin:      {} quote_lots",
            mm.margin.maintenance_margin.as_inner()
        );
        println!(
            "    limit_order_margin:      {} quote_lots",
            mm.margin.limit_order_margin.as_inner()
        );
        println!(
            "    unrealized_pnl:          {} signed_quote_lots",
            mm.margin.unrealized_pnl.as_inner()
        );
        println!(
            "    position_value:          {} signed_quote_lots",
            mm.margin.position_value.as_inner()
        );
        println!("    limit_orders:            {}", mm.limit_orders.len());
    }
    println!("=======================================\n");

    // Compute transferable collateral
    let transferable = margin.calculate_transferable_collateral()?;
    println!("Transferable collateral: {} quote_lots", transferable);

    // Build isolated market order via PhoenixTxBuilder
    let builder = PhoenixTxBuilder::new(metadata);
    let instructions = builder.build_isolated_market_order(
        &cached_trader,
        &symbol,
        side,
        num_base_lots,
        Some(IsolatedCollateralFlow::TransferFromCrossMargin { collateral }),
        false,
        bracket.as_ref(),
    )?;

    println!(
        "\nSending {} instruction(s): {} {} base lots on {}",
        instructions.len(),
        if side == Side::Bid { "Bid" } else { "Ask" },
        num_base_lots,
        symbol,
    );
    if let Some(ref b) = bracket {
        if let Some(sl) = b.stop_loss.as_ref().map(|leg| leg.price) {
            println!("  Stop-loss: {}", sl);
        }
        if let Some(tp) = b.take_profit.as_ref().map(|leg| leg.price) {
            println!("  Take-profit: {}", tp);
        }
    }

    let rpc =
        RpcClient::new_with_commitment(RPC_ENDPOINT.to_string(), CommitmentConfig::confirmed());
    let blockhash = rpc.get_latest_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &[&keypair],
        blockhash,
    );

    let signature = rpc.send_and_confirm_transaction(&tx).await?;

    println!("Transaction confirmed!");
    println!("Signature: {}", signature);
    println!("Explorer: https://explorer.solana.com/tx/{}", signature);

    client.shutdown();
    client.run().await;

    Ok(())
}
