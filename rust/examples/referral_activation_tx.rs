//! Example: activate a referral with the public activate-tx flow.
//!
//! This flow mirrors a wallet integration:
//!
//! 1. Fetch `/v1/referral/activation-permission`.
//! 2. Build the delegated onboarding/capability transaction.
//! 3. Sign it with the trader authority.
//! 4. Submit it to `/v1/referral/activate-tx`; the API adds the onboarder
//!    signature, submits the transaction, and waits for activation.
//! 5. Wait 5s, then fetch and print the trader account from RPC.
//!
//! The activate-tx endpoint does not create the trader account. Pass
//! `--register-if-missing` to register the trader first with the same keypair.
//!
//! Run with:
//!   PHOENIX_ENV=beta \
//!   cargo run -p phoenix-rise --example referral_activation_tx \
//!     --features solana-keypair -- <REFERRAL_CODE> \
//!     --api-url http://127.0.0.1:8080 \
//!     --rpc-url <RPC_URL> \
//!     --trader-keypair-path ~/.config/solana/id.json \
//!     --register-if-missing

use std::str::FromStr;
use std::time::Duration;
use std::{env, process};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use phoenix_rise::accounts::{Permission, Trader};
use phoenix_rise::phoenix_rise_ix::{
    OnboardTraderDelegatedParams, RegisterTraderParams, TRADER_ONBOARDING_PERMISSION,
    create_onboard_trader_delegated_ix, create_register_trader_ix,
};
use phoenix_rise::{ActivateReferralTxRequest, PhoenixHttpClient, PhoenixMetadata, TraderKey};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction::versioned::VersionedTransaction;

const DEFAULT_API_URL: &str = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_MAX_POSITIONS: u64 = 128;
const DEFAULT_POST_SUBMIT_WAIT_SECONDS: u64 = 5;
const USAGE: &str = r#"Usage:
  cargo run -p phoenix-rise --example referral_activation_tx --features solana-keypair -- <REFERRAL_CODE> [options]

Options:
  --api-url <url>                       Phoenix API URL
  --rpc-url <url>                       Solana RPC URL
  --trader-keypair-path <path>          Trader authority keypair path
  --trader-pda-index <n>                Trader PDA index (default: 0)
  --trader-subaccount-index <n>         Trader subaccount index (default: 0; activate-tx currently supports 0 only)
  --register-if-missing                 Create the trader account first if it is missing
  --max-positions <n>                   Max positions for --register-if-missing (default: 128)
  --post-submit-wait-seconds <n>        Seconds to wait before printing the RPC trader account (default: 5)
  -h, --help                            Show this help

Environment:
  PHOENIX_API_URL
  PHOENIX_RPC_URL / SOLANA_RPC_URL
  TRADER_KEYPAIR_PATH / KEYPAIR_PATH
  PHOENIX_ENV=beta                      Use beta Phoenix instruction addresses
"#;

struct CliArgs {
    referral_code: String,
    api_url: String,
    rpc_url: String,
    trader_keypair_path: String,
    trader_pda_index: u8,
    trader_subaccount_index: u8,
    register_if_missing: bool,
    max_positions: u64,
    post_submit_wait: Duration,
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

fn parse_u64(value: &str, flag: &str) -> u64 {
    value
        .parse::<u64>()
        .unwrap_or_else(|_| fail(&format!("Invalid value for {flag}: {value}")))
}

fn parse_max_positions(value: &str) -> u64 {
    let max_positions = parse_u64(value, "--max-positions");
    if max_positions == 0 || max_positions > DEFAULT_MAX_POSITIONS {
        fail("--max-positions must be between 1 and 128");
    }
    max_positions
}

fn parse_args(argv: Vec<String>) -> CliArgs {
    let mut api_url = env::var("PHOENIX_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());
    let mut rpc_url = env::var("PHOENIX_RPC_URL")
        .or_else(|_| env::var("SOLANA_RPC_URL"))
        .unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
    let mut trader_keypair_path = env::var("TRADER_KEYPAIR_PATH")
        .or_else(|_| env::var("KEYPAIR_PATH"))
        .unwrap_or_else(|_| default_keypair_path());
    let mut trader_pda_index = 0;
    let mut trader_subaccount_index = 0;
    let mut register_if_missing = false;
    let mut max_positions = DEFAULT_MAX_POSITIONS;
    let mut post_submit_wait = Duration::from_secs(DEFAULT_POST_SUBMIT_WAIT_SECONDS);
    let mut referral_code = None::<String>;

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
            "--trader-keypair-path" => {
                trader_keypair_path = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --trader-keypair-path"));
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
            "--register-if-missing" => {
                register_if_missing = true;
            }
            "--max-positions" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --max-positions"));
                max_positions = parse_max_positions(&value);
            }
            "--post-submit-wait-seconds" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --post-submit-wait-seconds"));
                post_submit_wait =
                    Duration::from_secs(parse_u64(&value, "--post-submit-wait-seconds"));
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                process::exit(0);
            }
            value if value.starts_with('-') => fail(&format!("Unknown argument: {value}")),
            value => {
                if referral_code.is_some() {
                    fail(&format!("Unexpected positional argument: {value}"));
                }
                referral_code = Some(value.to_string());
            }
        }
    }

    if trader_subaccount_index != 0 {
        fail("--trader-subaccount-index must be 0 for /v1/referral/activate-tx");
    }

    CliArgs {
        referral_code: referral_code
            .unwrap_or_else(|| fail("Expected exactly one positional argument: <REFERRAL_CODE>")),
        api_url,
        rpc_url,
        trader_keypair_path,
        trader_pda_index,
        trader_subaccount_index,
        register_if_missing,
        max_positions,
        post_submit_wait,
    }
}

async fn fetch_trader(
    rpc: &RpcClient,
    trader_account: &Pubkey,
) -> Result<Option<Trader>, Box<dyn std::error::Error>> {
    let Some(account) = rpc
        .get_account_with_commitment(trader_account, CommitmentConfig::confirmed())
        .await?
        .value
    else {
        return Ok(None);
    };
    Ok(Some(Trader::try_from_account_bytes(&account.data)?))
}

async fn register_trader_if_missing(
    rpc: &RpcClient,
    trader_keypair: &solana_keypair::Keypair,
    trader: &TraderKey,
    max_positions: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    if fetch_trader(rpc, &trader.pda()).await?.is_some() {
        println!("Trader account already exists; skipping registration.");
        return Ok(());
    }

    let register_params = RegisterTraderParams::builder()
        .payer(trader_keypair.pubkey())
        .trader(trader.authority())
        .trader_account(trader.pda())
        .max_positions(max_positions)
        .trader_pda_index(trader.pda_index)
        .subaccount_index(trader.subaccount_index)
        .build()?;
    let instruction: solana_instruction::Instruction =
        create_register_trader_ix(register_params)?.into();
    let blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&trader_keypair.pubkey()),
        &[trader_keypair],
        blockhash,
    );
    let signature = rpc.send_and_confirm_transaction(&tx).await?;
    println!("Registered trader account.");
    println!("Register signature: {signature}");
    Ok(())
}

async fn fetch_permission_account(
    rpc: &RpcClient,
    permission_account: &Pubkey,
) -> Result<Permission, Box<dyn std::error::Error>> {
    let account = rpc
        .get_account_with_commitment(permission_account, CommitmentConfig::confirmed())
        .await?
        .value
        .ok_or_else(|| format!("Permission account does not exist: {permission_account}"))?;
    Ok(Permission::try_from_account_bytes(&account.data)?)
}

fn validate_permission(
    permission: &Permission,
    expected_risk_authority: Pubkey,
    expected_onboarder: Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    if permission.permission_authority != expected_risk_authority {
        return Err(format!(
            "Permission authority mismatch: expected {expected_risk_authority}, got {}",
            permission.permission_authority
        )
        .into());
    }
    if permission.delegated_key != expected_onboarder {
        return Err(format!(
            "Permission delegated key mismatch: expected {expected_onboarder}, got {}",
            permission.delegated_key
        )
        .into());
    }
    if permission.permission & TRADER_ONBOARDING_PERMISSION != TRADER_ONBOARDING_PERMISSION {
        return Err("Permission account is missing trader-onboarding permission".into());
    }
    if permission.allowed_signer_actions == 0 {
        return Err("Permission account has no remaining signer actions".into());
    }
    Ok(())
}

fn parse_pubkey(value: &str, field: &str) -> Result<Pubkey, Box<dyn std::error::Error>> {
    Pubkey::from_str(value).map_err(|error| format!("Invalid {field}: {error}").into())
}

async fn build_signed_activation_transaction_base64(
    trader_keypair: &solana_keypair::Keypair,
    trader: &TraderKey,
    trader_onboarder: Pubkey,
    permission_account: Pubkey,
    global_trader_index: Vec<Pubkey>,
    active_trader_buffer: Vec<Pubkey>,
    rpc: &RpcClient,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let onboard_params = OnboardTraderDelegatedParams::builder()
        .authority(trader_onboarder)
        .permission_account(permission_account)
        .trader_account(trader.pda())
        .global_trader_index(global_trader_index)
        .active_trader_buffer(active_trader_buffer)
        .build()?;
    let instruction: solana_instruction::Instruction =
        create_onboard_trader_delegated_ix(onboard_params)?.into();

    let mut tx = Transaction::new_with_payer(&[instruction], Some(&trader_keypair.pubkey()));
    let recent_blockhash = rpc.get_latest_blockhash().await?;
    tx.try_partial_sign(&[trader_keypair], recent_blockhash)?;
    let versioned = VersionedTransaction::from(tx);
    let bytes = bincode::serialize(&versioned)?;
    Ok((BASE64_STANDARD.encode(bytes), recent_blockhash.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argv: Vec<String> = env::args().collect();
    if argv.len() == 1 {
        println!("{USAGE}");
        return Ok(());
    }

    let args = parse_args(argv);
    let trader_keypair = read_keypair_file(&args.trader_keypair_path)
        .map_err(|error| format!("Failed to read trader keypair: {error}"))?;
    let trader = TraderKey::new_with_idx(
        trader_keypair.pubkey(),
        args.trader_pda_index,
        args.trader_subaccount_index,
    );
    let rpc = RpcClient::new_with_commitment(args.rpc_url.clone(), CommitmentConfig::confirmed());
    let http = PhoenixHttpClient::new_public(args.api_url.clone())?;

    println!("API URL:             {}", args.api_url);
    println!("RPC URL:             {}", args.rpc_url);
    println!("Trader authority:    {}", trader.authority());
    println!("Trader account:      {}", trader.pda());
    println!("Referral code:       {}", args.referral_code);

    if args.register_if_missing {
        register_trader_if_missing(&rpc, &trader_keypair, &trader, args.max_positions).await?;
    } else if fetch_trader(&rpc, &trader.pda()).await?.is_none() {
        return Err(
            "Trader account is missing. Re-run with --register-if-missing or register it first."
                .into(),
        );
    }

    println!("\nFetching referral activation permission...");
    let permission_response = http.invite().get_referral_activation_permission().await?;
    let trader_onboarder = parse_pubkey(&permission_response.trader_onboarder, "trader_onboarder")?;
    let risk_authority = parse_pubkey(&permission_response.risk_authority, "risk_authority")?;
    let permission_account = parse_pubkey(
        &permission_response.permission_account,
        "permission_account",
    )?;
    println!("Trader onboarder:    {trader_onboarder}");
    println!("Risk authority:      {risk_authority}");
    println!("Permission account:  {permission_account}");

    let permission = fetch_permission_account(&rpc, &permission_account).await?;
    validate_permission(&permission, risk_authority, trader_onboarder)?;
    println!("Permission bits:     {}", permission.permission);
    println!("Signer actions left: {}", permission.allowed_signer_actions);

    println!("\nFetching exchange metadata...");
    let exchange = http.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);
    let keys = metadata.keys();
    let global_trader_index = keys
        .global_trader_index
        .iter()
        .map(|value| parse_pubkey(value, "global_trader_index"))
        .collect::<Result<Vec<_>, _>>()?;
    let active_trader_buffer = keys
        .active_trader_buffer
        .iter()
        .map(|value| parse_pubkey(value, "active_trader_buffer"))
        .collect::<Result<Vec<_>, _>>()?;

    let (transaction, recent_blockhash) = build_signed_activation_transaction_base64(
        &trader_keypair,
        &trader,
        trader_onboarder,
        permission_account,
        global_trader_index,
        active_trader_buffer,
        &rpc,
    )
    .await?;

    println!("\nSubmitting /v1/referral/activate-tx...");
    let response = http
        .invite()
        .activate_referral_tx(&ActivateReferralTxRequest {
            referral_code: args.referral_code,
            trader_authority: trader.authority().to_string(),
            trader_pda_index: Some(args.trader_pda_index),
            trader_subaccount_index: Some(args.trader_subaccount_index),
            recent_blockhash,
            transaction,
        })
        .await?;
    println!("Activation status:   {}", response.status);
    println!("Response trader PDA: {}", response.trader_pda);
    if let Some(signature) = response.signature {
        println!("Activation sig:      {signature}");
    }

    println!(
        "\nWaiting {}s before fetching trader account from RPC...",
        args.post_submit_wait.as_secs()
    );
    tokio::time::sleep(args.post_submit_wait).await;

    let trader_account = fetch_trader(&rpc, &trader.pda()).await?.ok_or_else(|| {
        format!(
            "Trader account still missing after activation: {}",
            trader.pda()
        )
    })?;
    println!("\nRPC trader account:");
    println!("{trader_account:#?}");

    Ok(())
}
