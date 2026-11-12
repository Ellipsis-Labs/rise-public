//! Load authoritative trader state, positions, and collateral at one RPC slot.
//!
//! PHOENIX_RPC_URL=<rpc-url> cargo run -p phoenix-rise --example
//! fetch_spline_state --features tx-builder -- <TRADER_ACCOUNT_PUBKEY>
//! Set PHOENIX_ENV=beta to use the beta deployment.

use std::str::FromStr;

use phoenix_rise::core::PhoenixAccountClient;
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let trader = Pubkey::from_str(
        &std::env::args()
            .nth(1)
            .ok_or("usage: fetch_spline_state <TRADER_ACCOUNT_PUBKEY>")?,
    )?;
    let rpc = RpcClient::new_with_commitment(
        std::env::var("PHOENIX_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_owned()),
        CommitmentConfig::confirmed(),
    );
    let client = PhoenixAccountClient::new(rpc);
    let snapshot = client.fetch_trader_snapshot(&trader)?;

    println!("Snapshot slot: {}", snapshot.context_slot());
    println!(
        "Trader: {}; active index: {:?}",
        snapshot.address(),
        snapshot.global_trader_index_node()
    );
    println!("Selected state: {:?}", snapshot.state());
    println!(
        "Quote-lot collateral: {}",
        snapshot.state().quote_lot_collateral.as_inner()
    );
    for position in snapshot.positions() {
        println!("Position: {position:?}");
    }
    for collateral in snapshot.spot_collaterals() {
        println!("Spot collateral: {collateral:?}");
    }
    println!("Locked deposit: {:?}", snapshot.locked_deposit());

    // The loader selects GTI/ATB for hot traders and the trader account for
    // cold traders. Margin calculations also require market metadata, prices,
    // and funding. Retry the complete load if arena discovery changes.
    Ok(())
}
