# Changelog

Entries are drafted by Phoenix Rise sync PRs. Review and edit each
entry in this repo before merging.

## v0.1.5 - 2026-06-10

Source Phoenix commit: `f0c4a154e63048a8517e72e8a6a8b806e3768cd0`

- Package: `phoenix-rise`
- Target repo version: 0.1.4
- Phoenix version: 0.1.2 -> 0.1.5

### Summary

- **Delegated trader onboarding**: New `OnboardTraderDelegatedParams` / `create_onboard_trader_delegated_ix` and `get_permission_address` PDA helper are now re-exported from the crate root, enabling a delegated authority to onboard traders without requiring the trader's own signature.
- **Market calendar APIs**: Three new `PhoenixHttpClient` methods — `get_next_market_calendar_transition(symbol)`, `get_market_calendar(symbol)`, and `get_commodity_market_calendar()` — backed by new types (`NextMarketCalendarTransition`, `MarketCalendarResponse`, `CommodityMarketCalendarResponse`, and supporting schedule types).
- **Market public metadata**: `ExchangeMarketConfig` and `ExchangeMarketSnapshot` now carry an optional `metadata: Option<MarketPublicMetadata>` field (name, description, logo URI, CoinGecko ID, etc.). Live updates arrive via a new `ExchangeDeltaOp::MarketMetadataUpdated` delta op and can be queried from `PhoenixExchangeCacheStore` via `market_metadata*` helpers.
- **Forward-compatible exchange delta deserialization**: `ExchangeDeltaOp` and `ExchangeMarketParameterUpdate` now deserialize unrecognized server variants as `Unknown` (via `#[serde(other)]`) instead of returning a deserialization error.
- **Auth hardening**: Access-token expiry is now derived from the JWT `exp` claim (with server-supplied `expires_in` as fallback); expired tokens are omitted from refresh request `Authorization` headers; `session_missing` server error is now treated as `InvalidRefreshToken`.
- **MSRV lowered** from 1.86.0 to 1.84.0.

### Breaking Changes

- **`ExchangeMarketConfig` and `ExchangeMarketSnapshot` gained a new `metadata` field.** Any code that constructs these structs with named-field syntax (including exhaustive struct patterns) must add `metadata: None`.
- **`ExchangeWsMarkPriceParameters` gained two new fields** (`book_hard_stale_multiplier: u8`, `oracle_hard_stale_multiplier: u8`). Named-field construction must add both (both default to `0` on deserialization).
- **`ExchangeDeltaOp` and `ExchangeMarketParameterUpdate` gained `Unknown` variants; `ExchangeCacheMarketChangeKind` gained `Metadata`.** Exhaustive `match` arms on any of these enums will fail to compile and must add a wildcard or explicit `Unknown`/`Metadata` arm.
- **`ServiceAccountDto` gained `description: Option<String>`.** Named-field struct construction must add this field.

### Consumer Notes

- The new `PhoenixHttpClient::get_json` / `get_json_with_query` methods provide a typed escape hatch for endpoints not yet covered by named helpers.
- `CommodityMarketStateView` and `CommodityMarketDaySchedule`/`CommodityMarketHoursRange`/`CommodityMarketCalendarView`/`CommodityMarketCalendarResponse` are newly exported from the crate root, completing the commodity-calendar surface.
- The repository URL in Cargo metadata has changed from `Ellipsis-Labs/rise-public` to `Ellipsis-Labs/rise`; update any documentation links accordingly.

## v0.1.6 - 2026-06-11

Source Phoenix commit: `443ff8dfd64a9e7d35960b8b1946b3248d6681e5`

- Package: `phoenix-rise`
- Target repo version: 0.1.5
- Phoenix version: 0.1.5 -> 0.1.6

### Summary

- **New:** `PhoenixHttpClient` and `TradesClient` now expose `get_user_liquidation_history()`, hitting `GET /v1/users/{authority}/liquidation-history`. Use `UserLiquidationHistoryQueryParams` (with builder methods `with_pda_index`, `with_subaccount_index`, `with_symbol`, `with_limit`, `with_cursor`) to filter results.
- **New types re-exported from crate root:** `UserLiquidationHistoryPoint`, `UserLiquidationHistoryResponse`, `UserLiquidationHistoryKind`, `UserLiquidationHistoryType`, `UserLiquidationHistoryRole`.
- **New optional basis-point fields on `ExchangeRiskFactors`:** each existing `f64` percentage field (`maintenance`, `backstop`, `high_risk`, `upnl`, `upnl_for_withdrawals`, `cancel_order`) now has a companion `Option<u16>` `_bps` field. `ExchangeLeverageTier` gains `limit_order_risk_factor_bps: Option<u16>`. All are `#[serde(default, skip_serializing_if = "Option::is_none")]`; existing deserialization is unaffected.
- **WebSocket parameter-update events extended:** `CancelRiskFactorUpdated`, `UpnlRiskFactorUpdated`, and `UpnlRiskFactorForWithdrawalsUpdated` variants of `ExchangeMarketParameterUpdate` now carry optional `previous_bps`/`new_bps` fields (additive, backward-compatible).

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- **Prefer `_bps` fields for risk factor arithmetic.** The server now populates them (e.g., `maintenance_bps: Some(5000)` = 50%). When present, these should be considered authoritative; the legacy `f64` fields may carry basis-point-scale values from the API rather than percentages in some contexts — divide by 100 only for human-readable display, as shown in the updated `http_client` example.
- `ExchangeRiskFactors` struct literals in your own code now require the six new `Option<u16>` fields; either supply `None` or use `..Default::default()` (if you derive `Default`) to stay forward-compatible.
