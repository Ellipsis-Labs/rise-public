//! HTTP error types for the Phoenix SDK.

use thiserror::Error;

/// Errors that can occur when using the Phoenix HTTP client.
#[derive(Debug, Error)]
pub enum PhoenixHttpError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    /// Failed to parse response.
    #[error("Failed to parse response: {0}")]
    ParseFailed(String),

    /// API returned an error.
    #[error(
        "API error: {status} - {message}{code}",
        code = error_code
            .as_ref()
            .map(|code| format!(" (code: {code})"))
            .unwrap_or_default()
    )]
    ApiError {
        status: u16,
        message: String,
        error_code: Option<String>,
    },

    /// Authentication failed or requires reauthentication.
    #[error(
        "Authentication error{status}: {message}{code}",
        status = status
            .map(|status| format!(" (status: {status})"))
            .unwrap_or_default(),
        code = error_code
            .as_ref()
            .map(|code| format!(" (code: {code})"))
            .unwrap_or_default()
    )]
    Authentication {
        status: Option<u16>,
        message: String,
        error_code: Option<String>,
    },

    /// API rate limit was hit and automatic retries were exhausted or disabled.
    #[error(
        "Rate limited after {attempts} attempt(s), retry_after_seconds={retry_after_seconds:?}: \
         {message}{code}",
        code = error_code
            .as_ref()
            .map(|code| format!(" (code: {code})"))
            .unwrap_or_default()
    )]
    RateLimited {
        retry_after_seconds: Option<u64>,
        message: String,
        error_code: Option<String>,
        attempts: u32,
    },

    /// Missing environment variable.
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    /// Requested SDK feature is not supported by this helper surface.
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}

impl PhoenixHttpError {
    pub fn error_code(&self) -> Option<&str> {
        match self {
            PhoenixHttpError::ApiError { error_code, .. }
            | PhoenixHttpError::Authentication { error_code, .. }
            | PhoenixHttpError::RateLimited { error_code, .. } => error_code.as_deref(),
            _ => None,
        }
    }

    pub fn is_auth_error(&self) -> bool {
        matches!(self, PhoenixHttpError::Authentication { .. })
    }

    pub fn is_rate_limited(&self) -> bool {
        match self {
            PhoenixHttpError::RateLimited { .. } => true,
            PhoenixHttpError::ApiError {
                status, error_code, ..
            } => *status == 429 || matches!(error_code.as_deref(), Some("rate_limited")),
            _ => false,
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            PhoenixHttpError::ApiError { status, .. } => Some(*status),
            PhoenixHttpError::Authentication { status, .. } => *status,
            _ => None,
        }
    }
}
