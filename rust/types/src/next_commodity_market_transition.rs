use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::next_market_calendar_transition::{
    MarketCalendarDaySchedule, MarketCalendarResponse, MarketCalendarStateView, MarketCalendarView,
    NextMarketCalendarTransition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub enum CommodityMarketStateView {
    Open,
    AfterHours,
}

impl CommodityMarketStateView {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Open => Self::AfterHours,
            Self::AfterHours => Self::Open,
        }
    }
}

impl From<MarketCalendarStateView> for CommodityMarketStateView {
    fn from(value: MarketCalendarStateView) -> Self {
        match value {
            MarketCalendarStateView::Open => Self::Open,
            MarketCalendarStateView::AfterHours => Self::AfterHours,
        }
    }
}

/// A single named session within a [`CommodityMarketDaySchedule`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CommodityMarketSessionRange {
    pub start: String,
    pub end: String,
    /// Serialized calendar mode name, e.g. `"EXTERNAL"`, `"INTERNAL"`.
    pub mode: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CommodityMarketDaySchedule {
    pub sessions: Vec<CommodityMarketSessionRange>,
}

impl From<MarketCalendarDaySchedule> for CommodityMarketDaySchedule {
    fn from(value: MarketCalendarDaySchedule) -> Self {
        let mut sessions: Vec<CommodityMarketSessionRange> = value
            .active_hours
            .into_iter()
            .map(|range| CommodityMarketSessionRange {
                start: range.start,
                end: range.end,
                mode: "EXTERNAL".to_string(),
            })
            .chain(
                value
                    .inactive_hours
                    .into_iter()
                    .map(|range| CommodityMarketSessionRange {
                        start: range.start,
                        end: range.end,
                        mode: "INTERNAL".to_string(),
                    }),
            )
            .collect();
        sessions.sort_by_key(|session| session.start.clone());
        Self { sessions }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CommodityMarketCalendarView {
    pub weekly_schedule: BTreeMap<String, CommodityMarketDaySchedule>,
    pub date_overrides: BTreeMap<String, CommodityMarketDaySchedule>,
}

impl From<MarketCalendarView> for CommodityMarketCalendarView {
    fn from(value: MarketCalendarView) -> Self {
        Self {
            weekly_schedule: value
                .weekly_schedule
                .into_iter()
                .map(|(day, schedule)| (day, schedule.into()))
                .collect(),
            date_overrides: value
                .date_overrides
                .into_iter()
                .map(|(date, schedule)| (date, schedule.into()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct CommodityMarketCalendarResponse {
    pub market: String,
    pub loaded_at: DateTime<Utc>,
    pub raw_json: String,
    pub calendar: CommodityMarketCalendarView,
}

impl From<MarketCalendarResponse> for CommodityMarketCalendarResponse {
    fn from(value: MarketCalendarResponse) -> Self {
        Self {
            market: value.market,
            loaded_at: value.loaded_at,
            raw_json: value.raw_json,
            calendar: value.calendar.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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

impl From<NextMarketCalendarTransition> for NextCommodityMarketTransition {
    fn from(value: NextMarketCalendarTransition) -> Self {
        Self {
            market: value.market,
            loaded_at: value.loaded_at,
            utc_next_transition: value.utc_next_transition,
            next_market_state: value.next_market_state.map(Into::into),
            current_state: value.current_state.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_current_commodity_calendar_response_shape() {
        let json = r#"{
            "market": "cme_commodities",
            "loadedAt": "2026-06-29T02:04:23.786418217Z",
            "rawJson": "{}",
            "calendar": {
                "weeklySchedule": {
                    "Mon": {
                        "sessions": [
                            {
                                "start": "00:00:00",
                                "end": "17:00:00",
                                "mode": "EXTERNAL"
                            },
                            {
                                "start": "17:00:00",
                                "end": "18:00:00",
                                "mode": "INTERNAL"
                            }
                        ]
                    }
                },
                "dateOverrides": {}
            }
        }"#;

        let response: CommodityMarketCalendarResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.market, "cme_commodities");
        assert_eq!(
            response.calendar.weekly_schedule["Mon"].sessions[0].mode,
            "EXTERNAL"
        );
    }
}
