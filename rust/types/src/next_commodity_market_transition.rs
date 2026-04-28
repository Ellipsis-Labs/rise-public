use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Commodity market state view returned by the transition endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub enum CommodityMarketStateView {
    Open,
    AfterHours,
}

/// Response for `GET /v1/market/next-commodity-market-transition`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct NextCommodityMarketTransition {
    pub market: String,
    pub loaded_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utc_next_transition: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_market_state: Option<CommodityMarketStateView>,
    pub current_state: CommodityMarketStateView,
}
