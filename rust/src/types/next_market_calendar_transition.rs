use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarketCalendarStateView {
    Open,
    AfterHours,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCalendarHoursRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCalendarDaySchedule {
    pub active_hours: Vec<MarketCalendarHoursRange>,
    pub inactive_hours: Vec<MarketCalendarHoursRange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCalendarView {
    pub weekly_schedule: BTreeMap<String, MarketCalendarDaySchedule>,
    pub date_overrides: BTreeMap<String, MarketCalendarDaySchedule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCalendarResponse {
    pub market: String,
    pub market_calendar_id: String,
    pub description: String,
    pub calendar_uri: String,
    pub content_sha256: String,
    pub loaded_at: DateTime<Utc>,
    pub raw_toml: String,
    pub calendar: MarketCalendarView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCalendarRecord {
    pub market_calendar_id: String,
    pub description: String,
    pub s3_path: String,
    pub calendar_uri: String,
    pub content_sha256: String,
    pub loaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub raw_toml: String,
    pub calendar: MarketCalendarView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCalendarSummary {
    pub market_calendar_id: String,
    pub description: String,
    pub s3_path: String,
    pub calendar_uri: String,
    pub content_sha256: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCalendarListResponse {
    pub calendars: Vec<MarketCalendarSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextMarketCalendarTransition {
    pub market: String,
    pub market_calendar_id: String,
    pub calendar_uri: String,
    pub loaded_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utc_next_transition: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_market_state: Option<MarketCalendarStateView>,
    pub current_state: MarketCalendarStateView,
}
