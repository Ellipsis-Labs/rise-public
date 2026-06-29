use std::error::Error;
use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use phoenix_rise::api::{AuthSession, PhoenixSolanaKeypairAuthSigner};
use serde::Serialize;

use crate::context::{CommandCtx, expand_cli_path};
use crate::output::print_json;
use crate::util::read_wallet_keypair;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Login with a wallet keypair or service-account credential file.
    Login(AuthLoginArgs),
    /// Refresh the cached API session.
    Refresh(AuthOutputArgs),
    /// Print the cached API session.
    Session(AuthOutputArgs),
    /// Clear the cached API session.
    Logout,
    /// Print the auth session cache path.
    CachePath,
}

#[derive(Debug, Clone, Args)]
pub struct AuthLoginArgs {
    /// Solana keypair path. Falls back to PHOENIX_AUTH_KEYPAIR_PATH,
    /// SOLANA_KEYPAIR_PATH, then ~/.config/solana/id.json.
    #[arg(long, conflicts_with = "service_credential_file")]
    pub keypair_path: Option<PathBuf>,

    /// Service-account credential JSON file containing client_id, key_id, and
    /// private_key for challenge-based service auth.
    #[arg(long, conflicts_with = "keypair_path")]
    pub service_credential_file: Option<PathBuf>,

    #[command(flatten)]
    pub output: AuthOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct AuthOutputArgs {
    /// Output format for auth session material.
    #[arg(long, value_enum, default_value_t = AuthOutputFormat::Json)]
    pub output: AuthOutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AuthOutputFormat {
    Json,
    Env,
}

#[derive(Debug, Serialize)]
struct EnvAuthSession<'a> {
    #[serde(rename = "PHOENIX_ACCESS_TOKEN")]
    access_token: &'a str,
    #[serde(
        rename = "PHOENIX_REFRESH_TOKEN",
        skip_serializing_if = "Option::is_none"
    )]
    refresh_token: Option<&'a str>,
    #[serde(rename = "PHOENIX_POP_KEY")]
    pop_key: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthCachePathOutput {
    path: String,
}

pub async fn run(cmd: AuthCommand, ctx: &CommandCtx) -> Result<(), Box<dyn Error>> {
    match cmd {
        AuthCommand::Login(args) => {
            if ctx.ledger.is_some() {
                return Err(
                    "ledger wallet login is not wired yet; use --service-credential-file or \
                     --keypair/--keypair-path for this command"
                        .into(),
                );
            }

            let client = ctx.auth_http_client()?;
            let auth = client
                .auth()
                .ok_or_else(|| std::io::Error::other("auth client is not configured"))?;
            let session = if let Some(path) = args.service_credential_file.clone() {
                let signer = PhoenixSolanaKeypairAuthSigner::from_service_account_path(
                    expand_cli_path(path)?,
                )?;
                auth.login_with_signer(&signer).await?
            } else {
                let keypair = read_wallet_keypair(args.keypair_path.or(ctx.keypair_path.clone()))?;
                auth.login_with_wallet_keypair(&keypair).await?
            };
            print_auth_session(&session, args.output.output, ctx)?;
        }
        AuthCommand::Refresh(args) => {
            let client = ctx.auth_http_client()?;
            let auth = client
                .auth()
                .ok_or_else(|| std::io::Error::other("auth client is not configured"))?;
            let session = auth.refresh_session().await?;
            print_auth_session(&session, args.output, ctx)?;
        }
        AuthCommand::Session(args) => {
            let client = ctx.auth_http_client()?;
            let auth = client
                .auth()
                .ok_or_else(|| std::io::Error::other("auth client is not configured"))?;
            let session = auth
                .session()?
                .ok_or_else(|| std::io::Error::other("no auth session available"))?;
            print_auth_session(&session, args.output, ctx)?;
        }
        AuthCommand::Logout => {
            let client = ctx.auth_http_client()?;
            let auth = client
                .auth()
                .ok_or_else(|| std::io::Error::other("auth client is not configured"))?;
            auth.logout().await?;
        }
        AuthCommand::CachePath => {
            print_json(
                &AuthCachePathOutput {
                    path: ctx.auth_session_path().display().to_string(),
                },
                ctx.output,
            )?;
        }
    }

    Ok(())
}

fn print_auth_session(
    session: &AuthSession,
    output: AuthOutputFormat,
    ctx: &CommandCtx,
) -> Result<(), Box<dyn Error>> {
    if ctx.output.json && output == AuthOutputFormat::Env {
        return Err("--json cannot be combined with --output env".into());
    }

    match output {
        AuthOutputFormat::Json => print_json(&session.snapshot(), ctx.output),
        AuthOutputFormat::Env => {
            let snapshot = session.snapshot();
            let env_session = EnvAuthSession {
                access_token: &snapshot.access_token,
                refresh_token: snapshot.refresh_token.as_deref(),
                pop_key: &snapshot.pop_key,
            };

            println!("PHOENIX_ACCESS_TOKEN={}", env_session.access_token);
            if let Some(refresh_token) = env_session.refresh_token {
                println!("PHOENIX_REFRESH_TOKEN={refresh_token}");
            }
            println!("PHOENIX_POP_KEY={}", env_session.pop_key);
            Ok(())
        }
    }
}
