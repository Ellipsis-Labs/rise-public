//! Example: fetch and deserialize Phoenix on-chain accounts via Solana RPC.
//!
//! Run with:
//!   PHOENIX_RPC_URL=https://api.mainnet-beta.solana.com \
//!   cargo run -p phoenix-rise --example fetch_on_chain_accounts --features
//! tx-builder
//!
//! Optionally include a trader account:
//!   TRADER_ACCOUNT=<TRADER_ACCOUNT_PUBKEY> cargo run -p phoenix-rise
//! --example fetch_on_chain_accounts --features tx-builder

use std::str::FromStr;

use phoenix_rise::core::{AccountDataFetcher, FetchedAccount, PhoenixAccountClient};
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;

const PHOENIX_RPC_URL_ENV: &str = "PHOENIX_RPC_URL";
const DEFAULT_SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

struct SolanaRpcAccountFetcher {
    rpc: RpcClient,
}

impl SolanaRpcAccountFetcher {
    fn new(rpc_url: String) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url),
        }
    }
}

impl AccountDataFetcher for SolanaRpcAccountFetcher {
    type Error = String;

    fn fetch_account_data(&self, address: &Pubkey) -> Result<Vec<u8>, Self::Error> {
        self.rpc
            .get_account(address)
            .map(|account| account.data)
            .map_err(|error| error.to_string())
    }

    fn fetch_account(&self, address: &Pubkey) -> Result<FetchedAccount, Self::Error> {
        self.rpc
            .get_account(address)
            .map(|account| FetchedAccount::new(account.data, account.owner, account.executable))
            .map_err(|error| error.to_string())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url =
        std::env::var(PHOENIX_RPC_URL_ENV).unwrap_or_else(|_| DEFAULT_SOLANA_RPC_URL.to_string());
    println!("Connecting to Solana RPC: {rpc_url}\n");

    let client = PhoenixAccountClient::new(SolanaRpcAccountFetcher::new(rpc_url));

    println!("=== Global Configuration ===");
    let global_config = client.fetch_global_configuration(None)?;
    println!("  Account:           {}", global_config.account_key);
    println!("  Quote decimals:    {}", global_config.quote_decimals);
    println!("  Exchange status:   {}", global_config.exchange_status);
    println!("  Global vault:      {}", global_config.global_vault_key);
    println!("  Perp asset map:    {}", global_config.perp_asset_map_key);
    println!("  Withdraw queue:    {}", global_config.withdraw_queue_key);
    println!(
        "  Unclaimed fees:    {} quote lots\n",
        global_config.unclaimed_quote_lot_fees
    );

    println!("=== Perp Asset Map ===");
    let perp_asset_map = client.fetch_perp_asset_map(&global_config.perp_asset_map_key)?;
    println!("  Active assets:     {}", perp_asset_map.metadata.len);
    println!("  Capacity:          {}", perp_asset_map.metadata.capacity);

    for (symbol, metadata) in perp_asset_map.iter().take(5) {
        let params = &metadata.static_market_params;
        println!(
            "  {symbol:<8} asset_id={} market={} tick_size={} base_lot_decimals={}",
            params.asset_id, params.market_account, params.tick_size, params.base_lot_decimals
        );
    }

    if let Some((symbol, metadata)) = perp_asset_map.iter().next() {
        println!("\n=== First Market Orderbook Header ({symbol}) ===");
        let market = metadata.static_market_params.market_account;
        let orderbook_header = client.fetch_orderbook_header(&market)?;
        println!("  Market account:    {market}");
        println!("  Asset symbol:      {}", orderbook_header.asset_symbol);
        println!("  Asset id:          {}", orderbook_header.asset_id);
        println!("  Market status:     {}", orderbook_header.market_status);
        println!(
            "  Sequence:          {}",
            orderbook_header.sequence_number.sequence_number
        );
        println!("  Recent trades:     {}", orderbook_header.trade_list.len());
    }

    if let Ok(trader_account) = std::env::var("TRADER_ACCOUNT") {
        println!("\n=== Trader Account ===");
        let trader_pubkey = Pubkey::from_str(&trader_account)?;
        let trader = client.fetch_trader(&trader_pubkey)?;
        println!("  Trader account:    {}", trader.key);
        println!("  Authority:         {}", trader.authority);
        println!("  PDA index:         {}", trader.trader_pda_index);
        println!("  Subaccount index:  {}", trader.trader_subaccount_index);
        println!(
            "  Collateral lots:   {}",
            trader.state.quote_lot_collateral.as_inner()
        );
        println!("  Positions:         {}", trader.positions.len);

        for entry in trader.position_entries().take(5) {
            println!(
                "  asset_id={} base_lots={} virtual_quote_lots={}",
                entry.asset_id,
                entry.position.base_lot_position.as_inner(),
                entry.position.virtual_quote_lot_position.as_inner()
            );
        }
    } else {
        println!("\nSet TRADER_ACCOUNT=<pubkey> to fetch and print a trader account.");
    }

    Ok(())
}
