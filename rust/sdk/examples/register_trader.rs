//! Example: Register a trader with a referral code.
//!
//! Run with:
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
  cargo run -p phoenix-rise --example register_trader -- <AUTHORITY_PUBKEY> --ref-code <code>

Example:
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
    let mut ref_code = None::<String>;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
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
    let ref_code = ref_code.unwrap_or_else(|| fail("Missing --ref-code"));
    let authority = Pubkey::from_str(&authority)?;

    let client = PhoenixHttpClient::new_from_env_with_auth()?;
    let response = client
        .invite()
        .activate_referral(&authority, &ref_code)
        .await?;
    println!("{}", response);

    Ok(())
}
