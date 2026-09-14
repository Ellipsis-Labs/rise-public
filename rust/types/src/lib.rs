//! Owned Phoenix domain and API types for Rise.
//!
//! These types mirror the wire format used by the Phoenix API without
//! requiring the full phoenix-api-types crate and its dependencies.
//! Serialization is intentionally feature-gated: enable `serde` for API DTOs
//! and `utoipa` when generating OpenAPI schemas.

#[cfg(feature = "serde")]
pub mod auth;
#[cfg(feature = "serde")]
pub mod candles;
#[cfg(feature = "serde")]
pub mod core;
#[cfg(feature = "serde")]
pub mod exchange;
#[cfg(feature = "serde")]
pub mod exchange_ws;
#[cfg(feature = "serde")]
pub mod funding;
#[cfg(feature = "serde")]
pub mod ix;
#[cfg(feature = "serde")]
pub mod js_safe_ints;
#[cfg(feature = "serde")]
pub mod l2book;
#[cfg(feature = "serde")]
pub mod market;
#[cfg(feature = "serde")]
pub mod market_state;
#[cfg(feature = "serde")]
pub mod market_stats;
#[cfg(feature = "serde")]
pub mod next_commodity_market_transition;
#[cfg(feature = "serde")]
pub mod next_market_calendar_transition;
#[cfg(feature = "serde")]
pub mod service_accounts;
#[cfg(feature = "serde")]
pub mod trader;
#[cfg(feature = "serde")]
pub mod trader_http;
#[cfg(feature = "serde")]
pub mod trades;
#[cfg(feature = "serde")]
pub mod ws;

/// Broad import surface for callers that want the DTO crate's old flat API.
#[cfg(feature = "serde")]
pub mod prelude {
    pub use super::auth::{
        AuthResponse, ChallengeResponse, CounterHashResponse, JwksResponse, PrivyLoginRequest,
        RefreshRequest, ServiceChallengeRequest, ServiceLoginRequest, WalletLoginRequest,
        WalletNonceQuery, WalletNonceResponse, WalletTransactionChallengeRequest,
        WalletTransactionChallengeResponse, WalletTransactionLoginRequest,
    };
    pub use super::candles::{
        ApiCandle, ApiCandleV2, CandleData, CandlesQueryParams, CandlesV2CursorQueryParams,
        CandlesV2InitialQueryParams, CandlesV2Page, CandlesV2QueryParams, CandlesV2Response,
        Timeframe,
    };
    pub use super::core::{Decimal, PaginatedResponse, Price, Side};
    pub use super::exchange::{
        AuthoritySetView, CollateralAssetMetadata, CollateralAssetsResponse, ExchangeKeysView,
        ExchangeLeverageTier, ExchangeMarketConfig, ExchangeResponse, ExchangeRiskFactors,
        ExchangeStatusView, ExchangeView, MarketCalendar, MarketPublicMetadata,
        MarketStatsSnapshot, SpotAssetConfig,
    };
    pub use super::exchange_ws::{
        AuthoritySet, CommodityMarketState, ExchangeDeltaMessage, ExchangeDeltaOp,
        ExchangeEncodedSnapshotMessage, ExchangeMarketParameterUpdate, ExchangeMarketSnapshot,
        ExchangeMessage, ExchangeSnapshotEncoding, ExchangeSnapshotMessage, ExchangeSnapshotReason,
        ExchangeSnapshotView, ExchangeStateSnapshot, ExchangeWsCommodityMetadata,
        ExchangeWsFeeConfig, ExchangeWsFundingConfig, ExchangeWsLeverageTier,
        ExchangeWsMarkPriceParameters, ExchangeWsMarketPriceBand,
        ExchangeWsRiskActionPriceValidityRules, ExchangeWsValidationRule,
    };
    pub use super::funding::{
        FundingHourlyEvent, FundingHourlyHistoryResponse, FundingHourlyQuery,
        FundingRateHistoryQuery, FundingRateHistoryResponse, FundingRatePoint,
    };
    pub use super::ix::{
        ApiAccountMeta, ApiInstructionResponse, CancelConditionalOrderRequest,
        CancelStopLossOrderRequest, ConditionalTriggerRequest,
        PlaceAttachedConditionalOrderRequest, PlaceIsolatedLimitOrderEnhancedResponse,
        PlaceIsolatedLimitOrderRequest, PlaceIsolatedLimitOrderWithConditionalsRequest,
        PlaceIsolatedMarketOrderEnhancedResponse, PlaceIsolatedMarketOrderRequest,
        PlacePositionConditionalOrderRequest, PlaceStopLossOrderRequest,
        StopLossExecutionDirection, TpSlOrderConfig,
    };
    pub use super::js_safe_ints::{JsSafeI64, JsSafeU64};
    pub use super::l2book::{L2Book, PriceLevel};
    pub use super::market::{
        L2BookUpdate, L2Orderbook, LeverageTier, MarkPriceResponse, MarketFeeConfig, MarketInfo,
        MarketStatsUpdate, MarketStatus, MarketSummary, MarketUnitConfig, MarketView, MarketsView,
        OrderbookView, RiskFactors, RiskState, RiskTier,
    };
    pub use super::market_state::Market;
    pub use super::market_stats::MarketStats;
    pub use super::next_commodity_market_transition::{
        CommodityMarketCalendarResponse, CommodityMarketCalendarView, CommodityMarketDaySchedule,
        CommodityMarketSessionRange, CommodityMarketStateView, NextCommodityMarketTransition,
    };
    pub use super::next_market_calendar_transition::{
        MarketCalendarDaySchedule, MarketCalendarHoursRange, MarketCalendarListResponse,
        MarketCalendarRecord, MarketCalendarResponse, MarketCalendarStateView,
        MarketCalendarSummary, MarketCalendarView, NextMarketCalendarTransition,
    };
    pub use super::service_accounts::{
        CreateServiceAccountRequest, CreateServiceKeyRequest, ServiceAccountDto,
        ServiceAccountKeyDto, ServiceAccountResponse, UpdateStatusRequest,
    };
    pub use super::trader::{
        CapabilityAccess, CooldownStatus, OrderHistoryDelta, TradeHistoryDelta,
        TraderCapabilitiesView, TraderStateCapabilities, TraderStateDelta,
        TraderStateLimitOrderEvent, TraderStateMarketLimitOrderEvent, TraderStatePayload,
        TraderStatePositionDelta, TraderStatePositionRow, TraderStatePositionSnapshot,
        TraderStateRowChangeKind, TraderStateServerMessage, TraderStateSnapshot,
        TraderStateSplineDelta, TraderStateSplineRow, TraderStateSplineSnapshot,
        TraderStateSpotCollateralSnapshot, TraderStateStopLossTrigger, TraderStateSubaccountDelta,
        TraderStateSubaccountSnapshot, TraderStateTakeProfitTrigger, TraderStateTickRegion,
        TraderStateTrigger,
    };
    pub use super::trader_http::{
        CollateralEvent, CollateralHistoryQueryParams, CollateralHistoryRequest,
        CollateralHistoryResponse, FundingHistoryEvent, FundingHistoryQueryParams,
        FundingHistoryResponse, LimitOrder, OrderHistoryItem, OrderHistoryQueryParams,
        OrderHistoryResponse, OrderStatus, PnlPoint, PnlQueryParams, PnlResolution, PnlResponse,
        SpotCollateralBalanceView, TraderActivityState, TraderPositionView, TraderStateResponse,
        TraderView, UserLiquidationHistoryKind, UserLiquidationHistoryPoint,
        UserLiquidationHistoryQueryParams, UserLiquidationHistoryResponse,
        UserLiquidationHistoryRole, UserLiquidationHistoryType,
    };
    pub use super::trades::{
        LiquidityRole, TradeEvent, TradeHistoryItem, TradeHistoryQueryParams, TradeHistoryResponse,
        TradeType, TradesMessage, TradesSubscriptionRequest,
    };
    pub use super::ws::{
        AllMidsData, CandlesSubscriptionRequest, ClientMessage, ErrorMessage,
        ExchangeSubscriptionRequest, FundingRateMessage, FundingRateSubscriptionRequest,
        MarketSubscriptionRequest, OrderbookSubscriptionRequest, ServerMessage,
        SubscriptionConfirmedMessage, SubscriptionErrorMessage, SubscriptionRequest,
        SubscriptionStatusMessage, TraderStateSubscriptionRequest,
    };
}

/// Deprecated module for backwards compatibility.
///
/// Use the specific modules instead:
/// - [`core`] for `Decimal`, `Price`
/// - [`market`] for market config and status types
/// - [`exchange`] for `ExchangeKeysView`, `AuthoritySetView`
#[cfg(feature = "serde")]
#[deprecated(
    since = "0.2.0",
    note = "Use specific modules instead: core, market, exchange"
)]
pub mod http {
    pub use crate::core::{Decimal, Price};
    pub use crate::exchange::{AuthoritySetView, ExchangeKeysView};
    pub use crate::market::{
        L2Orderbook, LeverageTier, MarketFeeConfig, MarketInfo, MarketStatus, MarketSummary,
        MarketUnitConfig, MarketView, MarketsView, RiskFactors, RiskState, RiskTier,
    };
}
