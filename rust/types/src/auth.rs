//! Shared authentication wire types for Phoenix HTTP APIs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct AuthResponse {
    pub token_type: String,
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub refresh_expires_in: u64,
    pub pop_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct PrivyLoginRequest {
    pub privy_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WalletLoginRequest {
    pub wallet_pubkey: String,
    pub signature: String,
    pub nonce_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WalletTransactionChallengeRequest {
    pub wallet_pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WalletTransactionChallengeResponse {
    pub nonce_id: String,
    /// Base64-encoded unsigned Solana legacy transaction (bincode wire
    /// format). See [`WalletTransactionLoginRequest::signed_transaction`] for
    /// the accepted signing shape.
    pub unsigned_transaction: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WalletTransactionLoginRequest {
    pub wallet_pubkey: String,
    pub nonce_id: String,
    /// Base64-encoded fully-signed Solana legacy transaction (bincode wire
    /// format). Wallets are permitted to prepend ComputeBudget instructions
    /// before signing; any other instruction is rejected. The transaction
    /// uses the deterministic, non-recent blockhash the server issued so
    /// the signed bytes can never be broadcast on-chain.
    pub signed_transaction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ServiceChallengeRequest {
    pub client_id: String,
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ChallengeResponse {
    pub nonce: String,
    pub message: String,
    pub expires_at: String,
    pub key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ServiceLoginRequest {
    pub client_id: String,
    pub key_id: Option<String>,
    pub nonce: String,
    pub timestamp: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WalletNonceQuery {
    pub wallet_pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct WalletNonceResponse {
    pub nonce_id: String,
    pub message: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CounterHashResponse {
    pub counter_hash: Option<String>,
}

pub type JwksResponse = serde_json::Value;
