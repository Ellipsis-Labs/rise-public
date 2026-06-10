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
