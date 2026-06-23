use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommodityMarketStateView {
    Open,
    AfterHours,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommodityMarketHoursRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommodityMarketDaySchedule {
    pub active_hours: Vec<CommodityMarketHoursRange>,
    pub inactive_hours: Vec<CommodityMarketHoursRange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommodityMarketCalendarView {
    pub weekly_schedule: BTreeMap<String, CommodityMarketDaySchedule>,
    pub date_overrides: BTreeMap<String, CommodityMarketDaySchedule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommodityMarketCalendarResponse {
    pub market: String,
    pub loaded_at: DateTime<Utc>,
    pub raw_toml: String,
    pub calendar: CommodityMarketCalendarView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextCommodityMarketTransition {
    pub market: String,
    pub loaded_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_next_transition: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_market_state: Option<CommodityMarketStateView>,
    pub current_state: CommodityMarketStateView,
}
