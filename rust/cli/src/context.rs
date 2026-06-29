use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use phoenix_rise::api::{
    FileAuthSessionStore, PhoenixEnv, PhoenixHttpAuthConfig, PhoenixHttpClient,
};
use url::Url;

use crate::app::{Cli, default_rpc_url};
use crate::output::OutputOptions;
use crate::util::{expand_home_path, home_dir};

const AUTH_SESSION_RELATIVE_PATH: &str = ".config/phoenix/rise-cli/session.json";

#[derive(Debug)]
pub struct CommandCtx {
    pub env: PhoenixEnv,
    pub rpc_url: String,
    pub output: OutputOptions,
    pub keypair_path: Option<PathBuf>,
    pub payer_keypair_path: Option<PathBuf>,
    pub ledger: Option<String>,
    auth_session_path: PathBuf,
}

impl CommandCtx {
    pub fn try_from_cli(cli: &Cli) -> Result<Self, Box<dyn Error>> {
        let mut env = PhoenixEnv::load();

        if let Some(api_url) = &cli.api_url {
            env.api_url = api_url.clone();
        }

        env.ws_url = if let Some(ws_url) = &cli.ws_url {
            ws_url.clone()
        } else {
            derive_ws_url(&env.api_url)?
        };

        let rpc_url = cli
            .rpc_url
            .clone()
            .or_else(|| std::env::var("PHOENIX_RPC_URL").ok())
            .or_else(|| std::env::var("SOLANA_RPC_URL").ok())
            .unwrap_or_else(|| default_rpc_url().to_string());

        Ok(Self {
            env,
            rpc_url,
            output: OutputOptions {
                json: cli.json,
                pretty: cli.pretty,
            },
            keypair_path: cli.keypair_path.clone(),
            payer_keypair_path: cli.payer_keypair_path.clone(),
            ledger: cli.ledger.clone(),
            auth_session_path: default_cli_auth_session_path()?,
        })
    }

    pub fn auth_session_path(&self) -> &PathBuf {
        &self.auth_session_path
    }

    pub fn http_client(&self) -> Result<PhoenixHttpClient, Box<dyn Error>> {
        let auth = PhoenixHttpAuthConfig::new()
            .with_file_session_store_path(self.auth_session_path.clone())?;
        let builder = PhoenixHttpClient::builder(self.env.api_url.clone())
            .with_auth(auth)
            .with_auth_session_from_env()?
            .with_auth_signer_from_env()?;
        Ok(builder.build()?)
    }

    pub fn auth_http_client(&self) -> Result<PhoenixHttpClient, Box<dyn Error>> {
        let store = Arc::new(FileAuthSessionStore::new(self.auth_session_path.clone()));
        Ok(PhoenixHttpClient::builder(self.env.api_url.clone())
            .with_auth_session_store(store)
            .build()?)
    }
}

pub fn default_cli_auth_session_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(home_dir()?.join(AUTH_SESSION_RELATIVE_PATH))
}

pub fn expand_cli_path(path: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    expand_home_path(path)
}

fn derive_ws_url(api_url: &str) -> Result<String, Box<dyn Error>> {
    let mut url = Url::parse(api_url)?;
    let scheme = url.scheme();

    if let Some(rest) = scheme.strip_prefix("http") {
        let _ = url.set_scheme(&format!("ws{rest}"));
    } else if scheme != "ws" && scheme != "wss" {
        return Err(format!("unsupported URL scheme for api_url: {scheme}").into());
    }

    let mut segments: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|seg| !seg.is_empty()).collect())
        .unwrap_or_default();

    if segments.ends_with(&["v1", "ws"]) {
    } else if segments.last().copied() == Some("ws") {
        segments.pop();
        segments.push("v1");
        segments.push("ws");
    } else {
        segments.push("v1");
        segments.push("ws");
    }

    url.set_path(&format!("/{}", segments.join("/")));
    url.set_query(None);
    url.set_fragment(None);

    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_v1_ws_url_from_api_url() {
        assert_eq!(
            derive_ws_url("https://perp-api.phoenix.trade").unwrap(),
            "wss://perp-api.phoenix.trade/v1/ws"
        );
    }

    #[test]
    fn uses_perp_api_as_sdk_default_api_url() {
        assert_eq!(
            PhoenixEnv::default().api_url,
            "https://perp-api.phoenix.trade"
        );
    }
}
