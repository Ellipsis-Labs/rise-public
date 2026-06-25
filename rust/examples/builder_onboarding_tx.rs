//! Example: register and onboard a trader without a referral code.
//!
//! This uses `POST /v1/exchange/build-register-ixs` to fetch the
//! register/onboard instructions, signs the user-controlled signer slots, then
//! submits the transaction to `POST /v1/exchange/send-register-ixs`. The API
//! validates, signs with the configured Phoenix onboarding keypair, simulates,
//! and sends it.
//!
//! Run with:
//! ```text
//!   cargo run -p phoenix-rise --example builder_onboarding_tx \
//!     --features solana-keypair -- \
//!     --trader-keypair-path ~/.config/solana/id.json
//! ```
//!
//! Options:
//!   --api-url <url>                  Phoenix API URL
//!   --rpc-url <url>                  Solana RPC URL
//!   --trader-keypair-path <path>     Trader authority keypair path
//!   --fee-payer-keypair-path <path>  Fee-payer keypair path
//!   --max-positions <n>              Max positions when registering
//!   --recent-blockhash <blockhash>   Optional blockhash for the transaction
//!
//! Environment:
//!   PHOENIX_API_URL
//!   PHOENIX_RPC_URL / SOLANA_RPC_URL
//!   TRADER_KEYPAIR_PATH / KEYPAIR_PATH
//!   FEE_PAYER_KEYPAIR_PATH

use std::str::FromStr;
use std::{env, process};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use phoenix_rise::{
    ApiInstructionResponse, BuildRegisterIxsRequest, PhoenixHttpClient, SendRegisterIxsRequest,
};
use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::read_keypair_file;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction::versioned::VersionedTransaction;

const DEFAULT_API_URL: &str = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_MAX_POSITIONS: u32 = 128;
const USAGE: &str = r#"Usage:
  cargo run -p phoenix-rise --example builder_onboarding_tx --features solana-keypair -- [options]

Options:
  --api-url <url>                  Phoenix API URL
  --rpc-url <url>                  Solana RPC URL
  --trader-keypair-path <path>     Trader authority keypair path
  --fee-payer-keypair-path <path>  Fee-payer keypair path (default: trader keypair)
  --max-positions <n>              Max positions when registering (32-128, default: 128)
  --recent-blockhash <blockhash>   Optional blockhash to use in the transaction
  -h, --help                       Show this help

Environment:
  PHOENIX_API_URL
  PHOENIX_RPC_URL / SOLANA_RPC_URL
  TRADER_KEYPAIR_PATH / KEYPAIR_PATH
  FEE_PAYER_KEYPAIR_PATH
"#;

struct CliArgs {
    api_url: String,
    rpc_url: String,
    trader_keypair_path: String,
    fee_payer_keypair_path: Option<String>,
    max_positions: u32,
    recent_blockhash: Option<String>,
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

fn parse_max_positions(value: &str) -> u32 {
    let max_positions = value
        .parse::<u32>()
        .unwrap_or_else(|_| fail(&format!("Invalid value for --max-positions: {value}")));
    if !(32..=DEFAULT_MAX_POSITIONS).contains(&max_positions) {
        fail("--max-positions must be between 32 and 128");
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
    let mut fee_payer_keypair_path = env::var("FEE_PAYER_KEYPAIR_PATH").ok();
    let mut max_positions = DEFAULT_MAX_POSITIONS;
    let mut recent_blockhash = None::<String>;

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
            "--fee-payer-keypair-path" => {
                fee_payer_keypair_path = Some(
                    args.next()
                        .unwrap_or_else(|| fail("Missing value for --fee-payer-keypair-path")),
                );
            }
            "--max-positions" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --max-positions"));
                max_positions = parse_max_positions(&value);
            }
            "--recent-blockhash" => {
                recent_blockhash = Some(
                    args.next()
                        .unwrap_or_else(|| fail("Missing value for --recent-blockhash")),
                );
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                process::exit(0);
            }
            value => fail(&format!("Unknown argument: {value}")),
        }
    }

    CliArgs {
        api_url,
        rpc_url,
        trader_keypair_path,
        fee_payer_keypair_path,
        max_positions,
        recent_blockhash,
    }
}

fn encode_transaction(
    transaction: &VersionedTransaction,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(BASE64_STANDARD.encode(bincode::serialize(transaction)?))
}

fn to_instruction(
    api_instruction: &ApiInstructionResponse,
) -> Result<Instruction, Box<dyn std::error::Error>> {
    Ok(Instruction {
        program_id: Pubkey::from_str(&api_instruction.program_id)?,
        accounts: api_instruction
            .keys
            .iter()
            .map(|account| {
                Ok(AccountMeta {
                    pubkey: Pubkey::from_str(&account.pubkey)?,
                    is_signer: account.is_signer,
                    is_writable: account.is_writable,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?,
        data: api_instruction.data.clone(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argv: Vec<String> = env::args().collect();
    if argv.len() == 1 {
        println!("This example submits a live builder onboarding transaction.");
        println!(
            "Pass --trader-keypair-path, or set TRADER_KEYPAIR_PATH, to register and onboard that \
             trader without a referral code.\n"
        );
        println!("{USAGE}");
        return Ok(());
    }

    let args = parse_args(argv);
    let trader_keypair = read_keypair_file(&args.trader_keypair_path)
        .map_err(|error| format!("Failed to read trader keypair: {error}"))?;
    let fee_payer_keypair = args
        .fee_payer_keypair_path
        .as_deref()
        .map(read_keypair_file)
        .transpose()
        .map_err(|error| format!("Failed to read fee-payer keypair: {error}"))?;
    let trader_authority = trader_keypair.pubkey();
    let tx_fee_payer = fee_payer_keypair
        .as_ref()
        .map(Signer::pubkey)
        .unwrap_or(trader_authority);

    println!("API URL:           {}", args.api_url);
    println!("RPC URL:           {}", args.rpc_url);
    println!("Trader authority:  {trader_authority}");
    println!("Transaction payer: {tx_fee_payer}");

    let http = PhoenixHttpClient::new_public(args.api_url)?;
    let response = http
        .exchange()
        .build_register_ixs(&BuildRegisterIxsRequest {
            trader_authority: trader_authority.to_string(),
            tx_fee_payer: tx_fee_payer.to_string(),
            max_positions: Some(args.max_positions),
        })
        .await?;
    let instructions = response
        .instructions
        .iter()
        .map(to_instruction)
        .collect::<Result<Vec<_>, _>>()?;
    let rpc = RpcClient::new_with_commitment(args.rpc_url, CommitmentConfig::confirmed());
    let recent_blockhash = match args.recent_blockhash.as_deref() {
        Some(blockhash) => blockhash.parse()?,
        None => rpc.get_latest_blockhash().await?,
    };
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&tx_fee_payer));
    if let Some(fee_payer_keypair) = fee_payer_keypair.as_ref() {
        if fee_payer_keypair.pubkey() != trader_authority {
            transaction
                .try_partial_sign(&[&trader_keypair, fee_payer_keypair], recent_blockhash)?;
        } else {
            transaction.try_partial_sign(&[&trader_keypair], recent_blockhash)?;
        }
    } else {
        transaction.try_partial_sign(&[&trader_keypair], recent_blockhash)?;
    }

    let transaction = VersionedTransaction::from(transaction);
    let signed_transaction = encode_transaction(&transaction)?;
    println!("Trader PDA:        {}", response.trader_pda);
    println!("Trader onboarder:  {}", response.trader_onboarder);
    println!("Recent blockhash:  {recent_blockhash}");
    println!("Max positions:     {}", response.max_positions);
    println!("Includes register: {}", response.include_register_trader);
    println!("Signed tx base64:  {signed_transaction}");

    let submitted = http
        .exchange()
        .send_register_ixs(&SendRegisterIxsRequest {
            transaction: signed_transaction,
            trader_authority: trader_authority.to_string(),
            tx_fee_payer: tx_fee_payer.to_string(),
            max_positions: Some(args.max_positions),
            trader_pda_index: Some(0),
            trader_subaccount_index: Some(0),
        })
        .await?;
    println!("\nTransaction submitted by API.");
    println!("Signature: {}", submitted.signature);
    println!(
        "Explorer:  https://explorer.solana.com/tx/{}",
        submitted.signature
    );

    Ok(())
}
