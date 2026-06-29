//! Example: onboard a trader with a delegated trader-onboarder keypair.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example onboard_trader_delegated \
//!     --features tx-builder,solana-keypair -- <TRADER_AUTHORITY> [options]
//!
//! Options:
//!   --api-url <url>                  Phoenix API URL
//!   --rpc-url <url>                  Solana RPC URL
//!   --onboarder-keypair-path <path>  Trader-onboarder keypair path
//!   --trader-pda-index <n>           Trader PDA index (default: 0)
//!   --trader-subaccount-index <n>    Trader subaccount index (default: 0)
//!   --max-positions <n>              Max positions if registering first
//!                                    (default: 128 for subaccount 0, 1
//! otherwise)
//!
//! Environment:
//!   PHOENIX_API_URL
//!   PHOENIX_RPC_URL / SOLANA_RPC_URL
//!   TRADER_ONBOARDER_KEYPAIR_PATH / KEYPAIR_PATH
//!   PHOENIX_ENV=beta                Use beta Phoenix instruction addresses
//!   PHOENIX_DRY_RUN=1               Simulate instead of submitting

use std::str::FromStr;
use std::{env, process};

use phoenix_rise::accounts::owned::Permission;
use phoenix_rise::api::PhoenixHttpClient;
use phoenix_rise::core::{PhoenixMetadata, TraderKey};
use phoenix_rise::ix::constants::get_permission_address;
use phoenix_rise::ix::onboard_trader_delegated::{
    OnboardTraderDelegatedParams, create_onboard_trader_delegated_ix,
};
use phoenix_rise::ix::register_trader::{RegisterTraderParams, create_register_trader_ix};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DEFAULT_API_URL: &str = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const USAGE: &str = r#"Usage:
  cargo run -p phoenix-rise --example onboard_trader_delegated --features tx-builder,solana-keypair -- <TRADER_AUTHORITY> [options]

Options:
  --api-url <url>                  Phoenix API URL
  --rpc-url <url>                  Solana RPC URL
  --onboarder-keypair-path <path>  Trader-onboarder keypair path
  --trader-pda-index <n>           Trader PDA index (default: 0)
  --trader-subaccount-index <n>    Trader subaccount index (default: 0)
  --max-positions <n>              Max positions if registering first (default: 128 for subaccount 0, 1 otherwise)
  -h, --help                       Show this help
"#;

struct CliArgs {
    api_url: String,
    rpc_url: String,
    onboarder_keypair_path: String,
    trader_authority: Pubkey,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    max_positions: u64,
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    eprintln!();
    eprintln!("{USAGE}");
    process::exit(1);
}

fn default_keypair_path() -> String {
    let home = env::var("HOME").expect("HOME environment variable not set");
    format!("{home}/.config/solana/id.json")
}

fn parse_u8(value: &str, flag: &str) -> u8 {
    value
        .parse::<u8>()
        .unwrap_or_else(|_| fail(&format!("Invalid value for {flag}: {value}")))
}

fn parse_max_positions(value: &str) -> u64 {
    let max_positions = value
        .parse::<u64>()
        .unwrap_or_else(|_| fail(&format!("Invalid value for --max-positions: {value}")));
    if max_positions == 0 || max_positions > 128 {
        fail("--max-positions must be between 1 and 128");
    }
    max_positions
}

fn parse_args(argv: Vec<String>) -> CliArgs {
    let mut api_url = env::var("PHOENIX_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    let mut rpc_url = env::var("PHOENIX_RPC_URL")
        .or_else(|_| env::var("SOLANA_RPC_URL"))
        .unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
    let mut onboarder_keypair_path = env::var("TRADER_ONBOARDER_KEYPAIR_PATH")
        .or_else(|_| env::var("KEYPAIR_PATH"))
        .unwrap_or_else(|_| default_keypair_path());
    let mut trader_pda_index = 0;
    let mut trader_subaccount_index = 0;
    let mut max_positions = None::<u64>;
    let mut trader_authority = None::<String>;

    let mut args = argv.into_iter().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--api-url" => {
                api_url = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --api-url"));
            }
            "--rpc-url" => {
                rpc_url = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --rpc-url"));
            }
            "--onboarder-keypair-path" => {
                onboarder_keypair_path = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --onboarder-keypair-path"));
            }
            "--trader-pda-index" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --trader-pda-index"));
                trader_pda_index = parse_u8(&value, "--trader-pda-index");
            }
            "--trader-subaccount-index" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --trader-subaccount-index"));
                trader_subaccount_index = parse_u8(&value, "--trader-subaccount-index");
            }
            "--max-positions" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --max-positions"));
                max_positions = Some(parse_max_positions(&value));
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                process::exit(0);
            }
            value if value.starts_with('-') => fail(&format!("Unknown argument: {value}")),
            value => {
                if trader_authority.is_some() {
                    fail(&format!("Unexpected positional argument: {value}"));
                }
                trader_authority = Some(value.to_string());
            }
        }
    }

    let trader_authority = trader_authority
        .unwrap_or_else(|| fail("Expected exactly one positional argument: <TRADER_AUTHORITY>"));
    let trader_authority =
        Pubkey::from_str(&trader_authority).unwrap_or_else(|error| fail(&error.to_string()));

    let max_positions = max_positions.unwrap_or(if trader_subaccount_index == 0 { 128 } else { 1 });
    if trader_subaccount_index > 0 && max_positions != 1 {
        fail("--max-positions must be 1 when --trader-subaccount-index is greater than 0");
    }

    CliArgs {
        api_url,
        rpc_url,
        onboarder_keypair_path,
        trader_authority,
        trader_pda_index,
        trader_subaccount_index,
        max_positions,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argv: Vec<String> = env::args().collect();
    if argv.len() == 1 {
        println!("This example submits a live trader-onboarding transaction.");
        println!(
            "Pass a target trader authority and an onboarder keypair with trader-onboarding \
             delegation permission.\n"
        );
        println!("{USAGE}");
        return Ok(());
    }

    let args = parse_args(argv);
    let onboarder_keypair = read_keypair_file(&args.onboarder_keypair_path)
        .map_err(|error| format!("Failed to read onboarder keypair: {error}"))?;
    let onboarder_authority = onboarder_keypair.pubkey();
    let trader = TraderKey::new_with_idx(
        args.trader_authority,
        args.trader_pda_index,
        args.trader_subaccount_index,
    );

    println!("Fetching exchange metadata...");
    let http = PhoenixHttpClient::new_public(args.api_url)?;
    let exchange = http.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);
    let keys = metadata.keys();

    let risk_authority = Pubkey::from_str(&keys.current_authorities.risk_authority)?;
    let permission_account = get_permission_address(&risk_authority, &onboarder_authority)?;
    let global_trader_index = keys
        .global_trader_index
        .iter()
        .map(|value| Pubkey::from_str(value))
        .collect::<Result<Vec<_>, _>>()?;
    let active_trader_buffer = keys
        .active_trader_buffer
        .iter()
        .map(|value| Pubkey::from_str(value))
        .collect::<Result<Vec<_>, _>>()?;

    let rpc = RpcClient::new_with_commitment(args.rpc_url.clone(), CommitmentConfig::confirmed());
    let permission_account_data = rpc
        .get_account_with_commitment(&permission_account, CommitmentConfig::confirmed())
        .await?
        .value
        .ok_or_else(|| format!("Permission account does not exist: {permission_account}"))?;
    let permission = Permission::try_from_account_bytes(&permission_account_data.data)?;
    if permission.permission_authority != risk_authority {
        return Err(format!(
            "Permission authority mismatch: expected {risk_authority}, got {}",
            permission.permission_authority
        )
        .into());
    }
    if permission.delegated_key != onboarder_authority {
        return Err(format!(
            "Permission delegated key mismatch: expected {onboarder_authority}, got {}",
            permission.delegated_key
        )
        .into());
    }

    let trader_account = trader.pda();
    let trader_already_registered = rpc
        .get_account_with_commitment(&trader_account, CommitmentConfig::confirmed())
        .await?
        .value
        .is_some();

    let mut instructions = Vec::<solana_instruction::Instruction>::new();
    if !trader_already_registered {
        let register_params = RegisterTraderParams::builder()
            .payer(onboarder_authority)
            .trader(trader.authority())
            .trader_account(trader_account)
            .max_positions(args.max_positions)
            .trader_pda_index(args.trader_pda_index)
            .subaccount_index(args.trader_subaccount_index)
            .build()?;
        instructions.push(create_register_trader_ix(register_params)?.into());
    }

    let onboard_params = OnboardTraderDelegatedParams::builder()
        .authority(onboarder_authority)
        .permission_account(permission_account)
        .trader_account(trader_account)
        .global_trader_index(global_trader_index)
        .active_trader_buffer(active_trader_buffer)
        .build()?;
    instructions.push(create_onboard_trader_delegated_ix(onboard_params)?.into());

    println!("Onboarder authority:     {onboarder_authority}");
    println!("Trader authority:        {}", trader.authority());
    println!("Trader account:          {trader_account}");
    println!("Risk authority:          {risk_authority}");
    println!("Permission account:      {permission_account}");
    println!("Trader already exists:   {trader_already_registered}");
    println!("Permission bits:         {}", permission.permission);
    println!(
        "Allowed signer actions:  {}",
        permission.allowed_signer_actions
    );
    println!("Instruction count:       {}", instructions.len());

    let blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&onboarder_authority),
        &[&onboarder_keypair],
        blockhash,
    );

    if env::var_os("PHOENIX_DRY_RUN").is_some() {
        println!("\nDry run: simulating transaction without sending...");
        let simulation = rpc.simulate_transaction(&tx).await?;
        println!("Simulation result: {:?}", simulation.value.err);
        if let Some(logs) = simulation.value.logs {
            for log in logs {
                println!("{log}");
            }
        }
        return Ok(());
    }

    let signature = rpc.send_and_confirm_transaction(&tx).await?;
    println!("\nTransaction confirmed!");
    println!("Signature: {signature}");
    println!("Explorer:  https://explorer.solana.com/tx/{signature}");

    Ok(())
}
