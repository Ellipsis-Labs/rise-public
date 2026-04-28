//! Example: Isolated margin limit order.
//!
//! Two modes:
//!   **Client-side** (default) — uses `PhoenixTxBuilder` with local state
//!     from WebSocket to construct instructions.
//!   **Server-side** (`--async`) — POSTs to the HTTP API and receives
//!     pre-built instructions, no WebSocket connection needed.
//!
//! Run with:
//!   export PHOENIX_WS_URL=wss://perp-api.phoenix.trade/ws
//!   cargo run -p phoenix-rise --example isolated_limit_order --features
//! solana-keypair -- SOL 10 150.50 5000000 [--async]

use std::env;

use phoenix_rise::{
    IsolatedCollateralFlow, PhoenixHttpClient, PhoenixMetadata, PhoenixTxBuilder, PhoenixWSClient,
    Side, Trader, TraderKey,
};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::Instruction;
use solana_keypair::read_keypair_file;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const RPC_ENDPOINT: &str = "https://api.mainnet-beta.solana.com";

/// Build isolated limit order instructions via the server-side HTTP endpoint.
/// No WebSocket or local state required.
async fn build_via_server(
    http: &PhoenixHttpClient,
    authority: &solana_pubkey::Pubkey,
    symbol: &str,
    side: Side,
    price: f64,
    num_base_lots: u64,
    collateral: u64,
) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
    let ixs = http
        .build_isolated_limit_order_tx(
            authority,
            symbol,
            side,
            price,
            num_base_lots,
            Some(IsolatedCollateralFlow::TransferFromCrossMargin { collateral }),
            false,
        )
        .await?;

    Ok(ixs)
}

/// Build isolated limit order instructions locally using on-chain state
/// fetched via WebSocket.
async fn build_via_client(
    http: &PhoenixHttpClient,
    authority: &solana_pubkey::Pubkey,
    symbol: &str,
    side: Side,
    price: f64,
    num_base_lots: u64,
    collateral: u64,
) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
    let exchange = http.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);
    let builder = PhoenixTxBuilder::new(&metadata);
    let key = TraderKey::new(*authority);

    println!("Connecting to WebSocket for trader state...");
    let ws = PhoenixWSClient::new_from_env()?;
    let (mut rx, _handle) = ws.subscribe_to_trader_state(&key.authority())?;

    let mut trader = Trader::new(key);
    let msg = rx
        .recv()
        .await
        .ok_or("WebSocket closed before receiving trader state")?;
    trader.apply_update(&msg);
    println!(
        "Loaded trader state: {} subaccounts, collateral {}",
        trader.subaccounts.len(),
        trader.total_collateral()
    );

    let ixs = builder.build_isolated_limit_order(
        &trader,
        symbol,
        side,
        price,
        num_base_lots,
        Some(IsolatedCollateralFlow::TransferFromCrossMargin { collateral }),
        false,
    )?;

    Ok(ixs)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let use_async = args.iter().any(|a| a == "--async");
    let args: Vec<&String> = args.iter().filter(|a| *a != "--async").collect();

    if args.len() < 5 {
        eprintln!(
            "Usage: isolated_limit_order <SYMBOL> <NUM_BASE_LOTS> <PRICE> <COLLATERAL> [--async]"
        );
        eprintln!("  NUM_BASE_LOTS: positive for bid, negative for ask");
        eprintln!("  PRICE: limit price in USD (e.g. 150.50)");
        eprintln!("  --async: use server-side instruction building (no WS needed)");
        eprintln!("Example: isolated_limit_order SOL 10 150.50 5.0");
        std::process::exit(1);
    }
    let symbol = args[1].as_str();
    let num_base_lots_signed: i64 = args[2].parse().expect("NUM_BASE_LOTS must be an integer");
    let price: f64 = args[3].parse().expect("PRICE must be a number");
    let collateral: u64 = args[4]
        .parse()
        .expect("COLLATERAL must be quote lots (u64)");

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

    println!("Trader authority: {}", keypair.pubkey());
    println!("\nFetching exchange metadata...");
    let http = PhoenixHttpClient::new_from_env()?;

    let authority = keypair.pubkey();
    let instructions = if use_async {
        println!("Building instructions via server (--async)...");
        build_via_server(
            &http,
            &authority,
            symbol,
            side,
            price,
            num_base_lots,
            collateral,
        )
        .await?
    } else {
        build_via_client(
            &http,
            &authority,
            symbol,
            side,
            price,
            num_base_lots,
            collateral,
        )
        .await?
    };

    println!(
        "\nSending {} instruction(s): {} {} base lots @ ${} on {}",
        instructions.len(),
        if side == Side::Bid { "Bid" } else { "Ask" },
        num_base_lots,
        price,
        symbol,
    );

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
