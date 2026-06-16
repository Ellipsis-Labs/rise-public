//! Environment configuration for Phoenix SDK.
//!
//! This module provides a centralized way to load configuration from
//! environment variables with sane defaults.

use url::Url;

use crate::phoenix_rise_types::PhoenixWsError;

pub(crate) const PHOENIX_WS_URL_ENV: &str = "PHOENIX_WS_URL";
pub(crate) const PHOENIX_API_URL_ENV: &str = "PHOENIX_API_URL";

pub(crate) const DEFAULT_PHOENIX_API_URL: &str = "https://perp-api.phoenix.trade";
pub(crate) const DEFAULT_WS_PATH: &str = "/v1/ws";

fn default_ws_url() -> String {
    ws_url_from_api_url(DEFAULT_PHOENIX_API_URL)
        .unwrap_or_else(|_| format!("wss://perp-api.phoenix.trade{}", DEFAULT_WS_PATH))
}

/// Environment configuration for Phoenix SDK.
///
/// Holds the API URL and WebSocket URL needed to connect to the Phoenix API.
///
/// # Example
///
/// ```no_run
/// use phoenix_rise::PhoenixEnv;
///
/// // Load configuration from environment variables
/// let env = PhoenixEnv::load();
///
/// println!("API URL: {}", env.api_url);
/// println!("WS URL: {}", env.ws_url);
/// ```
#[derive(Debug, Clone)]
pub struct PhoenixEnv {
    /// Base URL for the Phoenix HTTP API.
    pub api_url: String,
    /// WebSocket URL for real-time subscriptions.
    pub ws_url: String,
}

impl PhoenixEnv {
    /// Build environment configuration from an API URL, deriving the canonical
    /// Phoenix WebSocket URL at `/v1/ws` under any configured path prefix.
    pub fn from_api_url(api_url: impl Into<String>) -> Result<Self, PhoenixWsError> {
        let api_url = api_url.into();
        let ws_url = ws_url_from_api_url(&api_url)?;
        Ok(Self { api_url, ws_url })
    }

    /// Load configuration from environment variables.
    ///
    /// # Environment Variables
    ///
    /// * `PHOENIX_API_URL` - Base URL for the Phoenix API. Defaults to `https://perp-api.phoenix.trade`.
    /// * `PHOENIX_WS_URL` - WebSocket URL. If not set, derived from the API URL
    ///   by converting the scheme (https→wss, http→ws) and using `/v1/ws`.
    pub fn load() -> Self {
        let api_url = std::env::var(PHOENIX_API_URL_ENV)
            .unwrap_or_else(|_| DEFAULT_PHOENIX_API_URL.to_string());

        let ws_url = std::env::var(PHOENIX_WS_URL_ENV)
            .unwrap_or_else(|_| ws_url_from_api_url(&api_url).unwrap_or_else(|_| default_ws_url()));

        Self { api_url, ws_url }
    }
}

impl Default for PhoenixEnv {
    /// Returns the default environment configuration.
    ///
    /// Uses `https://perp-api.phoenix.trade` as the API URL and
    /// `wss://perp-api.phoenix.trade/v1/ws` as the WebSocket URL.
    fn default() -> Self {
        Self {
            api_url: DEFAULT_PHOENIX_API_URL.to_string(),
            ws_url: default_ws_url(),
        }
    }
}

// ============================================================================
// URL utilities
// ============================================================================

pub(crate) fn ws_url_from_api_url(api_url: &str) -> Result<String, PhoenixWsError> {
    let mut url = Url::parse(api_url)?;

    let scheme = url.scheme().to_ascii_lowercase();
    match scheme.as_str() {
        "http" => {
            url.set_scheme("ws")
                .map_err(|_| PhoenixWsError::UnsupportedUrlScheme(scheme.clone()))?;
        }
        "https" => {
            url.set_scheme("wss")
                .map_err(|_| PhoenixWsError::UnsupportedUrlScheme(scheme.clone()))?;
        }
        "ws" | "wss" => {}
        _ => return Err(PhoenixWsError::UnsupportedUrlScheme(scheme)),
    }

    let path = url.path().trim_end_matches('/');
    let ws_path = if path.ends_with(DEFAULT_WS_PATH) {
        path.to_string()
    } else if path.ends_with("/v1") {
        format!("{path}/ws")
    } else if path.is_empty() || path == "/" {
        DEFAULT_WS_PATH.to_string()
    } else {
        format!("{path}{DEFAULT_WS_PATH}")
    };

    url.set_path(&ws_path);
    url.set_query(None);
    url.set_fragment(None);

    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {
        // load() always succeeds with defaults when env vars are not set
        let env = PhoenixEnv::load();
        // Should have valid URLs (either from env or defaults)
        assert!(!env.api_url.is_empty());
        assert!(!env.ws_url.is_empty());
    }

    #[test]
    fn test_default_env() {
        let env = PhoenixEnv::default();
        assert_eq!(env.api_url, "https://perp-api.phoenix.trade");
        assert_eq!(env.ws_url, "wss://perp-api.phoenix.trade/v1/ws");
    }

    #[test]
    fn test_from_api_url_derives_ws_url() {
        let env = PhoenixEnv::from_api_url("http://localhost:8080").unwrap();
        assert_eq!(env.api_url, "http://localhost:8080");
        assert_eq!(env.ws_url, "ws://localhost:8080/v1/ws");
    }

    #[test]
    fn test_ws_url_from_https_api_url_appends_ws() {
        let ws_url = ws_url_from_api_url("https://perp-api.phoenix.trade").unwrap();
        assert_eq!(ws_url, "wss://perp-api.phoenix.trade/v1/ws");
    }

    #[test]
    fn test_ws_url_from_http_api_url_appends_ws() {
        let ws_url = ws_url_from_api_url("http://localhost:8080").unwrap();
        assert_eq!(ws_url, "ws://localhost:8080/v1/ws");
    }

    #[test]
    fn test_ws_url_preserves_path() {
        let ws_url = ws_url_from_api_url("https://api.phoenix.trade/v1").unwrap();
        assert_eq!(ws_url, "wss://api.phoenix.trade/v1/ws");
    }

    #[test]
    fn test_ws_url_preserves_path_prefix() {
        let ws_url = ws_url_from_api_url("https://gateway.example.com/phoenix").unwrap();
        assert_eq!(ws_url, "wss://gateway.example.com/phoenix/v1/ws");
    }

    #[test]
    fn test_ws_url_handles_trailing_slash() {
        let ws_url = ws_url_from_api_url("https://perp-api.phoenix.trade/").unwrap();
        assert_eq!(ws_url, "wss://perp-api.phoenix.trade/v1/ws");
    }

    #[test]
    fn test_ws_url_does_not_double_append_ws() {
        let ws_url = ws_url_from_api_url("https://perp-api.phoenix.trade/v1/ws").unwrap();
        assert_eq!(ws_url, "wss://perp-api.phoenix.trade/v1/ws");
    }
}
