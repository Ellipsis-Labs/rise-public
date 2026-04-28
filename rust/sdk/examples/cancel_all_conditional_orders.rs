//! Example: cancel every active conditional-order leg for a trader.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example cancel_all_conditional_orders
//! --features solana-keypair -- \     --authority <AUTHORITY_PUBKEY> [options]
//!
//! Options:
//!   --authority <pubkey>                 Trader authority pubkey (required)
//!   --position-authority <pubkey>        Optional delegated signer pubkey
//!   --trader-pda-index <n>               Trader PDA index (default: 0)
//!   --trader-subaccount-index <n>        Trader subaccount index (default: 0)
//!   --isolated                           Set isIsolated=true in API requests
//!   --max-instructions-per-tx <n>        Max cancel ixs per transaction
//! (default: 8)   --rpc-url <url>                      Solana RPC URL
//!   --api-url <url>                      Phoenix API URL
//!   --keypair-path <path>                Signer keypair path
//!
//! Environment fallbacks:
//!   PHOENIX_RPC_URL / SOLANA_RPC_URL
//!   PHOENIX_API_URL
//!   KEYPAIR_PATH

use std::collections::HashMap;
use std::env;
use std::str::FromStr;

use phoenix_rise::accounts::{ConditionalOrderCollection, StopLossDirection};
use phoenix_rise::{
    AccountDataFetcher, CancelConditionalOrderRequest, PhoenixAccountClient, PhoenixHttpClient,
    TraderKey, get_conditional_orders_address,
};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client::rpc_client::RpcClient as BlockingRpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_MAX_INSTRUCTIONS_PER_TX: usize = 8;
const DEFAULT_PHOENIX_API_URL: &str = "https://perp-api.phoenix.trade";

struct Args {
    authority: Pubkey,
    position_authority: Option<Pubkey>,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    is_isolated: bool,
    max_instructions_per_tx: usize,
    rpc_url: String,
    api_url: String,
    keypair_path: String,
}

struct SolanaRpcAccountFetcher {
    rpc: BlockingRpcClient,
}

impl SolanaRpcAccountFetcher {
    fn new(rpc_url: String) -> Self {
        Self {
            rpc: BlockingRpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed()),
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
}

fn usage() -> &'static str {
    "Usage:
  cargo run -p phoenix-rise --example cancel_all_conditional_orders --features solana-keypair -- \
     --authority <AUTHORITY_PUBKEY> [options]

Options:
  --position-authority <pubkey>        Optional delegated signer pubkey
  --trader-pda-index <n>               Trader PDA index (default: 0)
  --trader-subaccount-index <n>        Trader subaccount index (default: 0)
  --isolated                           Set isIsolated=true in API requests
  --max-instructions-per-tx <n>        Max cancel ixs per transaction (default: 8)
  --rpc-url <url>                      Solana RPC URL
  --api-url <url>                      Phoenix API URL
  --keypair-path <path>                Signer keypair path

Environment fallbacks:
  PHOENIX_RPC_URL / SOLANA_RPC_URL
  PHOENIX_API_URL
  KEYPAIR_PATH"
}

fn fail(message: impl AsRef<str>) -> ! {
    eprintln!("{}", message.as_ref());
    eprintln!();
    eprintln!("{}", usage());
    std::process::exit(1);
}

fn parse_pubkey(value: &str, flag: &str) -> Pubkey {
    Pubkey::from_str(value).unwrap_or_else(|error| fail(format!("Invalid {flag}: {error}")))
}

fn parse_u8(value: &str, flag: &str) -> u8 {
    value
        .parse::<u8>()
        .unwrap_or_else(|error| fail(format!("Invalid {flag}: {error}")))
}

fn parse_positive_usize(value: &str, flag: &str) -> usize {
    let parsed = value
        .parse::<usize>()
        .unwrap_or_else(|error| fail(format!("Invalid {flag}: {error}")));
    if parsed == 0 {
        fail(format!("{flag} must be greater than 0"));
    }
    parsed
}

fn default_keypair_path() -> String {
    env::var("KEYPAIR_PATH").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| fail("HOME environment variable not set"));
        format!("{home}/.config/solana/id.json")
    })
}

fn parse_args(argv: &[String]) -> Args {
    let mut authority: Option<Pubkey> = None;
    let mut position_authority: Option<Pubkey> = None;
    let mut trader_pda_index = 0u8;
    let mut trader_subaccount_index = 0u8;
    let mut is_isolated = false;
    let mut max_instructions_per_tx = DEFAULT_MAX_INSTRUCTIONS_PER_TX;
    let mut rpc_url = env::var("PHOENIX_RPC_URL")
        .or_else(|_| env::var("SOLANA_RPC_URL"))
        .unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
    let mut api_url =
        env::var("PHOENIX_API_URL").unwrap_or_else(|_| DEFAULT_PHOENIX_API_URL.to_string());
    let mut keypair_path = default_keypair_path();

    let mut index = 0usize;
    while index < argv.len() {
        match argv[index].as_str() {
            "--authority" => {
                index += 1;
                let value = argv
                    .get(index)
                    .unwrap_or_else(|| fail("Missing value for --authority"));
                authority = Some(parse_pubkey(value, "--authority"));
            }
            "--position-authority" => {
                index += 1;
                let value = argv
                    .get(index)
                    .unwrap_or_else(|| fail("Missing value for --position-authority"));
                position_authority = Some(parse_pubkey(value, "--position-authority"));
            }
            "--trader-pda-index" => {
                index += 1;
                let value = argv
                    .get(index)
                    .unwrap_or_else(|| fail("Missing value for --trader-pda-index"));
                trader_pda_index = parse_u8(value, "--trader-pda-index");
            }
            "--trader-subaccount-index" => {
                index += 1;
                let value = argv
                    .get(index)
                    .unwrap_or_else(|| fail("Missing value for --trader-subaccount-index"));
                trader_subaccount_index = parse_u8(value, "--trader-subaccount-index");
            }
            "--isolated" => {
                is_isolated = true;
            }
            "--max-instructions-per-tx" => {
                index += 1;
                let value = argv
                    .get(index)
                    .unwrap_or_else(|| fail("Missing value for --max-instructions-per-tx"));
                max_instructions_per_tx = parse_positive_usize(value, "--max-instructions-per-tx");
            }
            "--rpc-url" => {
                index += 1;
                rpc_url = argv
                    .get(index)
                    .unwrap_or_else(|| fail("Missing value for --rpc-url"))
                    .clone();
            }
            "--api-url" => {
                index += 1;
                api_url = argv
                    .get(index)
                    .unwrap_or_else(|| fail("Missing value for --api-url"))
                    .clone();
            }
            "--keypair-path" => {
                index += 1;
                keypair_path = argv
                    .get(index)
                    .unwrap_or_else(|| fail("Missing value for --keypair-path"))
                    .clone();
            }
            "-h" | "--help" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            unknown => fail(format!("Unknown argument: {unknown}")),
        }
        index += 1;
    }

    let authority = authority.unwrap_or_else(|| fail("Missing --authority"));

    Args {
        authority,
        position_authority,
        trader_pda_index,
        trader_subaccount_index,
        is_isolated,
        max_instructions_per_tx,
        rpc_url,
        api_url,
        keypair_path,
    }
}

fn execution_direction_to_api_string(direction: StopLossDirection) -> String {
    match direction {
        StopLossDirection::GreaterThan => "greater_than".to_string(),
        StopLossDirection::LessThan => "less_than".to_string(),
    }
}

fn build_http_client(args: &Args) -> PhoenixHttpClient {
    PhoenixHttpClient::new_public(args.api_url.clone())
        .unwrap_or_else(|error| fail(format!("Failed to build HTTP client: {error}")))
}

fn build_asset_id_to_symbol_map(
    perp_asset_map: &phoenix_rise::accounts::PerpAssetMap,
) -> HashMap<u32, String> {
    perp_asset_map
        .iter()
        .map(|(symbol, metadata)| (metadata.static_market_params.asset_id, symbol.clone()))
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args(&env::args().skip(1).collect::<Vec<_>>());

    println!("Loading signer from: {}", args.keypair_path);
    let signer = read_keypair_file(&args.keypair_path)
        .map_err(|error| format!("Failed to read keypair: {error}"))?;

    let expected_signer = args.position_authority.unwrap_or(args.authority);
    if signer.pubkey() != expected_signer {
        return Err(format!(
            "Signer pubkey {} does not match the required instruction signer {}",
            signer.pubkey(),
            expected_signer
        )
        .into());
    }

    let trader = TraderKey::new_with_idx(
        args.authority,
        args.trader_pda_index,
        args.trader_subaccount_index,
    );
    let trader_account = trader.pda();
    let conditional_orders_address = get_conditional_orders_address(&trader_account);

    println!("Authority:                 {}", args.authority);
    println!("Trader account:            {}", trader_account);
    println!("Conditional orders account {}", conditional_orders_address);

    let blocking_rpc =
        BlockingRpcClient::new_with_commitment(args.rpc_url.clone(), CommitmentConfig::confirmed());
    let maybe_account = blocking_rpc
        .get_account_with_commitment(&conditional_orders_address, CommitmentConfig::confirmed())?
        .value;

    let Some(account) = maybe_account else {
        println!("Conditional orders account does not exist; nothing to cancel.");
        return Ok(());
    };

    let conditional_orders = ConditionalOrderCollection::try_from_account_bytes(&account.data)?;
    if conditional_orders.active_order_indices.is_empty() {
        println!("Conditional orders account exists but has no active orders.");
        return Ok(());
    }

    let account_client =
        PhoenixAccountClient::new(SolanaRpcAccountFetcher::new(args.rpc_url.clone()));
    let global_configuration = account_client.fetch_global_configuration(None)?;
    let perp_asset_map =
        account_client.fetch_perp_asset_map(&global_configuration.perp_asset_map_key)?;
    let asset_id_to_symbol = build_asset_id_to_symbol_map(&perp_asset_map);

    let mut requests = Vec::new();
    for (order_index, order) in conditional_orders.active_orders() {
        let symbol = asset_id_to_symbol
            .get(&order.asset_id)
            .ok_or_else(|| format!("Missing market symbol for asset_id {}", order.asset_id))?
            .clone();

        for direction in order.active_trigger_directions() {
            requests.push(CancelConditionalOrderRequest {
                authority: args.authority.to_string(),
                position_authority: args.position_authority.map(|pubkey| pubkey.to_string()),
                trader_pda_index: args.trader_pda_index,
                trader_subaccount_index: Some(args.trader_subaccount_index),
                is_isolated: args.is_isolated,
                symbol: symbol.clone(),
                conditional_order_index: order_index,
                execution_direction: execution_direction_to_api_string(direction),
            });
        }
    }

    println!(
        "Found {} active conditional orders across {} active legs.",
        conditional_orders.active_order_indices.len(),
        requests.len()
    );

    if requests.is_empty() {
        println!("No active conditional-order legs to cancel.");
        return Ok(());
    }

    let http = build_http_client(&args);
    let mut instructions = Vec::with_capacity(requests.len());
    for request in requests {
        let mut built = http.orders().cancel_conditional_order(request).await?;
        instructions.append(&mut built);
    }

    let rpc = RpcClient::new_with_commitment(args.rpc_url.clone(), CommitmentConfig::confirmed());
    let mut signatures = Vec::new();
    for (batch_index, batch) in instructions
        .chunks(args.max_instructions_per_tx)
        .enumerate()
    {
        println!(
            "Submitting batch {}/{} ({} instruction(s))...",
            batch_index + 1,
            instructions.len().div_ceil(args.max_instructions_per_tx),
            batch.len()
        );
        let blockhash = rpc.get_latest_blockhash().await?;
        let transaction = Transaction::new_signed_with_payer(
            batch,
            Some(&signer.pubkey()),
            &[&signer],
            blockhash,
        );
        let signature = rpc.send_and_confirm_transaction(&transaction).await?;
        println!("  Confirmed {}", signature);
        signatures.push(signature);
    }

    println!();
    println!("Cancelled all active conditional-order legs.");
    println!("Signatures:");
    for signature in signatures {
        println!("  {}", signature);
    }

    Ok(())
}
