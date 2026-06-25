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

## v0.1.7 - 2026-06-16

Source Phoenix commit: `65fc5aad3e13c249bcc606b410059b8adbc61e2d`

### Summary

- **Delegated market orders**: New `MarketOrderDelegatedParams` / `create_place_market_order_delegated_ix` instruction builder and `PhoenixTxBuilder::place_market_order_delegated` for signing via a wallet that differs from the trader account authority. Delegated market-order instructions are now routable through `PhoenixFlightClient`.
- **Session manager abstraction**: New `PhoenixSessionManager` trait with two built-in implementations — `PhoenixMemorySessionManager` (any `AuthSession`) and `PhoenixWalletSessionManager` (Solana keypair login, `solana-keypair` feature). Concurrent refreshes are coalesced. Pass via `PhoenixHttpClientBuilder::with_session_manager`.
- **Authenticated WebSocket handshake**: `PhoenixClient::new_from_env_with_auth` / `from_env_with_auth` / `from_env_with_http_client` propagate the HTTP client's auth lifecycle to WebSocket connections; the handshake now sends `Authorization` and `Sec-WebSocket-Protocol` JWT headers and auto-retries on 401/403/409.
- **WS connection status observable**: `PhoenixClient::connection_status()` and `subscribe_connection_status()` expose a `watch::Receiver<WsConnectionStatus>` for the full client's reconnection state.
- **`MaxLiquidationSizeUpdated` WS event**: New `ExchangeMarketParameterUpdate::MaxLiquidationSizeUpdated` variant tracked by `ExchangeCacheMarketChangeKind::MaxLiquidationSize`.

### Breaking Changes

- **`TradersClient` API replaced**: `get_trader(authority)`, `get_trader_internal(authority, pda_index)`, and `get_trader_state(authority, pda_index)` are removed. Use `get_trader_by_pubkey(&trader_pda)` instead, which takes the trader PDA directly. The convenience top-level methods `PhoenixHttpClient::get_traders` and `get_trader_state` are also removed.
- **`get_permission_address` removed** from the public re-export list in `phoenix_rise`.
- **New `ExchangeMarketParameterUpdate::MaxLiquidationSizeUpdated` variant**: exhaustive `match` arms over this enum must add a handler for the new variant.
- **New `PhoenixWsError::Authentication` variant**: exhaustive `match` arms over this error type must add a handler.

### Consumer Notes

- `PhoenixEnv::from_api_url(api_url)` is a new convenience constructor that derives the WebSocket URL automatically, including preserving reverse-proxy path prefixes (e.g. `https://gateway.example.com/phoenix` → `wss://gateway.example.com/phoenix/v1/ws`).
- `PhoenixHttpClient::from_url_with_wallet_keypair(url, keypair)` (`solana-keypair` feature) creates a fully authenticated client in one step.
- `PhoenixHttpClient::post_json` is now public.
- `is_auth_recovery_error` is now a public export for downstream error classification.
- `async-trait 0.1` is a new direct dependency of `phoenix-rise`.

## v0.1.8 - 2026-06-16

Source Phoenix commit: `6487be852f3a382717f345a43fd956af3fba1766`

### Summary

- Upgraded `rand` from 0.9 to 0.10 (now a workspace dependency); the `rand::Rng` trait import in math tests is replaced by `rand::RngExt`.
- Rate-limit retry logic now applies ±15% jitter to the fallback delay when no `Retry-After` header is present, reducing thundering-herd retries.
- `Retry-After` header parsing now accepts both integer seconds and HTTP-date (`RFC 2822`) values; previously only integer seconds were recognized.
- The `max_delay` field on `RateLimitRetryConfig` now bounds only the jittered fallback delay; explicit `Retry-After` values are bounded by `max_total_wait` instead (doc clarification, behavior change for large `Retry-After` values).

### Breaking Changes

- **`rand` 0.10 workspace dependency**: if your crate depends on `phoenix-rise` and also uses `rand`, you may need to align to `rand = "0.10"`. The `rand::Rng` trait is replaced by `rand::RngExt` in 0.10 — any code importing `rand::Rng` via a shared dependency tree will need updating.
- **`RateLimitRetryConfig::max_delay` semantics changed**: previously capped all retry delays including explicit `Retry-After` header values; now only caps the jittered fallback delay. If you set a small `max_delay` to bound `Retry-After`-driven waits, use `max_total_wait` instead.

### Consumer Notes

- No changes to public API surface beyond the `RateLimitRetryConfig` doc/semantic fix above.
- The jitter on fallback delays (±15%) is applied automatically; no configuration required or available.
- If your server sends `Retry-After` as an HTTP-date string, the client will now correctly parse and honor it.

## v0.1.9 - 2026-06-16

Source Phoenix commit: `00d34904a9c9e981dcc62666906e2985a98624d9`

### Summary

- **New `opentelemetry` feature flag**: adds opt-in W3C trace-context propagation for outbound HTTP requests and WebSocket handshakes. Enable with `phoenix-rise = { features = ["opentelemetry"] }`. When active, `PhoenixHttpClientBuilder` exposes `with_trace_context_provider(...)` and `PhoenixWSClient` gains `new_with_trace_context_provider` and `new_with_connection_status_and_trace_context_provider` constructors.
- **`TransferCollateralParamsBuilder` gains an optional `permission_account`**: pass a secondary position-authority permission account via `.permission_account(pubkey)`. When set, it is appended as a writable account at the end of the instruction's account list.
- **`PlaceMarketOrderDelegated` removed from Flight routing**: `is_flight_routable_instruction` now returns `false` for delegated market orders. The discriminant function remains public but is no longer matched by the Flight client.
- Dependency updates: `proptest` → `1.11.0`, `getrandom` → `0.4.2`, and several transitive `windows-sys` pins resolved to earlier versions.

### Breaking Changes

- **`is_flight_routable_instruction` behavior change**: `PlaceMarketOrderDelegated` instructions are no longer considered flight-routable. If your code wrapped delegated market orders via `PhoenixFlightClient::try_wrap_order_instruction`, those instructions must now be submitted directly rather than through Flight.

### Consumer Notes

- The `opentelemetry` feature does not install a global propagator automatically — callers must call `opentelemetry::global::set_text_map_propagator(...)` (e.g., with `TraceContextPropagator`) before trace headers will be injected into requests.
- The `permission_account` builder method is additive and optional; existing `TransferCollateralParams` construction code requires no changes.

## v0.1.10 - 2026-06-17

Source Phoenix commit: `e01d6bf79f112b8f9c7c6e7e3ad84fb83050eb43`

### Summary

- Added the **Phoenix Hawkeye** read-only view layer: instruction builders, typed return-data structs, a decode pipeline, and a high-level async RPC client for querying margin, per-asset risk, liquidation prices, BBO, and funding state via simulated transactions.
- New top-level exports from `phoenix_rise`: `PhoenixHawkeyeClient`, `HawkeyeSimulation<T>`, `PhoenixHawkeyeClientError`, `HawkeyeReturnData`, `HawkeyeReturnDataError`, all `View*Return` structs, `Hawkeye*ViewAccounts` account-list structs, low-level instruction builders (`create_hawkeye_view_*_ix`), and decode helpers (`decode_hawkeye_return_data`, `decode_hawkeye_return`).
- `PhoenixTxBuilder` gains nine new `build_hawkeye_view_*` methods that construct Hawkeye simulation instructions from a `PhoenixMetadata` context, with both PDA-derived and explicit-trader variants.
- Three new mandatory runtime dependencies: `solana-rpc-client-api ~2.3`, `solana-transaction-error ~2.2`, and `solana-transaction-status-client-types ~2.3`; `solana-compute-budget-interface ~2.2` is also added. Downstream `Cargo.toml` files that pin Solana crate ranges tightly may need adjustments.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `PhoenixHawkeyeClient::new(rpc_url, &metadata)` is the recommended entry point; it defaults to an unsigned simulation fee payer (`HAWKEYE_SIMULATION_FEE_PAYER`) and a 1.4M CU limit, both overridable via builder methods.
- All Hawkeye view calls are **read-only simulations** — no fee payer signature required and no state is mutated on-chain.
- Return data is versioned (`HAWKEYE_RETURN_VERSION = 1`); callers should assert `version` matches if they hard-code struct layouts.
- `ViewBboReturn::best_bid_ticks()` / `best_ask_ticks()` return `Option<u64>` based on presence flags — a `None` means no resting orders on that side, not an error.

## v0.1.11 - 2026-06-22

Source Phoenix commit: `fd9d044ad0e76e6bcbef1333b1ebc8648f511b7a`

### Summary

- **Flight per-order fee override**: `ProxyInstructionParams::builder()` now accepts `.fee_bps_override(bps: u64)` (0–10 000). When set, `create_proxy_instruction_ix` emits the `proxy_instruction_with_fee_override` variant with a different discriminant and a Borsh `Option<u64>` prefix before the inner instruction data. `PhoenixFlightClient` gains a matching `try_wrap_order_instruction_with_fee_bps_override(ix, trader_wallet, fee_bps_override: Option<u64>)` method; the existing `try_wrap_order_instruction` is unchanged and delegates to it with `None`.
- **WebSocket subscription event channel**: `WsSubscriptionEvent` is now a public export. Call `client.subscription_event_receiver()` (once) to receive `Status` and `Error` lifecycle events for every subscription acknowledgement or rejection. New `PhoenixWSClient::new_with_connection_status_and_auth` constructor combines authenticated and connection-status modes.
- **Automatic auth reconnect on WS close**: The client now detects auth-related close codes (4401, 4403, 1008) and reason strings (`access_token_expired`, `invalid_access_token`, etc.), refreshes the session, reconnects, and resubmits active subscriptions transparently.
- **`ExchangeMarketConfig` gains `stats_snapshot: Option<MarketStatsSnapshot>`**: The new field is `#[serde(default)]` so deserialization of existing payloads is unchanged, but Rust struct literal construction must now include `stats_snapshot: None`.
- **New `JsSafeI64` type** added to `phoenix_rise::types::js_safe_ints` (mirrors `JsSafeU64` for signed values; serialises as a decimal string).

### Breaking Changes

- `AdminChallengeRequest` and `AdminLoginRequest` are removed from `phoenix_rise::types::auth`. Code referencing these types will fail to compile.
- `CreateServiceAccountRequest.role` and `ServiceAccountDto.role` fields are removed. Struct construction and pattern matching on these types must be updated.
- `ExchangeMarketConfig` requires a new `stats_snapshot` field for struct literal construction (e.g. in tests). Add `stats_snapshot: None` to any existing struct literals.

### Consumer Notes

- The `fee_bps_override` builder method validates the range at `build()` time and returns `PhoenixIxError::InvalidFeeBpsOverride` for values above 10 000. Pass `None` (or omit the call) to continue using the builder's registered fee.
- `subscription_event_receiver()` takes ownership of the channel and can only be called once per client instance; subsequent calls return `None`.
- `ServerMessage` now has a `SubscriptionStatus(SubscriptionStatusMessage)` variant — match arms with `ServerMessage::Other` that previously caught subscription-status frames will now route through the new variant instead.

## v0.1.12 - 2026-06-22

Source Phoenix commit: `a419e23d7d1b2a3e37696d76e85fac7f0a023f5e`

### Summary

- Three new cancellation/maintenance helpers added to `PhoenixTxBuilder`: `build_cancel_all_orders`, `build_cancel_up_to`, and `build_uncross_crank` (the crank is permissionless — no trader signer required).
- Matching low-level params types and instruction constructors exported from `phoenix_rise_ix`: `CancelAllParams`/`create_cancel_all_ix`, `CancelUpToParams`/`create_cancel_up_to_ix`, `UncrossCrankParams`/`create_uncross_crank_ix`.
- New granular Cargo features: `sdk`, `types`, and `test-fixture`. The default feature set changed from `[]` to `["sdk"]`.
- New `test-fixture` feature exposes `phoenix_rise::test_fixture` with `LiteSVM`-backed `SdkLocalnetContext`, fixture deserialization types, and a bundled `default-localnet.json` for in-process localnet tests.
- `ix` program constants (`PHOENIX_PROGRAM_ID`, `PHOENIX_LOG_AUTHORITY`, `PHOENIX_GLOBAL_CONFIGURATION`) now compile under `target_os = "solana"`, making the `ix` sub-crate usable in on-chain programs.

### Breaking Changes

- **Default features changed from `[]` to `["sdk"]`**: consumers who previously added `phoenix-rise` without `default-features = false` had near-zero transitive deps; they now pull in the full SDK set (`reqwest`, `tokio`, `parking_lot`, etc.). Add `default-features = false` to restore a minimal build.
- **`solana-keypair` and `ed25519-dalek` re-exports now also require `sdk`**: `PhoenixWalletSessionManager`, `PhoenixSolanaKeypairAuthSigner`, `default_solana_keypair_path`, and `PhoenixEd25519ServiceAuthSigner` are gated on `all(feature = "sdk", feature = "solana-keypair/ed25519-dalek")`. Consumers using `default-features = false` with only those features enabled will see missing-item compile errors; add `features = ["sdk"]` to restore the exports.

### Consumer Notes

- `rust_decimal = []` was previously an empty stub; it now activates `dep:rust_decimal`. The feature was always advertised but did nothing; it now pulls in the crate.
- `utoipa` and `opentelemetry` features now implicitly enable `types`/`sdk` respectively, so enabling them without `default-features = false` is safe but may add transitive deps if you were relying on those features being lighter.
- The `test-fixture` feature is intended for test harnesses only; it adds `litesvm 0.7` and `solana-commitment-config` as optional deps and is not covered by the `sdk` default.

## v0.1.13 - 2026-06-23

Source Phoenix commit: `d1ccda811fc68e0cbf1fbf5d3e6d0235d508c3db`

### Summary

- The `phoenix-rise` crate was consolidated from a multi-root workspace layout (`sdk/src/`, `ix/src/`, `math/src/`, `types/src/`) into a standard single-crate layout (`src/`). Public module paths (`phoenix_rise::ix`, `::math`, `::types`) are unchanged; this is a transparent restructure for existing crate consumers.
- New instruction builders for permission account management: `create_permission_ix` and `set_permission_delegated_ix`, with supporting constants `TRADER_ONBOARDING_PERMISSION` and `TRADER_MANAGEMENT_PERMISSION` (in `phoenix_rise::ix`).
- New trader capability constants exported from `phoenix_rise::ix`: `TRADER_CAPABILITY_HOT`, `TRADER_CAPABILITY_CAN_PLACE_LIMIT`, `TRADER_CAPABILITY_CAN_PLACE_MARKET`, `TRADER_CAPABILITY_CAN_RISK_INCREASE`, `TRADER_CAPABILITY_CAN_DEPOSIT`, `TRADER_CAPABILITY_CAN_WITHDRAW`, and `REQUIRED_TRADER_CAPABILITIES`. Also adds `is_trader_ready` helper.
- New `delegated_trader_management_onboarding` example demonstrating the full off-chain integrator flow for granting and consuming delegated trader-onboarding permissions.

### Breaking Changes

- If you depended on `phoenix-rise-types` as a standalone crate, it is no longer published. Switch to `phoenix_rise::types` (requires the `types` feature flag on `phoenix-rise`).

### Consumer Notes

- Update your `Cargo.toml` to `phoenix-rise = "0.1.13"`.
- Delegated onboarding flows that previously required manual capability bitmask construction can now use the exported `TRADER_CAPABILITY_*` constants and `is_trader_ready` to check readiness without hardcoding bit values.
- The new `permission` instruction builders (`create_permission_ix`, `set_permission_delegated_ix`) are needed for integrators who programmatically manage delegated trader-onboarding budgets; see the `delegated_trader_management_onboarding` example for the end-to-end flow.

## v0.1.14 - 2026-06-24

Source Phoenix commit: `f2a0ac7b66eae85ef8ddb3f21ff68ce6f7063754`

### Summary

- Added two new unauthenticated referral activation endpoints to `InviteClient`: `get_referral_activation_permission()` (`GET /v1/referral/activation-permission`) and `activate_referral_tx()` (`POST /v1/referral/activate-tx`), enabling a wallet-side transaction-signing flow for referral activation.
- New public types `ActivateReferralTxRequest`, `ActivateReferralTxResponse`, and `ReferralActivationPermissionResponse` are re-exported from the crate root under the `sdk` feature flag.
- Added `ExchangeStatusView` struct (fields: `active`, `gated`, `withdrawals_available`) and a corresponding `ExchangeClient::get_status()` method (`GET /exchange/status`).
- Added `withdrawals_available: bool` field to `ExchangeStateSnapshot` and the `ExchangeStatusChanged` variant of `ExchangeDeltaOp`; both default to `true` when the field is absent from the wire payload (backward-compatible deserialization).
- Added a `referral_activation_tx` example (requires `solana-keypair` feature) demonstrating the full end-to-end activate-tx wallet integration flow.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- If you pattern-match on `ExchangeDeltaOp::ExchangeStatusChanged`, you must now bind or ignore the new `withdrawals_available` field; omitting it is a compile error.
- `ExchangeStateSnapshot` gains a `withdrawals_available` field; struct literal construction (if any) must be updated to include it.
- `ExchangeStatusView` is a new type exported from `phoenix_rise_types`; no migration needed unless a local type of the same name exists.
- The `activate_referral_tx` flow requires the trader account to exist on-chain before calling `/v1/referral/activate-tx`; use `--register-if-missing` (or call `register_trader` first) if the account may not exist yet.

## v0.1.15 - 2026-06-25

Source Phoenix commit: `823bb5e64efafb221f37a7e0a8f8720da843741c`

### Summary

- Added new public exports for inspecting trader activation status before building a referral activation transaction: `ReferralActivationTraderStatus` enum, `ReferralActivationTraderStatusError` error enum, `fetch_referral_activation_trader_status` async function, `has_referral_activation_capabilities` const function, and `referral_activation_trader_status` function.
- The referral activation transaction now handles missing trader accounts automatically: when the trader account does not exist, the built transaction includes `register_trader` followed by delegated onboarding in a single atomic transaction. No separate registration step is required.
- `ReferralActivationTraderStatus::should_include_register_trader()` returns `true` only for the `Missing` variant, making it straightforward to conditionally include the registration instruction when constructing activation transactions manually.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- The `referral_activation_tx` example no longer requires `--register-if-missing`; the flag is accepted but silently ignored for backward compatibility. Remove it from any example invocations you have documented.
- If you call `fetch_referral_activation_trader_status` and receive `ReferralActivationTraderStatus::Missing`, pass `should_include_register_trader() == true` (and a valid `max_positions` in the range 32–128) to your transaction builder. Accounts in the `Registered` or `Activated` states skip registration and only set capabilities.
- `--max-positions` in the example binary now enforces a minimum of 32 (previously 1). This affects the example only and does not change the library API.
