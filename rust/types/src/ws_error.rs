//! WebSocket error types for the Phoenix SDK.

use thiserror::Error;

/// Errors that can occur when using the Phoenix WebSocket SDK.
#[derive(Debug, Error)]
pub enum PhoenixWsError {
    /// Failed to connect to the WebSocket server.
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(#[from] tokio_tungstenite::tungstenite::Error),

    /// Failed to parse the WebSocket URL.
    #[error("Invalid WebSocket URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Unsupported URL scheme in configuration.
    #[error("Unsupported URL scheme: {0}")]
    UnsupportedUrlScheme(String),

    /// Invalid header value.
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(String),

    /// Failed to serialize a message.
    #[error("Failed to serialize message: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    /// The subscription channel was closed unexpectedly.
    #[error("Subscription channel closed")]
    SubscriptionClosed,

    /// The WebSocket connection was closed.
    #[error("WebSocket connection closed: code={code}, reason={reason}")]
    ConnectionClosed { code: u16, reason: String },

    /// Failed to send a message on the WebSocket.
    #[error("Failed to send WebSocket message")]
    SendFailed,

    /// Invalid trader key configuration.
    #[error("Invalid trader key: {0}")]
    InvalidTraderKey(String),

    /// Missing environment variable.
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
}
