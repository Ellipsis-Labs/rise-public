//! Example: register and onboard a trader without a referral code.
//!
//! This uses `POST /v1/exchange/build-register-ixs` to fetch the
//! register/onboard instructions, signs with the transaction fee payer, then
//! submits the transaction to `POST /v1/exchange/send-register-ixs`. The API
//! validates, signs with the configured Phoenix onboarding keypair, simulates,
//! and sends it. The trader authority is only a public key and does not sign.
//!
//! Run with:
//! ```text
//!   cargo run -p phoenix-rise --example builder_onboarding_tx \
//!     --features api -- \
//!     [--fee-payer-keypair-path <PATH>] \
//!     [--trader-authority <TRADER_AUTHORITY_PUBKEY>]
//! ```
//!
//! Options:
//!   --api-url <url>                  Phoenix API URL
//!   --rpc-url <url>                  Solana RPC URL
//!   --fee-payer-keypair-path <path>  Fee-payer keypair path
//!                                    (default: ~/.config/solana/id.json)
//!   --trader-authority <pubkey>      Trader authority (default: fee payer)
//!   --max-positions <n>              Max positions when registering
//!   --recent-blockhash <blockhash>   Optional blockhash for the transaction
//!
//! Environment:
//!   PHOENIX_API_URL
//!   PHOENIX_RPC_URL / SOLANA_RPC_URL
//!   FEE_PAYER_KEYPAIR_PATH
//!   TRADER_AUTHORITY

use std::str::FromStr;
use std::{env, process};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use phoenix_rise::api::{
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
  cargo run -p phoenix-rise --example builder_onboarding_tx --features api -- [options]

Options:
  --api-url <url>                  Phoenix API URL
  --rpc-url <url>                  Solana RPC URL
  --fee-payer-keypair-path <path>  Fee-payer keypair path (default: ~/.config/solana/id.json)
  --trader-authority <pubkey>       Trader authority public key (default: fee payer)
  --max-positions <n>              Max positions when registering (32-128, default: 128)
  --recent-blockhash <blockhash>   Optional blockhash to use in the transaction
  -h, --help                       Show this help

Environment:
  PHOENIX_API_URL
  PHOENIX_RPC_URL / SOLANA_RPC_URL
  FEE_PAYER_KEYPAIR_PATH
  TRADER_AUTHORITY
"#;

struct CliArgs {
    api_url: String,
    rpc_url: String,
    trader_authority: Option<String>,
    fee_payer_keypair_path: String,
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
    let mut trader_authority = env::var("TRADER_AUTHORITY").ok();
    let mut fee_payer_keypair_path =
        env::var("FEE_PAYER_KEYPAIR_PATH").unwrap_or_else(|_| default_keypair_path());
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
            "--trader-authority" => {
                trader_authority = Some(
                    args.next()
                        .unwrap_or_else(|| fail("Missing value for --trader-authority")),
                );
            }
            "--fee-payer-keypair-path" => {
                fee_payer_keypair_path = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --fee-payer-keypair-path"));
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
        trader_authority,
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
            "The fee payer is the only local signer. Its keypair path defaults to \
             ~/.config/solana/id.json and can be overridden with --fee-payer-keypair-path or \
             FEE_PAYER_KEYPAIR_PATH; --trader-authority is optional and defaults to the fee \
             payer's public key.\n"
        );
        println!("{USAGE}");
        return Ok(());
    }

    let args = parse_args(argv);
    let fee_payer_keypair = read_keypair_file(&args.fee_payer_keypair_path)
        .map_err(|error| format!("Failed to read fee-payer keypair: {error}"))?;
    let tx_fee_payer = fee_payer_keypair.pubkey();
    let trader_authority = match args.trader_authority.as_deref() {
        Some(authority) => Pubkey::from_str(authority)
            .map_err(|error| format!("Invalid trader authority: {error}"))?,
        None => tx_fee_payer,
    };

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
    transaction.try_partial_sign(&[&fee_payer_keypair], recent_blockhash)?;

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
