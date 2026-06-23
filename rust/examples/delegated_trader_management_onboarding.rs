//! Example: grant a delegated onboarding budget, then onboard traders with it.
//!
//! This demonstrates the off-chain integrator flow:
//!
//! 1. An external team gives Phoenix a `user_key`.
//! 2. Phoenix, or a signer with trader-management delegation from the risk
//!    authority, grants `risk_authority -> user_key` trader-onboarding
//!    permission with N signer actions.
//! 3. The holder of `user_key` signs `RegisterTrader` plus
//!    `SetTraderCapabilitiesDelegated` to onboard a trader.
//!
//! Run the user-side onboarding flow:
//!   cargo run -p phoenix-rise --example delegated_trader_management_onboarding
//! --features solana-keypair -- <TRADER_AUTHORITY> --user-keypair-path
//! ./user-keypair.json
//!
//! Run the optional Phoenix-side grant first:
//!   cargo run -p phoenix-rise --example delegated_trader_management_onboarding
//! --features solana-keypair -- <TRADER_AUTHORITY> --user-keypair-path
//! ./user-keypair.json --grant-manager-keypair-path
//! ./trader-management-keypair.json --grant-uses 25

use std::str::FromStr;
use std::{env, process};

use phoenix_rise::accounts::Permission;
use phoenix_rise::phoenix_rise_ix::{
    CreatePermissionParams, OnboardTraderDelegatedParams, RegisterTraderParams,
    SetPermissionDelegatedParams, TRADER_MANAGEMENT_PERMISSION, TRADER_ONBOARDING_PERMISSION,
    create_create_permission_ix, create_onboard_trader_delegated_ix, create_register_trader_ix,
    create_set_permission_delegated_ix,
};
use phoenix_rise::{PhoenixHttpClient, PhoenixMetadata, TraderKey, get_permission_address};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::{Keypair, read_keypair_file};
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DEFAULT_API_URL: &str = "https://perp-api.phoenix.trade";
const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_MAX_POSITIONS: u64 = 128;
const USAGE: &str = r#"Usage:
  cargo run -p phoenix-rise --example delegated_trader_management_onboarding --features solana-keypair -- <TRADER_AUTHORITY> [options]

Options:
  --api-url <url>                       Phoenix API URL
  --rpc-url <url>                       Solana RPC URL
  --user-keypair-path <path>            Delegated user keypair that receives the onboarding budget
  --grant-manager-keypair-path <path>   Optional signer with risk authority or trader-management delegation
  --grant-uses <n>                      Number of onboarding uses to grant when --grant-manager-keypair-path is present (default: 1)
  --trader-pda-index <n>                Trader PDA index (default: 0)
  --trader-subaccount-index <n>         Trader subaccount index (default: 0)
  --max-positions <n>                   Max positions if registering first (default: 128 for subaccount 0, 1 otherwise)
  -h, --help                            Show this help

Environment:
  PHOENIX_API_URL
  PHOENIX_RPC_URL / SOLANA_RPC_URL
  DELEGATED_USER_KEYPAIR_PATH / KEYPAIR_PATH
  TRADER_MANAGEMENT_KEYPAIR_PATH
  PHOENIX_ENV=beta
  PHOENIX_DRY_RUN=1
"#;

struct CliArgs {
    api_url: String,
    rpc_url: String,
    user_keypair_path: String,
    grant_manager_keypair_path: Option<String>,
    grant_uses: i64,
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

fn parse_positive_i64(value: &str, flag: &str) -> i64 {
    let parsed = value
        .parse::<i64>()
        .unwrap_or_else(|_| fail(&format!("Invalid value for {flag}: {value}")));
    if parsed <= 0 {
        fail(&format!("{flag} must be greater than 0"));
    }
    parsed
}

fn parse_max_positions(value: &str) -> u64 {
    let max_positions = value
        .parse::<u64>()
        .unwrap_or_else(|_| fail(&format!("Invalid value for --max-positions: {value}")));
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
    let mut user_keypair_path = env::var("DELEGATED_USER_KEYPAIR_PATH")
        .or_else(|_| env::var("KEYPAIR_PATH"))
        .unwrap_or_else(|_| default_keypair_path());
    let mut grant_manager_keypair_path = env::var("TRADER_MANAGEMENT_KEYPAIR_PATH").ok();
    let mut grant_uses = 1;
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
            "--user-keypair-path" => {
                user_keypair_path = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --user-keypair-path"));
            }
            "--grant-manager-keypair-path" => {
                grant_manager_keypair_path = Some(
                    args.next()
                        .unwrap_or_else(|| fail("Missing value for --grant-manager-keypair-path")),
                );
            }
            "--grant-uses" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| fail("Missing value for --grant-uses"));
                grant_uses = parse_positive_i64(&value, "--grant-uses");
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
    let max_positions = max_positions.unwrap_or(if trader_subaccount_index == 0 {
        DEFAULT_MAX_POSITIONS
    } else {
        1
    });
    if trader_subaccount_index > 0 && max_positions != 1 {
        fail("--max-positions must be 1 when --trader-subaccount-index is greater than 0");
    }

    CliArgs {
        api_url,
        rpc_url,
        user_keypair_path,
        grant_manager_keypair_path,
        grant_uses,
        trader_authority,
        trader_pda_index,
        trader_subaccount_index,
        max_positions,
    }
}

async fn account_exists(
    rpc: &RpcClient,
    account: &Pubkey,
) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(rpc
        .get_account_with_commitment(account, CommitmentConfig::confirmed())
        .await?
        .value
        .is_some())
}

async fn fetch_permission(
    rpc: &RpcClient,
    account: &Pubkey,
) -> Result<Permission, Box<dyn std::error::Error>> {
    let account_data = rpc
        .get_account_with_commitment(account, CommitmentConfig::confirmed())
        .await?
        .value
        .ok_or_else(|| format!("Permission account does not exist: {account}"))?;
    Ok(Permission::try_from_account_bytes(&account_data.data)?)
}

fn validate_permission_account(
    permission: &Permission,
    expected_authority: &Pubkey,
    expected_user: &Pubkey,
    required_permission: u64,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if permission.permission_authority != *expected_authority {
        return Err(format!(
            "{label} permission authority mismatch: expected {expected_authority}, got {}",
            permission.permission_authority
        )
        .into());
    }
    if permission.delegated_key != *expected_user {
        return Err(format!(
            "{label} permission delegated key mismatch: expected {expected_user}, got {}",
            permission.delegated_key
        )
        .into());
    }
    if permission.permission & required_permission != required_permission {
        return Err(
            format!("{label} permission is missing required bit {required_permission}").into(),
        );
    }
    if permission.allowed_signer_actions == 0 {
        return Err(format!("{label} permission has no remaining signer actions").into());
    }
    Ok(())
}

async fn send_or_simulate(
    rpc: &RpcClient,
    payer: &Pubkey,
    signers: &[&Keypair],
    instructions: &[solana_instruction::Instruction],
    label: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let blockhash = rpc.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(instructions, Some(payer), signers, blockhash);

    if env::var_os("PHOENIX_DRY_RUN").is_some() {
        println!("\nDry run: simulating {label} transaction without sending...");
        let simulation = rpc.simulate_transaction(&tx).await?;
        println!("{label} simulation result: {:?}", simulation.value.err);
        if let Some(logs) = simulation.value.logs {
            for log in logs {
                println!("{log}");
            }
        }
        return Ok(false);
    }

    let signature = rpc.send_and_confirm_transaction(&tx).await?;
    println!("\n{label} transaction confirmed!");
    println!("Signature: {signature}");
    println!("Explorer:  https://explorer.solana.com/tx/{signature}");
    Ok(true)
}

async fn maybe_grant_onboarding_budget(
    args: &CliArgs,
    rpc: &RpcClient,
    risk_authority: Pubkey,
    user_authority: Pubkey,
    permission_account: Pubkey,
) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(path) = args.grant_manager_keypair_path.as_ref() else {
        return Ok(false);
    };
    let grant_manager_keypair = read_keypair_file(path)
        .map_err(|error| format!("Failed to read grant keypair: {error}"))?;
    let grant_authority = grant_manager_keypair.pubkey();
    let authority_permission_account = if grant_authority == risk_authority {
        grant_authority
    } else {
        get_permission_address(&risk_authority, &grant_authority)
    };

    if grant_authority != risk_authority {
        let manager_permission = fetch_permission(rpc, &authority_permission_account).await?;
        validate_permission_account(
            &manager_permission,
            &risk_authority,
            &grant_authority,
            TRADER_MANAGEMENT_PERMISSION,
            "Grant manager",
        )?;
        println!(
            "Grant manager uses left before grant: {}",
            manager_permission.allowed_signer_actions
        );
    }

    let mut instructions = Vec::<solana_instruction::Instruction>::new();
    if !account_exists(rpc, &permission_account).await? {
        let create_params = CreatePermissionParams::builder()
            .payer(grant_authority)
            .permission_authority(risk_authority)
            .delegated_key(user_authority)
            .permission_account(permission_account)
            .build()?;
        instructions.push(create_create_permission_ix(create_params)?.into());
    }
    let grant_params = SetPermissionDelegatedParams::builder()
        .authority(grant_authority)
        .authority_permission_account(authority_permission_account)
        .target_permission_account(permission_account)
        .permission_authority(risk_authority)
        .delegated_key(user_authority)
        .permission(TRADER_ONBOARDING_PERMISSION)
        .allowed_signer_actions(Some(args.grant_uses))
        .build()?;
    instructions.push(create_set_permission_delegated_ix(grant_params)?.into());

    println!("\nPhoenix-side grant");
    println!("Grant authority:        {grant_authority}");
    println!("Authority permission:   {authority_permission_account}");
    println!("User authority:         {user_authority}");
    println!("Target permission:      {permission_account}");
    println!("Granted uses:           {}", args.grant_uses);
    println!("Grant instruction count: {}", instructions.len());

    let sent = send_or_simulate(
        rpc,
        &grant_authority,
        &[&grant_manager_keypair],
        &instructions,
        "grant",
    )
    .await?;
    Ok(sent)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argv: Vec<String> = env::args().collect();
    if argv.len() == 1 {
        println!("This example can submit live permission-grant and onboarding transactions.");
        println!(
            "The user-side transaction must be signed by the delegated user key that received \
             trader-onboarding permission.\n"
        );
        println!("{USAGE}");
        return Ok(());
    }

    let args = parse_args(argv);
    let user_keypair = read_keypair_file(&args.user_keypair_path)
        .map_err(|error| format!("Failed to read delegated user keypair: {error}"))?;
    let user_authority = user_keypair.pubkey();
    let trader = TraderKey::new_with_idx(
        args.trader_authority,
        args.trader_pda_index,
        args.trader_subaccount_index,
    );

    println!("Fetching exchange metadata...");
    let http = PhoenixHttpClient::new_public(args.api_url.clone())?;
    let exchange = http.get_exchange().await?.into();
    let metadata = PhoenixMetadata::new(exchange);
    let keys = metadata.keys();

    let risk_authority = Pubkey::from_str(&keys.current_authorities.risk_authority)?;
    let permission_account = get_permission_address(&risk_authority, &user_authority);
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
    let grant_sent = maybe_grant_onboarding_budget(
        &args,
        &rpc,
        risk_authority,
        user_authority,
        permission_account,
    )
    .await?;
    if env::var_os("PHOENIX_DRY_RUN").is_some() && args.grant_manager_keypair_path.is_some() {
        println!(
            "\nDry-run grant was not committed, so the user-side onboarding transaction was not \
             built."
        );
        return Ok(());
    }

    let permission = fetch_permission(&rpc, &permission_account).await?;
    validate_permission_account(
        &permission,
        &risk_authority,
        &user_authority,
        TRADER_ONBOARDING_PERMISSION,
        "User",
    )?;

    let trader_account = trader.pda();
    let trader_already_registered = account_exists(&rpc, &trader_account).await?;
    let mut instructions = Vec::<solana_instruction::Instruction>::new();
    if !trader_already_registered {
        let register_params = RegisterTraderParams::builder()
            .payer(user_authority)
            .trader(trader.authority())
            .trader_account(trader_account)
            .max_positions(args.max_positions)
            .trader_pda_index(args.trader_pda_index)
            .subaccount_index(args.trader_subaccount_index)
            .build()?;
        instructions.push(create_register_trader_ix(register_params)?.into());
    }

    let onboard_params = OnboardTraderDelegatedParams::builder()
        .authority(user_authority)
        .permission_account(permission_account)
        .trader_account(trader_account)
        .global_trader_index(global_trader_index)
        .active_trader_buffer(active_trader_buffer)
        .build()?;
    instructions.push(create_onboard_trader_delegated_ix(onboard_params)?.into());

    println!("\nUser-side onboarding");
    println!("User authority:          {user_authority}");
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
    println!("Grant submitted:         {grant_sent}");
    println!("Instruction count:       {}", instructions.len());

    send_or_simulate(
        &rpc,
        &user_authority,
        &[&user_keypair],
        &instructions,
        "onboarding",
    )
    .await?;

    Ok(())
}
