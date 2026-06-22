//! Shared service-account DTOs for Phoenix admin APIs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateServiceAccountRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub client_id: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ServiceAccountResponse {
    pub account: ServiceAccountDto,
    pub keys: Vec<ServiceAccountKeyDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateServiceKeyRequest {
    pub key_id: String,
    pub name: String,
    pub public_key: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ServiceAccountDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ServiceAccountKeyDto {
    pub id: i64,
    pub key_id: String,
    pub name: String,
    pub public_key: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub rotated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct UpdateStatusRequest {
    pub status: String,
}
