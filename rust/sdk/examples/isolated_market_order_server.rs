//! Example: Isolated margin market order via server-side HTTP endpoint.
//!
//! Uses `PhoenixHttpClient` to POST to the HTTP API and receive
//! pre-built instructions. No WebSocket connection needed.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example isolated_market_order_server
//! --features solana-keypair -- SOL 10 5000000 [SL_PRICE|x] [TP_PRICE|x]
//!
//! Use "x" in place of a price to skip stop-loss or take-profit.
//! Examples:
//!   isolated_market_order_server SOL 10 5000000              # no bracket legs
//!   isolated_market_order_server SOL 10 5000000 120.5 x      # SL at 120.5
//!   isolated_market_order_server SOL 10 5000000 x 150.0      # TP at 150.0
//!
//! The server-side isolated endpoint has one shared TP/SL order kind. Use the
//! client-side example when you need both default legs in one order: SL = IOC
//! with slippage, TP = limit at the trigger price.

use std::{env, io};

use phoenix_rise::api::PhoenixHttpClient;
use phoenix_rise::core::{BracketLeg, BracketLegOrders, DEFAULT_BRACKET_LEG_SLIPPAGE_BPS};
use phoenix_rise::ix::types::{IsolatedCollateralFlow, Side};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!(
            "Usage: isolated_market_order_server <SYMBOL> <NUM_BASE_LOTS> <COLLATERAL> \
             [SL_PRICE|x] [TP_PRICE|x]"
        );
        eprintln!("  NUM_BASE_LOTS: positive for bid, negative for ask");
        eprintln!("Example: isolated_market_order_server SOL 10 5.0 120.5 150.0");
        std::process::exit(1);
    }

    let symbol = args[1].as_str();
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
    if sl_price.is_some() && tp_price.is_some() {
        return Err(
            "server-side isolated TP/SL cannot encode mixed default order kinds; use \
             isolated_market_order_client for SL IOC plus TP limit"
                .into(),
        );
    }
    let bracket = if sl_price.is_some() || tp_price.is_some() {
        Some(BracketLegOrders {
            stop_loss: sl_price.map(|price| {
                BracketLeg::new(price).with_slippage_bps(DEFAULT_BRACKET_LEG_SLIPPAGE_BPS)
            }),
            take_profit: tp_price.map(|price| BracketLeg::new(price).with_limit_order()),
        })
    } else {
        None
    };

    let (side, num_base_lots) = if num_base_lots_signed >= 0 {
        (Side::Bid, num_base_lots_signed as u64)
    } else {
        (Side::Ask, num_base_lots_signed.unsigned_abs())
    };

    let keypair_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("HOME environment variable not set");
        format!("{}/.config/solana/id.json", home)
    });

    println!("Loading keypair from: {}", keypair_path);
    let keypair =
        read_keypair_file(&keypair_path).map_err(|e| format!("Failed to read keypair: {}", e))?;
    let authority = keypair.pubkey();
    println!("Trader authority: {}", authority);

    let http = PhoenixHttpClient::new_from_env()?;
    let tp_sl = bracket
        .as_ref()
        .map(|bracket| {
            bracket
                .try_to_tp_sl_config_for_side(side)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
        })
        .transpose()?;

    println!("Building instructions via server...");
    let instructions = http
        .build_isolated_market_order_tx(
            &authority,
            symbol,
            side,
            num_base_lots,
            Some(IsolatedCollateralFlow::TransferFromCrossMargin { collateral }),
            false,
            tp_sl,
        )
        .await?;

    println!(
        "\nSending {} instruction(s): {} {} base lots on {}",
        instructions.len(),
        if side == Side::Bid { "Bid" } else { "Ask" },
        num_base_lots,
        symbol,
    );
    if let Some(ref b) = bracket {
        if let Some(sl) = b.stop_loss.as_ref().map(|leg| leg.price) {
            println!(
                "  Stop-loss: {} (IOC, {} bps slippage)",
                sl, DEFAULT_BRACKET_LEG_SLIPPAGE_BPS
            );
        }
        if let Some(tp) = b.take_profit.as_ref().map(|leg| leg.price) {
            println!("  Take-profit: {} (limit at trigger price)", tp);
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

    Ok(())
}
