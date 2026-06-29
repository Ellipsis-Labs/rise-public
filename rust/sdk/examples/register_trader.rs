//! Example: Register a trader with either an access code or a referral code.
//!
//! Run with:
//!   cargo run -p phoenix-rise --example register_trader -- AUTHORITY
//! --access-code ACCESS123
//!   cargo run -p phoenix-rise --example register_trader -- AUTHORITY
//! --ref-code REF123
//!
//! Referral activation uses /v1/referral/activate and requires a user auth
//! session for AUTHORITY.

use std::process;
use std::str::FromStr;

use phoenix_rise::api::PhoenixHttpClient;
use solana_pubkey::Pubkey;

const USAGE: &str = r#"Usage:
  cargo run -p phoenix-rise --example register_trader -- <AUTHORITY_PUBKEY> (--access-code <code> | --ref-code <code>)

Examples:
  cargo run -p phoenix-rise --example register_trader -- AUTHORITY --access-code ACCESS123
  cargo run -p phoenix-rise --example register_trader -- AUTHORITY --ref-code REF123

Note:
  --ref-code requires a user auth session for AUTHORITY.
"#;

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    eprintln!();
    eprintln!("{USAGE}");
    process::exit(1);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut authority = None::<String>;
    let mut access_code = None::<String>;
    let mut ref_code = None::<String>;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--access-code" => {
                access_code = Some(
                    args.next()
                        .unwrap_or_else(|| fail("Missing value for --access-code")),
                );
            }
            "--ref-code" => {
                ref_code = Some(
                    args.next()
                        .unwrap_or_else(|| fail("Missing value for --ref-code")),
                );
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                process::exit(0);
            }
            value if value.starts_with('-') => fail(&format!("Unknown argument: {value}")),
            value => {
                if authority.is_some() {
                    fail(&format!("Unexpected positional argument: {value}"));
                }
                authority = Some(value.to_string());
            }
        }
    }

    let authority = authority.unwrap_or_else(|| fail("Missing <AUTHORITY_PUBKEY>"));
    if access_code.is_some() == ref_code.is_some() {
        fail(
            "Pass exactly one of --access-code or --ref-code so the example can call the correct \
             invite endpoint.",
        );
    }

    let authority = Pubkey::from_str(&authority)?;

    let response = match (access_code, ref_code) {
        (Some(code), None) => {
            let client = PhoenixHttpClient::new_from_env()?;
            client.invite().activate_invite(&authority, &code).await?
        }
        (None, Some(code)) => {
            let client = PhoenixHttpClient::new_from_env_with_auth()?;
            client.invite().activate_referral(&authority, &code).await?
        }
        _ => unreachable!("validated above"),
    };
    println!("{}", response);

    Ok(())
}
