# Changelog

Entries are drafted by Phoenix Rise sync PRs. Review and edit each
entry in this repo before merging.

## v0.5.2 - 2026-08-26

Source Phoenix commit: `bb97cff3b8d6ca1cb131fcdd43d2a8baa471c23a`

### Summary

- Adds native SOL spot collateral support across the SDK: new `accounts::spot_collateral`, `ix::native_sol` (sync/withdraw/transfer/swap/liquidate instructions), `math::spot_collateral` valuation, and new `SpotCollateral{Deposited,Withdrawn,Liquidated}` events.
- Adds TWAP/Flicker instruction builders (`ix::twap`, `FLICKER_PROGRAM_ID`, `TwapInstruction`) and scale/ladder orders via `PlaceMultiLimitOrderV2` / `CondensedOrderV2` / `MultiLimitOrderParamsV2`.
- Adds a candles v2 client (`CandlesClient::get_candles_v2`, cursor pagination, mark-price OHLC) alongside the unchanged legacy `get_candles`.
- Adds `/v1/collateral/assets` (`CollateralClient::get_assets`) and a new exchange websocket channel (`ExchangeSubscriptionRequest` / `ExchangeMessage`) that streams spot-collateral and exchange-config updates into `PhoenixExchangeCacheStore` and `PhoenixMetadata`.
- `PhoenixFlightClient` is reworked to resolve the Phoenix root authority from a `SharedExchangeCacheStore` for position-authority (delegate-signed) order wraps, appending the collateral-transfer permission accounts when needed.

### Breaking Changes

- `PhoenixFlightClient::try_wrap_order_instruction(ix, signer)` now takes a third `use_position_authority: bool` parameter; `PhoenixFlightClient` is no longer `Copy` (only `Clone`) and gains a required exchange-cache-store field, so manual construction call sites need updating (prefer `PhoenixFlightClient::from_exchange_store(...)`).
- `RiskAction::Withdrawal` is replaced by `RiskAction::WithdrawQuoteCollateral` and a new `RiskAction::WithdrawSpotCollateral`; any downstream `match` on `RiskAction` must be updated. `RiskAction::ADL` is now deprecated.
- `ScalarBounds::lower_bound()` / `upper_bound()` now return `Self` instead of the inner numeric type.
- Instruction discriminants `SetExchangeStatusBits` and `DisableExchangeCapabilities` are removed; `PlaceMultiLimitOrderV2` and `AuthorizedTransferCollateral` are added. Code matching or serializing the old discriminants will break.
- `GlobalConfiguration`/`GlobalConfigPrefixRaw` grew new fields (`native_sol_spot_metadata`, `acknowledged_restart_slot`), and its on-chain prefix length changed from 776 to 1104 bytes — any code assuming the old fixed layout size will break.
- `Trader` gains new required fields (`disable_position_authority_swap`, `native_sol_collateral`); `TraderPortfolio` gains a required `spot_collaterals` field — struct literals constructing these without the new fields will no longer compile.

### Consumer Notes

- `usdc_mint()` resolves the active USDC mint from `PHOENIX_ENV` (adds beta support); the `USDC_MINT` constant still exists and defaults to mainnet.
- `PhoenixClientError` gained a `Metadata` variant, and `MarginTrigger` gained a `SpotCollateralsUpdated` variant — exhaustive matches on either enum need updating.
- `PhoenixIxError` gained new variants (`MissingRootAuthority`, `InvalidSwapAmount`, `InvalidWithdrawDestination`, `UnexpectedVenueSigner`, `TooManyVenueAccounts`) related to native SOL/swap instruction validation.
- The `builder_onboarding_tx` example's CLI flags changed: `--trader-keypair-path` is replaced by `--fee-payer-keypair-path` (defaults to `~/.config/solana/id.json`) and an optional `--trader-authority`, since the trader authority is now public-key-only and does not sign.

## v0.3.4 - 2026-07-08

Source Phoenix commit: `6051225fb045fbb5b6a454bd445e7fc2e31e5722`

### Summary

- Reworked the per-market liquidation price calculation (`math`) to search for the actual tick boundary where an account becomes liquidatable, replacing the previous closed-form algebraic approximation. This correctly returns the current mark price when an account is already liquidatable and properly accounts for target-market limit-order maintenance instead of treating it as a fixed outside term.
- Added a new projected liquidation price API — `projected_liquidation_price`, `ProjectedLiquidation`, `ProjectedLiquidationFill`, `ProjectedLiquidationParams`, and `TraderPortfolioMargin::projected_liquidation_prices` — that simulates a trader's own position-side resting orders filling along the adverse price path, for risk/explainer displays distinct from the static Hawkeye-compatible liquidation price.
- Added `initial_margin_for_asset_with_mark_price` and `calculate_liquidation_price_usd_with_target_limit_order_maintenance` as supporting public entry points for callers that need to supply an explicit mark price or an additional target-market limit-order maintenance term.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `calculate_liquidation_price_usd` keeps its existing signature and default behavior (it now delegates to the new maintenance-coefficient variant with `0.0`), so existing callers are unaffected.
- Liquidation prices from portfolio margin calculations are now solved as an exact tick boundary rather than a closed-form approximation, so returned values may shift slightly (now quantized to the market's tick size) and should be more accurate, particularly for accounts with resting orders in the target market.
- Consumers wanting "what if my resting orders fill before liquidation" estimates can adopt `TraderPortfolioMargin::projected_liquidation_prices` alongside the existing static liquidation price fields; it's an additive risk estimate, not a replacement for the static value.

## v0.3.3 - 2026-07-08

Source Phoenix commit: `25625a376965069d216ba53f8bbf0457f097a927`

### Summary

- `math`: `LimitOrderMarginState` now tracks best bid/ask prices and reduce-only order size separately, and initial margin calculations reserve additional collateral for the mark-to-market loss a resting order would incur if it filled at its limit price (adverse fill loss), including for reduce-only orders.
- `math`: Reduce-only order sizes are now capped to the lots that can actually reduce the trader's existing position before they factor into margin.
- `math`: Added a `is_zero()` helper on the numeric wrapper types generated by the shared `basic_num!` macro (e.g. `BaseLots`, `SignedBaseLots`, `QuoteLots`).

### Breaking Changes

- `LimitOrderMarginState::new(...)` has been removed. Construct margin state via the new `LimitOrderMarginState::from_orders(orders, position)`, which takes an iterator of the new `LimitOrderSummary` type plus the trader's current signed base-lot position.
- `LimitOrderMarginState` gained new fields (`lowest_ask`, `highest_bid`, `total_reduce_only_ask_base_lots`, `total_reduce_only_bid_base_lots`) and can no longer be built with a struct literal (an internal marker field enforces construction through `from_orders`/`empty`). Any code building this struct directly will need to switch to the canonical constructors.
- `LimitOrder::aggregate_margin_state(orders)` now requires an additional `position: SignedBaseLots` argument.
- Margin requirements can increase for accounts with resting limit orders priced away from mark (bids above mark / asks below mark), since margin now reserves the potential adverse-fill loss on those orders — including reduce-only orders, which previously contributed no margin at all.

### Consumer Notes

- New public type `LimitOrderSummary` (side, price, remaining size, reduce-only flag) is the input to `LimitOrderMarginState::from_orders`.
- New accessors `LimitOrderMarginState::total_ask_base_lots()`, `total_bid_base_lots()`, `lowest_ask()`, and `highest_bid()` are available for inspecting aggregated order state.
- If you compute margin off this crate, re-verify expected collateral values for accounts with resting orders away from mark price, since totals may now be higher than before.

## v0.3.2 - 2026-07-07

Source Phoenix commit: `84c3feb1a2f5d9b4e94f9372a706b0e3e3c88b0e`

### Summary

- Added a margin/liquidation simulation API to `math`: `TraderPortfolio::simulate_position_fill`, `simulate_margin`, and `simulate_margin_scenarios`, along with supporting types (`MarginSimulationAction`, `SimulateMarginParams`, `SimulateMarginScenariosParams`, `MarginScenario`, `SimulatedMargin`, `SimulatedPositionFill`, `SimulatedMarginScenarios`, etc.) for projecting fills, order changes, funding, and mark-price moves against cross or isolated margin.
- Added a standalone closed-form liquidation price solver, `calculate_liquidation_price_usd`, with its `CalculateLiquidationPriceUsdInput` input struct.
- Fixed the sign of unsettled funding in `math` margin calculations so a positive value means funding owed *to* the trader and a negative value means funding owed *by* the trader, matching the on-chain contract's `calculate_funding_payment` semantics.

### Breaking Changes

- The unsettled-funding sign fix changes the numeric sign of `unsettled_funding` returned from portfolio and position margin calculations in `math`; consumers reading this field must update their sign handling.
- `PhoenixStateError` gained a new variant, `InvalidMarginSimulationInput`; downstream exhaustive matches on this enum need an added arm.

### Consumer Notes

- Use the new simulation API to preview the margin and liquidation-price impact of fills, order placement/cancellation, collateral adjustments, funding settlement, and mark-price moves before submitting transactions.
- `simulate_margin_scenarios` lets you branch several labeled scenarios off one shared baseline for efficient side-by-side what-if comparisons.
- Isolated-margin simulations require all actions to target a single symbol; mixing symbols now errors with `InvalidMarginSimulationInput`.

## v0.3.1 - 2026-06-30

Source Phoenix commit: `cf8419bc21f9f539306198f7c08d0aca14a39580`

### Summary

- Adds trader preference bits (initially `disable_collateral_sweep`) to trader registration/account state, plus a new zero-copy `accounts::orderbook` view for reading FIFO best bid/ask and book sides directly from account bytes.
- Reworks numeric fields across `accounts` views (orderbook, perp asset map, trader, stop-losses, conditional orders, spline collection, withdraw queue) from raw integers to typed quantity wrappers (`Ticks`, `BaseLots`, `QuoteLots`, `SignedBaseLots`, `SignedQuoteLots`, etc.).
- Unifies the `Side` enum on a single `phoenix-rise-math::Side` shared by `ix`, `events`, and `types`, removing duplicate per-crate definitions.
- Adds a `solana-signature` dependency to `api`/`litesvm-test` and bumps the shared Rise Rust crates to `0.3.1`.

### Breaking Changes

- `Side` now serializes as lowercase `"bid"`/`"ask"` (previously `"Bid"`/`"Ask"`); deserialization still accepts the old PascalCase as an alias, but consumers matching on serialized output must update.
- Account-view fields that were previously raw `u64`/`i64`/`i128` (prices, lot sizes, funding fields, withdraw-throttle budgets, etc.) are now typed wrappers — call `.as_inner()` instead of using them as plain integers.
- CPI `RegisterTraderArgs` and ix `RegisterTraderParams` gained a new `trader_preference_bits: u32` field; existing struct literals must add it (or switch to the new `RegisterTraderArgs::new(...)` constructor/builder methods).
- `trader::Trader` is now the primary zero-copy trader view; some position count/capacity accessors moved off `TraderHeader` onto `Trader`, and owned `Trader.max_positions` changed from `u64` to `u32`.

### Consumer Notes

- Use `accounts::trader::Trader::try_from_account_bytes` for a combined header+positions view, and the new `trader::preferences` helpers (`TraderPreferenceKind`, `TraderPreferences`, `TraderPreferenceFlags`) to read/set `disable_collateral_sweep`.
- Build preference-aware registrations with `RegisterTraderArgs::new(...).with_disable_collateral_sweep(true)` (or `.with_trader_preference(...)`) instead of hand-assembling the bitfield.
- The new `accounts::orderbook::Orderbook` view covers FIFO limit orders only (no spline liquidity); use Hawkeye's BBO view when full market liquidity is needed. `accounts::permission::PermissionAccount::try_from_buffer` adds a zero-copy alternative to the existing copying decoder.

## v0.3.0 - 2026-06-29

Source Phoenix commit: `08743d17a258f1f4d6e744bd0dc5d64bc0a9d580`

### Summary

- Bumps all `phoenix-rise` workspace crates from `0.2.0` to `0.3.0`.
- Upgrades all `solana-*` workspace dependencies from the `~2.x` range to `~3.x` (e.g. `solana-pubkey ~2.4` → `~3.0`, `solana-rpc-client ~2.3` → `~3.1`).
- Pins `litesvm` at `=0.11.0` (up from `0.7`) and adds the `borsh` feature to the workspace `solana-pubkey` dependency.

### Breaking Changes

- **Solana SDK `~2.x` → `~3.x` (all components)**: Every `solana-*` dependency across `accounts`, `ix`, `core`, `sdk`, `workspace`, and example `programs` has been bumped to the `~3.x` range. Downstream crates that pin any `solana-*` dependency to `~2.x` will encounter dependency-resolution conflicts and must be updated.
- **`core` — `SimulationFailed` error variant inner type changed**: `PhoenixHawkeyeClientError::SimulationFailed` now stores a converted (`err.into()`) type rather than the raw Solana `~2.x` error, reflecting a type change in the Solana 3.x RPC API. Any `match` arm destructuring the inner value must be updated to the new Solana 3.x error type.
- **`solana-pubkey` gains `borsh` feature**: The workspace `solana-pubkey` dependency now enables `borsh` in addition to `curve25519` and `serde`. This pulls in `borsh` transitive dependencies for all crates that share the workspace pin; crates that previously relied on `borsh` being absent from this dep tree may see new transitive crate additions.

### Consumer Notes

- Update all `solana-*` entries in your `Cargo.toml` or workspace to the `~3.x` range to stay compatible with this release.
- The minimum supported Rust version for example programs is now `1.89.0` (edition `2024`); the main workspace `rust-version` was already `1.89.0`.
- `litesvm` is now pinned at `=0.11.0`. Test helpers that seed accounts via `airdrop` must use at least `890_880` lamports (the rent-exempt minimum for an empty account) rather than `1`; the newer `litesvm` enforces rent-exemption requirements that the prior version did not.
- The `events` source-parity tests comparing `phoenix-rise-events` against the internal Phoenix exchange source have been removed. Serialization compatibility is still covered by `representative_event_serialization_matches_phoenix_exchange`.

## v0.2.0 - 2026-06-29

Source Phoenix commit: `d087e01780d6f8cfadb10005c6607f7de59d3de2`

### Summary

- **Rust SDK split into focused crates**: `phoenix-rise` is now a facade over
  dedicated crates: `phoenix-rise-accounts`, `phoenix-rise-api`,
  `phoenix-rise-core`, `phoenix-rise-events`, `phoenix-rise-ix`,
  `phoenix-rise-math`, `phoenix-rise-types`, and
  `phoenix-rise-litesvm-test`. The facade still imports as `phoenix_rise`, but
  the implementation and feature graph are now organized by use case.
- **On-chain program support is now first-class**: `phoenix-rise` exposes a
  minimal `cpi` feature profile for Solana programs that need borrowed account
  views, instruction layouts, and Pinocchio CPI helpers without pulling in the
  HTTP, WebSocket, RPC, DTO, or `solana-instruction` client stack.
- **Dedicated account decoding crate**: `phoenix-rise-accounts` provides
  borrowed account views, account discriminators, PDA helpers, and owned
  off-chain account readers under `phoenix_rise::accounts`.
- **Dedicated instruction crate**: `phoenix-rise-ix` provides raw Phoenix,
  Ember, Flight, and Hawkeye instruction builders, typed discriminants, return
  data helpers, and `ix::cpi` helpers.
- **Dedicated client and transaction crates**: `phoenix-rise-api` now owns the
  HTTP, WebSocket, auth, exchange-cache, Flight, and Hawkeye client surfaces;
  `phoenix-rise-core` owns account fetchers, order tickets, and
  `PhoenixTxBuilder`.
- **Dedicated test fixture crate**: LiteSVM localnet fixtures and helpers moved
  into `phoenix-rise-litesvm-test` so integration tests can opt in explicitly.
- **New `events` crate**: Market event parsing from Phoenix `Log` and
  `LogEventLengths` instruction payloads is available through
  `phoenix-rise-events` and the facade `events` feature.

### Breaking Changes

- **Feature flags were reorganized around the split crates**:
  - Default features are now `["api", "ws", "sdk", "tx-builder"]`. The default
    `phoenix-rise` dependency remains the full off-chain SDK bundle.
  - The old broad `core` feature is now a compatibility alias for
    `tx-builder`.
  - `sdk` now means account views, instruction builders, math, and domain
    types, without API transport or RPC-backed transaction helpers.
  - `api`, `ws`, `tx-builder`, `accounts`, `ix`, `math`, `events`, `types`,
    `types-sdk`, and `cpi` are now the main selection points for slim builds.
- **Minimal/off-chain feature selections may need updates**: if your
  `Cargo.toml` used `default-features = false`, add the explicit features that
  match your imports:
  - HTTP/auth/Flight clients: `features = ["api"]`
  - WebSocket clients: `features = ["ws"]`
  - `PhoenixTxBuilder`, order tickets, or account fetchers:
    `features = ["tx-builder"]`
  - Raw instruction builders: `features = ["ix"]`
  - Account byte decoding: `features = ["accounts"]`
  - Math helpers: `features = ["math"]`
  - On-chain CPI/program usage: `features = ["cpi"]`
- **`PhoenixTxBuilder` moved behind `phoenix_rise::core` and `tx-builder`**:
  code that imported transaction-building types from the old monolithic SDK
  surface should import from `phoenix_rise::core::{PhoenixTxBuilder, ...}` and
  enable `tx-builder`.
- **HTTP, WebSocket, auth, Flight, and Hawkeye clients moved behind
  `phoenix_rise::api` and `api`/`ws`**: update imports such as
  `PhoenixHttpClient`, `PhoenixWSClient`, `PhoenixClient`, auth signers, and
  Flight helpers to use `phoenix_rise::api::{...}` when relying on the facade.
- **Low-level account imports changed**: borrowed account views are under
  modules such as `phoenix_rise::accounts::trader`,
  `phoenix_rise::accounts::perp_asset_map`, and
  `phoenix_rise::accounts::global_config`; serde-friendly/materialized account
  readers are under `phoenix_rise::accounts::owned`.
- **LiteSVM fixture helpers are no longer part of the facade**:
  `phoenix_rise::test_fixture` was removed. Test code should depend on
  `phoenix-rise-litesvm-test` directly and import
  `SdkLocalnetContext`, `default_sdk_localnet_fixture`, and related helpers from
  that crate.
- **Standalone permission instruction module was removed from `ix`**: callers
  using the old `phoenix_rise::ix::permission::*` builders should migrate to
  the current delegated onboarding / permission flows exposed by the SDK
  examples and the account/PDA helpers in `phoenix_rise::accounts`.
- **`rust_decimal` is now a normal math dependency**: decimal price conversion
  support is no longer guarded by the old `rust_decimal` feature wiring, so
  minimal builds that enable math should account for that dependency.

### Consumer Notes

- Most off-chain applications can keep a single dependency:
  `phoenix-rise = "0.2"`. The facade re-exports the split crates through
  `phoenix_rise::{accounts, api, core, events, ix, math, types}` according to
  enabled features.
- On-chain programs should prefer the slim facade profile:
  `phoenix-rise = { version = "0.2", default-features = false, features = ["cpi"] }`.
  Direct dependencies on `phoenix-rise-accounts` and `phoenix-rise-ix` are also
  supported when you want only those crates.
- API-only tools can use:
  `phoenix-rise = { version = "0.2", default-features = false, features = ["api"] }`.
  Add `ws` for WebSocket support or `tx-builder` for local transaction
  construction.
- The old `delegated_trader_management_onboarding` example has been replaced by
  `sdk/examples/onboard_trader_delegated.rs`; CPI-oriented examples now live
  under `rise/programs/`.
- New `math::quantities::serde_numeric` helpers provide optional serde support
  for quantity types when the `serde` feature is enabled.

## v0.1.16 - 2026-06-25

Source Phoenix commit: `2f2bb2ba1256f28465278ff1c00d61f0ffc2c5f2`

### Summary

- Added builder-initiated trader onboarding support: two new `ExchangeClient` methods — `build_register_ixs` and `send_register_ixs` — let a builder register and onboard a trader without a referral code, using `POST /v1/exchange/build-register-ixs` and `POST /v1/exchange/send-register-ixs`.
- Six new public types exported from the crate root (under the `sdk` feature): `BuildRegisterIxsRequest`, `BuildRegisterIxsResponse`, `SendRegisterIxsRequest`, `SendRegisterIxsResponse`, `ApiInstructionResponse`, and `ApiAccountMeta`.
- New `builder_onboarding_tx` example (requires `solana-keypair` feature) demonstrates the full build-sign-submit flow; run with `--trader-keypair-path` and an optional `--fee-payer-keypair-path`.
- Documentation updated to clarify the three distinct onboarding paths: access-code (`/v1/invite/activate`), referral-code (`/v1/referral/activate-tx`), and builder (`/v1/exchange/build-register-ixs` + `/v1/exchange/send-register-ixs`).

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- To use the new builder onboarding APIs, call `client.exchange().build_register_ixs(...)` with a `BuildRegisterIxsRequest` (trader authority, fee payer, optional `max_positions`), sign the returned instructions locally, then submit the base64-encoded signed transaction via `client.exchange().send_register_ixs(...)`. The API validates, co-signs, simulates, and broadcasts the transaction.
- `ApiInstructionResponse` and `ApiAccountMeta` are the deserialized instruction shapes returned by `build_register_ixs`; downstream code that builds transactions from the response will need to import these types.
- The `max_positions` field on both request types is optional (serialized only when `Some`); defaults to 128 on the server side.
- `trader_pda_index` and `trader_subaccount_index` on `SendRegisterIxsRequest` are also optional; omit them to accept server defaults.
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
