# Changelog

Entries are drafted by Phoenix Rise sync PRs. Review and edit each entry in this
repo before merging.

## v0.9.0 - 2026-10-30

Source Phoenix commit: `af192e9878960010bbdd5bbff8190b03e7a6c578`

### Summary

- Added `l2Book`, `markPrice` and `fills` WebSocket channels to the Rust SDK. New `PhoenixWSClient` methods are `subscribe_to_l2_book`, `subscribe_to_l2_book_with_options`, `subscribe_to_mark_price` and `subscribe_to_fills(Option<symbol>)`. Passing `None` to `subscribe_to_fills` receives fills for every market.
- Added matching `types` and `api` support. `types` gains `L2BookMessage`, `MarkPriceUpdate`, `FillsMessage`, `FillRecord` and the new subscription request types, all exported through the prelude. `api` gains `SubscriptionKey::l2_book`, `SubscriptionKey::mark_price` and `SubscriptionKey::fills`, plus `PhoenixClientEvent::{L2BookUpdate, MarkPriceUpdate, FillsUpdate}`.
- `L2Book` and `Market` can now apply `L2BookMessage` snapshots through `apply_l2_book_message`. `L2Book::slot()` and `slot_index()` expose the slot of the last applied l2Book message. They return `None` after an orderbook-channel update.
- Added `ws l2-book`, `ws mark-price` and `ws fills` commands to the `cli`. The `sdk` examples and READMEs now use the l2Book channel.
- The `orderbook` channel APIs are deprecated in favor of l2Book. This covers `PhoenixWSClient::subscribe_to_orderbook`, `subscribe_to_orderbook_with_options` and `PhoenixClientEvent::OrderbookUpdate`.

### Breaking Changes

- `PhoenixClientEvent::OrderbookUpdate` is no longer emitted for market subscriptions. They now depend on `SubscriptionKey::l2_book` and emit `PhoenixClientEvent::L2BookUpdate` instead. The update payload is `L2BookMessage`, not `L2BookUpdate`, so book fields are read directly as `update.bids` and `update.asks`. Code that handles `OrderbookUpdate` for market subscriptions must switch to `L2BookUpdate`.
- Exhaustive `match` expressions fail to compile on the new variants of these public enums:
  - `ServerMessage`: `L2Book`, `MarkPrice`, `Fills`.
  - `SubscriptionRequest`: `L2Book`, `MarkPrice`, `Fills`.
  - `SubscriptionKey`: `L2Book`, `MarkPrice`, `Fills`.
  - `PhoenixClientEvent`: `L2BookUpdate`, `MarkPriceUpdate`, `FillsUpdate`.
- The `ws_debug_cli` example config key `orderbook` is renamed to `l2_book`. Existing example configs need updating.

### Consumer Notes

- To migrate, replace `subscribe_to_orderbook*` with `subscribe_to_l2_book*`. Replace `SubscriptionKey::orderbook` with `SubscriptionKey::l2_book`. Replace `apply_update` and `apply_l2_book_update` with `apply_l2_book_message` for l2Book messages.
- `L2BookMessage` carries `slot`, an optional `slot_index` and `bypass_execution_band`. The l2Book subscription request uses `coin` and an optional `bypassExecutionBand`.
- Fills subscriptions use an optional `marketSymbol`. A fill message for a market is delivered to both that market's subscribers and all-markets subscribers.
- Routing keys are case-insensitive by symbol for the new channels.
- The deprecated orderbook APIs still compile but emit deprecation warnings. Crates that use `#![deny(warnings)]` will need to migrate or allow the lint.

## v0.8.3 - 2026-10-30

Source Phoenix commit: `5ade6930ff3c7a2bf769435b3d3de8cc5e91af53`

### Summary

- **sdk/types:** `marketStatsV2` websocket updates now carry optional freshness fields: `slot` and `slot_index` on `MarketStatsV2Update`, and `oracle_price_slot` and `mark_price_slot` on `MarketStatsV2Data`. All four are `Option` and are omitted from serialized output when `None`.
- **api:** `get_trader_subaccount` now returns `Ok(None)` when the API returns a trader view whose `state` is `Uninitialized`. Before, only a 404 gave `None`.
- **workspace:** All Rust crates move together from `0.8.2` to `0.8.3`.
- **docs:** The READMEs and `CRATES.md` are reworded. The `referral_activation_tx` example command now lists the `solana-keypair` feature. The CLI README no longer documents the repository-root `mise rise` task.

### Breaking Changes

- **types:** `MarketStatsV2Data` and `MarketStatsV2Update` gained public fields. Code that builds these structs with struct literals must set `slot`, `slot_index`, `oracle_price_slot` and `mark_price_slot`, usually to `None`. The structs are not marked `#[non_exhaustive]` in the diff.
- **api:** `get_trader_subaccount` previously returned `Some(view)` for an uninitialized subaccount. It now returns `None`. Code that treated `Some` as "the subaccount exists" and read the `Uninitialized` state needs updating.

### Consumer Notes

- The four new `marketStatsV2` fields are optional on the wire, so older and newer servers both deserialize. To compute an age in slots, subtract `oracle_price_slot` or `mark_price_slot` from the batch `slot`.
- `oracle_price_slot` is omitted until the market has a full oracle quorum. `mark_price_slot` is omitted once the market is closed and `markPrice` is final.
- If you have a Cargo manifest that pins `0.8.2` for any Rise crate, bump all of them to `0.8.3` together.

## v0.8.2 - 2026-10-30

Source Phoenix commit: `bf2da711175758799b1faf6a2b6627214b8b9fc5`

### Summary

- Rust SDK removes the access-code invite flow. `/v1/invite/activate` is no longer called; referral activation (`/v1/referral/activate`) is now the only trader registration path.
- The `register_trader` example and the usage snippet in `CRATES.md` now take only `--ref-code`. Referral activation requires a user auth context, so the example builds its client with `PhoenixHttpClient::new_from_env_with_auth()`.
- All Rise crates (`api`, `cli`, `sdk`, and the rest of the workspace) move together from `0.8.1` to `0.8.2`.

### Breaking Changes

- `api`: removed `InviteClient::activate_invite(authority, code)`.
- `api`: removed `PhoenixHttpClient::register_trader(authority, code)`, which wrapped `activate_invite`.
- `cli`: removed the `ActivateInvite` subcommand, with its `--authority` and `--code` arguments, from both the `api` and `exchange` command groups.
- `sdk`: the `register_trader` example no longer accepts `--access-code`. It now requires `--ref-code` and fails with `Missing --ref-code` if the flag is absent.

### Consumer Notes

- Replace calls to `activate_invite` or `register_trader` with `client.invite().activate_referral(&authority, referral_code)`.
- `activate_referral` needs a client created with auth, such as `PhoenixHttpClient::new_from_env_with_auth()`. The removed invite call worked with an unauthenticated client.
- Scripts that ran `activate-invite` through the CLI need to switch to `activate-referral`.
- Because this is a `0.x` release, the removals ship in a patch bump (`0.8.1` → `0.8.2`). Consumers pinned with `~0.8` or `^0.8` will pick them up automatically and should review the changes above before upgrading.

## v0.8.1 - 2026-10-30

Source Phoenix commit: `5d208723c66907f0b5641333d6ec13d7e7251117`

### Summary

- **Funding clamp support (`accounts`, `events`, `math`):** `FundingAccumulator` now exposes `funding_clamp_center`, `funding_clamp_band_radius`, `funding_divisor` and `funding_remainder`, taken from what was previously padding. The struct size stays 96 bytes. It also gains `is_paused()`, and the serialized form adds the four new fields. A new `FundingParametersUpdatedWithClamp` market event carries the clamp parameters. `FundingCalculator::projected_rate_percentage` projects the rate that settles at the next funding boundary and applies the clamp, divisor and carried remainder.
- **Trader preference updates (`ix`):** The new `update_trader_preference_bits` module provides `create_update_trader_preference_bits_ix`. A wallet-signed instruction toggles `CollateralSweep` and `PositionAuthoritySwap` through `TraderPreferenceToggleTarget`. `PhoenixInstruction::UpdateTraderPreferenceBits` is added and the module is re-exported in the prelude.
- **Withdrawable collateral (`math`):** `TraderPortfolioMargin::calculate_withdrawable_collateral` now accounts for unsettled funding losses, because settlement consumes the remaining locked deposit before the margin check. Each market's loss is counted separately and credits in other markets don't offset it.
- **Docs (`sdk`):** The README now documents the collateral asset registry semantics and says the exchange cache subscription explicitly requests JSON snapshots.

### Breaking Changes

- `FundingCalculator::new` takes three new arguments: `funding_clamp_center`, `funding_clamp_band_radius` and `funding_divisor`. Pass `0, 0, 1` to keep the previous unclamped behavior.
- `FundingParametersUpdated` (event, enum variant and struct) is deprecated in favor of `FundingParametersUpdatedWithClamp`. `MarketEvent` and `MarketEventType` gain new variants, so exhaustive `match` statements must handle them.
- `RegisterTraderParams` and `OnboardTraderParams` now mask `trader_preference_bits` to the `DisableCollateralSweep` bit only. Any other bits are silently dropped. Set them afterwards with `update_trader_preference_bits`.
- `calculate_withdrawable_collateral` can return larger values than before for traders with unsettled funding losses and a remaining locked deposit.
- The `PhoenixInstruction` variant count rises from 110 to 111. The new variant is inserted mid-enum, after `DelegateTrader`.
- `FundingAccumulator` serialization has four additional fields, which may affect strict downstream schemas.

### Consumer Notes

- Upgrade all Rise crates together to 0.8.1.
- Indexers and event consumers should decode `FundingParametersUpdatedWithClamp` (borsh tag 74). The clamp center and band radius are in hundredths of a basis point of price.
- Callers that relied on `RegisterTrader` or `OnboardTrader` to set preferences other than the collateral-sweep bit must now send a follow-up `update_trader_preference_bits` instruction signed by the trader wallet.
- To display funding for the current interval, use `projected_rate_percentage`. It needs the accumulator, remainder, `last_diff`, timestamps, mark price and index price, and it returns `0.0` for paused markets. `current_rate_percentage` does not apply the clamp.

## v0.8.0 - 2026-10-30

Source Phoenix commit: `42b97feb2da38be7e95b4f3284ccff1c97ecb6b7`

### Summary

- Rust `0.8.0` extends the `types` wire models with new optional/defaulted fields: `ApiCandle` now carries mark-price OHLC (`mark_open/high/low/close`), `volume_quote`, and `external_source`. `MarketPublicMetadata` gains `circulating_supply`, and `SpotAssetConfig` gains `mint` (absent for native SOL). `TraderView` gains `slot`, `slot_index`, and `num_markets_with_splines`. `ExchangeStateSnapshot` gains `pending_withdraw_throttle_replenishment`.
- The `api` WebSocket client and trader-state cache now tolerate unrecognized server messages. Unknown channels, message types, delta fields, and exchange delta ops are skipped without disrupting the exchange store or trader subscriptions, so known deltas continue to be delivered.
- Unrecognized trader escrow actions now deserialize to the new `TraderStateEscrowAction::Unknown` variant instead of failing the whole message. Malformed known actions (e.g. `cash` without `amount`) still fail to deserialize.
- All Rise crates move to `0.8.0` together, with a refreshed `Cargo.lock` (e.g. `tokio` 1.54.1, `compression-codecs` 0.4.44, `zstd-sys` 2.1.2) and extended `deny.toml` advisory exceptions for transitive `crypto-bigint`/`zstd` dependencies.

### Breaking Changes

- `types`: `TraderStateEscrowAction` has a new `Unknown` variant. Exhaustive `match` expressions on it will no longer compile. Treat `Unknown` conservatively, because a request containing it may have effects this SDK cannot interpret.
- `types`: New public fields on `ApiCandle`, `SpotAssetConfig`, `TraderView`, and `ExchangeStateSnapshot` break downstream code that builds these structs with literal syntax. Add the new fields, or use `..Default::default()` where the type implements `Default`.
- `types`: `MarketPublicMetadata` no longer derives `Eq` because of the new `f64` field `circulating_supply`. Code that requires `Eq` or `Hash` on it, or on types containing it, will no longer compile. `PartialEq` is still available.

### Consumer Notes

- The new wire fields use `#[serde(default)]`, so payloads from servers that omit them still deserialize. Missing mark-price candle fields, `slot`, `slot_index`, and `num_markets_with_splines` read as `0`. Missing optional fields read as `None`. A missing `pending_withdraw_throttle_replenishment` reads as `0`.
- Re-serialized `ApiCandle` values now always include the `mark_*` fields. `volume_quote` and `external_source` are omitted when `None`.
- Update any `match` on `TraderStateEscrowAction` with an `Unknown` arm. The `deny.toml` changes only affect this repository's `cargo-deny` checks and need no consumer action.

## v0.7.1 - 2026-10-30

Source Phoenix commit: `1a5e1b01974e5b11a7efe59f95dbcdd8e32fb60e`

### Summary

- `accounts`: Added `Trader::locked_deposit()`, which reads the locked-deposit entry from the trader position map. It returns the new `TraderLockedDeposit` type, which has `locked_quote_lots`, `consumed_quote_lots`, `is_isolated` and `remaining_quote_lots()`. The entry never appears in `positions()`. The new `LOCKED_DEPOSIT_ASSET_INDEX` constant and the new type are exported through the prelude.
- `accounts`: The owned `Trader` now carries `locked_deposit: Option<TraderLockedDeposit>`.
- `types`: Added pending escrow request support to trader-state WebSocket payloads. This adds `TraderStateEscrowRequest`, which has an `is_expired(current_slot)` helper, and `TraderStateEscrowAction` (`Noop`, `Cash`, `LockedDeposit`). Both are exported through the prelude. Snapshots and deltas carry the complete current set of requests, ordered by sequence number.
- `api` / `sdk`: `SubaccountState` now tracks `escrow_requests`. Snapshots replace the list. Deltas replace it only when the field is present, and an empty list clears it. A delta that omits the field leaves the list unchanged.

### Breaking Changes

- `types`: `TraderStateSubaccountSnapshot` has a new public field, `escrow_requests: Vec<TraderStateEscrowRequest>`. `TraderStateSubaccountDelta` has a new public field, `escrow_requests: Option<Vec<TraderStateEscrowRequest>>`. Code that builds these structs with struct literals must add the fields. Deserialization is unaffected, because both fields default when absent.
- `accounts`: The owned `Trader` struct has a new public `locked_deposit` field. `api`: `SubaccountState` has a new public `escrow_requests` field. Struct-literal construction of either type must add the new field.

### Consumer Notes

- Rust crates move from 0.7.0 to 0.7.1 across the workspace. There are no MSRV or dependency changes.
- Escrow request amounts are raw on-chain units. For `Cash` and `LockedDeposit` actions these are quote lots. `expiration_slot` is inclusive, and `None` means the request never expires.
- A locked deposit with `is_isolated` set stays in the account after it is fully consumed. A cross-margin lock is removed instead.

## v0.7.0 - 2026-10-26

Source Phoenix commit: `961469f9d494b060caae4d0c7f10bf59068340da`

### Summary

- **Locked deposits:** Trader state now carries a per-subaccount locked-deposit reserve. This adds the `LockedDepositState` and `TraderStateLockedDeposit` types, the `locked_deposit` field on subaccount snapshots and deltas, `Trader::locked_deposit_{total,consumed,remaining}`, and `TraderView::locked_deposit_remaining`. The `events` and `accounts` components add `LockedDepositChanged`, plus a `LockedDeposit` escrow action kind and a `locked_deposit_amount` accessor. `math` simulations now consume the reserve on realized losses, fees, and funding payments.
- **Spline base widths:** `accounts` reads `bid_base_width` and `ask_base_width` from the spline account, using bytes that were previously padding. Materialized liquidity regions are shifted by the side's base width.
- **New `ix` instruction:** `UpdateSplinePriceWithMetadata` adds a params type with a builder, an `encode`/`decode`/`decode_instruction_data` codec, `create_update_spline_price_with_metadata_ix`, and a new `PhoenixIxError::InvalidSplinePriceMetadata` variant. It supports optional bid and ask widths and a client order ID.
- **Earnings metadata:** `types` adds `MarketPublicMetadata::earnings_dates_v2`, with `EarningsDate` and `EarningsTime` (`amc`/`bmo`). The field defaults to empty, so older payloads still decode.
- **Dependencies:** `pastey` is bumped to 0.2.3, and the lockfiles move to `rustls` 0.23.46, `rustls-webpki` 0.103.16, and `webpki-roots` 1.0.10. `deny.toml` now bans vulnerable `webpki-roots` versions.

### Breaking Changes

- `TraderPortfolioMargin::calculate_transferable_collateral` now deducts the locked-deposit reserve, which changes its result. It is deprecated in favor of `calculate_withdrawable_collateral`. For cross-margin to isolated transfers within one trader PDA, use `calculate_withdrawable_collateral_pre_lock`, which returns the old value.
- New public fields break struct-literal construction:
  - `TraderPortfolio` and `TraderPortfolioMargin` gain `locked_deposit_remaining`.
  - `TraderStateSubaccountSnapshot` and `TraderStateSubaccountDelta` gain `locked_deposit`.
  - `MarketPublicMetadata` gains `earnings_dates_v2`.
  - `SubaccountState` gains `locked_deposit`.
  - The owned `Spline` gains `bid_base_width` and `ask_base_width`.
- New variants break exhaustive matches: `MarketEvent::LockedDepositChanged`, `MarketEventType::LockedDepositChanged`, `EscrowActionKind::LockedDeposit`, and `events::EscrowAction::LockedDeposit`.
- The serialized `EscrowAction` gains a `locked_deposit_amount` field, and the serialized `Spline` gains `bid_base_width` and `ask_base_width`.
- The shared `PhoenixInstruction` enum gains a variant, so `PhoenixInstruction::ALL` grows from 109 to 110 entries. Exhaustive matches and code that depends on the count need updating.
- `PhoenixIxError` gains a variant, which breaks exhaustive matches.
- The workspace `pastey` requirement moves from 0.1 to 0.2.3. Consumers who pin `pastey` 0.1 alongside these crates may see duplicate versions.

### Consumer Notes

- Use `Trader::locked_deposit_remaining()` or the per-subaccount `locked_deposit.remaining()` when computing withdrawals. `calculate_withdrawable_collateral` already applies the reserve.
- Servers that predate `lockedDeposit` omit the field, and the SDK treats a missing entry as no lock. The lock amounts are `JsSafeU64` strings on the wire, so u64 precision is preserved.
- Builders default `refresh_regions` to `true` for `UpdateSplinePriceWithMetadata`, matching `UpdateSplinePrice`. For widths, omission preserves the current value and `0` resets it.
- `UpdateSplinePriceWithMetadataParams::decode` accepts a missing or short client order ID tail and reports it as zero.

## v0.6.15 - 2026-10-26

Source Phoenix commit: `9f55713fed206e5d504ff59ec487c6aee437fb58`

### Summary

- Rust market types now carry an optional `display_name` (a human-readable market name). It appears on `ExchangeMarketConfig`, `ExchangeMarketSnapshot`, `MarketInfo`, `MarketSummary`, and the `MarketStatsV2Data` WebSocket payload. The field is `Option<String>`. It deserializes to `None` when the server omits it, and it is left out of serialized output when `None`.
- The WebSocket exchange-state delta stream adds a `DisplayNameUpdated { previous, new }` market parameter update. `api` applies it to the cached market's `display_name` and emits a `MarketUpdated` cache event with the new `DisplayName` change kind.
- `ExchangeMarketConfig::from(&ExchangeMarketSnapshot)` now copies `display_name` across.
- This is a shared release at version 0.6.15. All Rise crates, including `sdk`, `api`, `core`, `types`, and `litesvm-test`, move together from 0.6.14.

### Breaking Changes

- Source-breaking for Rust code that builds these structs with literal syntax (`ExchangeMarketConfig { .. }`, `ExchangeMarketSnapshot { .. }`, `MarketInfo { .. }`, `MarketSummary { .. }`, `MarketStatsV2Data { .. }`). These types gain a public `display_name` field, so each literal needs `display_name: None` or a value. Exhaustive struct patterns without `..` also need updating.
- `ExchangeMarketParameterUpdate` and `ExchangeCacheMarketChangeKind` each gain a new variant (`DisplayNameUpdated` and `DisplayName`). Exhaustive `match` expressions over either enum must add an arm or a wildcard. Unrecognized update kinds already fall back to `Unknown`, so JSON deserialization is unaffected.

### Consumer Notes

- No wire-format migration is needed. Existing payloads without `displayName` still deserialize, and `None` values are not serialized.
- If you subscribe to cache events, handle `ExchangeCacheMarketChangeKind::DisplayName` to pick up display-name changes. The cache's market entry updates automatically.

## v0.6.14 - 2026-10-12

Source Phoenix commit: `dcad55ef9df91589b8c5e4b03047af27ccab244e`

### Summary

- Rust 0.6.14 adds Order History V2 to `api`/`sdk`. `get_user_order_history_v2` and `get_trader_order_history_v2` return flat packets that include TWAP children. `get_user_organized_order_history_v2` and `get_trader_organized_order_history_v2` return terminal TWAP parents with nested children, plus standalone orders. The new `types::order_history_v2` module holds the request and response types, and live `OrderHistoryDelta` rows gain optional `parent_order_id` and `parent_order_type`.
- Spot collateral now covers SPL tokens as well as native SOL. `accounts` adds `TraderSpotCollateral`, `Trader::spot_collaterals()` and `PerpAssetMap::spot_collaterals()`. Spot assets can be priced by their own oracle (`SpotOraclePriceView`, `SpotCollateralPricing`, `SpotPrice`) when no perp market is bound. `math` margin valuation and the `api` trader state use this pricing.
- `math` converts amounts exactly. `quote_usd_to_quote_lots` and `quote_lots_from_decimal_str` are new, and quote amounts round down. `core` order tickets, bracket legs and multi-limit orders use `order_price_to_ticks` with the order side. `types` `Decimal::from_i64_with_decimals` no longer goes through floating point.
- `cli` Flight `--usdc-amount` uses the exact conversion and rejects amounts below one USDC unit.

### Breaking Changes

- `api` no longer exports `decimal_str_to_quote_lots`. Use `math::quote_lots_from_decimal_str` instead.
- `accounts` and `ix` no longer re-export `TraderPositions` or `TraderPositionIter` from their `trader` and `accounts` modules. `accounts::prelude` still exports them, but they are deprecated. Use `Trader` with `position_entries()` instead.
- `SpotCollateralParams.perp_symbol` is replaced by `pricing: SpotCollateralPricing`. In `SpotCollateralInput`, `pricing_market_symbol` and `index_price` are replaced by `perp_symbol: Option<String>` and `price: Option<SpotPrice>`. `SpotCollateralMargin.pricing_market_symbol` is now `Option<String>`.
- `SpotAssetConfig` has a new `oracle_price` field, and `OrderHistoryDelta` has new `parent_order_id` and `parent_order_type` fields. Code that builds these with struct literals must set them, usually to `None`.
- Owned `accounts::Trader.native_sol_collateral` is deprecated. Use `spot_collaterals` and pick the `NATIVE_SOL_ASSET_INDEX` entry.
- The size of `SpotCollateralMetadata` is unchanged. The oracle price now occupies bytes that were previously padding.

### Consumer Notes

- Price-to-tick conversion for orders is now side-aware. Check order prices if you depend on the previous rounding.
- USDC deposit and withdraw amounts convert exactly and round down, where they previously used floating-point multiplication.
- The V2 history methods work with opaque cursors. Start without a cursor when switching between the flat and organized views. The legacy order history methods are unchanged.
- Update any code that used `Trader` header-extension SOL balances to read `spot_collaterals()` so SPL balances are included.

## v0.6.13 - 2026-10-09

Source Phoenix commit: `96a1919a3bb7bc6a05f6f16604b3dd3def8e102d`

### Summary

- `ix`: Adds a new `spot` module, re-exported through the prelude, with builders for SPL spot collateral instructions: sync, withdraw, transfer, child-to-parent sweep, liquidate, and swaps against USDC, SOL and other spot assets. Adds the `get_trader_wallet_address` PDA helper and eight new `PhoenixInstruction` variants (`SyncSpot`, `WithdrawSpot`, `TransferSpot`, `TransferSpotFromChildToParent`, `LiquidateSpot`, `SwapSpotWithUsdc`, `SwapSpotWithSol`, `SwapSpotWithSpot`).
- `events`: Adds the `MarketEvent` variants `AdditionalFeeAccrued` and `SpotAssetPriceUpdated`, with matching `MarketEventType` variants and the new event structs. `SpotAssetPriceUpdated` encodes at Borsh tag 72. The docs for `SpotCollateralWithdrawnEvent::excess` now say it covers uncounted units of any spot asset, and that child-to-parent sweeps move all uncounted backing.
- `types` and `api`: `MarketStatsUpdate` and `MarketStatsV2Data` gain an optional `cumulative_funding_quote_lots_per_base_lot` decimal string. `TwapSnapshot` gains `terminal_events`, a list of the new `TwapTerminalEvent` type. It is skipped on serialization when empty.
- `ix` and `types` (TWAP): `PlaceTwapOrderParamsBuilder::build` no longer rejects a dust order count above the regular child count. It still requires a positive child count and a total that fits in `u64`. The `PlaceTwapOrderRequest` docs now say the API derives dust executions within the requested `childOrders` total.
- Workspace: All crates move to `0.6.13`. The lockfiles now resolve `solana-pubkey` to `3.0.0` instead of `4.2.x`.

### Breaking Changes

- `types`: `MarketStatsUpdate` has a new public field, so code that builds it with a struct literal must set `cumulative_funding_quote_lots_per_base_lot` (use `None` if unused). `MarketStatsV2Data` has the same new field. `TwapSnapshot` has a new `terminal_events` field with the same effect.
- `events`: `MarketEvent` and `MarketEventType` gain variants. Exhaustive `match` expressions over either enum no longer compile until the new arms are added.
- `ix`: `PhoenixInstruction` gains eight variants, so exhaustive matches on it are affected. `PhoenixInstruction::ALL` grows from 101 to 109 entries.
- `ix` (TWAP): Dust counts that the builder used to reject, such as a dust count greater than the child count, are now accepted. Code that relied on the old `InvalidTwapOrderCounts` rejection must validate on its own.

### Consumer Notes

- Spot builders take a `mint` and derive the trader wallet PDA and its custody ATAs internally. Swap and liquidate builders require an explicit `min_amount_out` or `without_slippage_protection()`. The program applies no oracle price floor to swaps.
- Deposits are a plain SPL transfer into the custody ATA followed by `create_sync_spot_ix`, which is permissionless. Spot withdrawals are never queued and fail if the exchange-wide throttle can't absorb them. Admin instructions (add, configure) are not exposed.
- Struct-literal construction of the changed `types` structs is the main source-compatibility risk. Deserialization of older payloads still works because the new fields default.
- The lockfile `solana-pubkey` resolution moved from `4.2.x` to `3.0.0`. Check dependency resolution if your project pins `solana-pubkey` itself.

## v0.6.12 - 2026-09-29

Source Phoenix commit: `b50781149f884f35dc5e4bca07782a6e6b8e461c`

### Summary

- Rust SDK release `0.6.12`: all Rise crates (`sdk`, `api`, `core`, `ix`, `math`, `types`, `accounts`, `events`) move from `0.6.11` to `0.6.12`. The diff has no public API changes.
- `pinocchio` is bumped from `0.9.2` to `0.9.3` in the workspace and in the example on-chain programs (`close-position-and-withdraw`, `example-program`, `trader-onboarder`).
- The example programs and CLI now use caret ranges for `borsh`, `borsh-derive` and the `solana-*` crates (for example `1.7` instead of `~1.7`). This makes their version bounds more permissive.
- The example programs' lockfiles now resolve `solana-pubkey` `4.2.0`, replacing `3.0.0` for the `api`, `core`, `math` and `litesvm-test` crates.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- If you pin `pinocchio` in an on-chain program that depends on `phoenix-rise`, allow `0.9.3` or newer so it matches the workspace.
- The `solana-pubkey` `4.2.0` resolution appears only in the example programs' lockfiles. It doesn't change the SDK's own `Cargo.toml` requirements.
- To upgrade, set `phoenix-rise` (and any other `phoenix-rise-*` crates you use) to `0.6.12`.

## v0.6.11 - 2026-09-29

Source Phoenix commit: `f00817047214aaf92ece9d8a2aa56e2c7a907fd8`

### Summary

- `types`: `MarketPublicMetadata` has a new public field, `earnings_dates: Vec<DateTime<Utc>>`. It holds known earnings timestamps in ascending order. Date-only values use midnight UTC.
- The field is `#[serde(default)]` and serialized as `earningsDates` (camelCase). Payloads that omit it deserialize to an empty list.
- `api`: the exchange cache test fixtures were updated for the new field. There are no other changes to the published `api` behavior.
- `workspace`: all Rise crates move to the shared version `0.6.11`. The changes are in `programs` lockfiles and workspace dependency pins only.

### Breaking Changes

- Rust code that builds `MarketPublicMetadata` with a struct literal must now set `earnings_dates`, for example `earnings_dates: Vec::new()`. Otherwise it fails to compile. Code that only deserializes API responses is unaffected.

### Consumer Notes

- Clients should pick upcoming earnings from `earnings_dates` by comparing against their own current time. The list is not filtered to future dates.
- Bump all `phoenix-rise*` dependencies to `0.6.11` together so the crate versions stay aligned.

## v0.6.10 - 2026-09-28

Source Phoenix commit: `543c500e521060e7bee25280dbb7d10360bbc1d9`

### Summary

- **ix**: Adds Hawkeye order-quote support, which simulates an order against the matching engine without placing it. New public items are `HawkeyeInstruction::ViewOrderQuote`, `HawkeyeOrderQuoteAccounts`, `create_hawkeye_view_order_quote_ix`, `ViewOrderQuoteReturn`, `OrderQuoteOutcome` and `OrderQuoteRejectionReason`. `ViewOrderQuoteReturn` has helpers such as `net_quote_lots`, `is_fully_filled` and `slippage_bps`.
- **core**: Adds `PhoenixTxBuilder::build_hawkeye_view_order_quote`. It resolves the market accounts for a symbol and builds the quote instruction for a given trader and scratch account.
- **api / types**: Trader-state spot collateral now includes `max_balance`, the effective cap on the trader's counted balance in native units. It is `Option<u64>` in `api::SpotCollateral` and an optional decimal string in `types::TraderStateSpotCollateralSnapshot`. It is `None` when the server predates the field.
- **workspace**: All Rise crates are bumped from `0.6.9` to `0.6.10`, and the shared instruction fixtures are regenerated to include `ViewOrderQuote`.

### Breaking Changes

- `api::SpotCollateral` has a new public field, `max_balance: Option<u64>`. Code that builds `SpotCollateral` with a struct literal, or matches it exhaustively, will stop compiling until the field is added.
- `types::TraderStateSpotCollateralSnapshot` has a new public field, `max_balance: Option<String>`. Struct-literal construction breaks the same way. Deserialization is unaffected because the field defaults to `None`.
- `ix::HawkeyeReturnData` and `ix::HawkeyeInstruction` each gain an `OrderQuote` or `ViewOrderQuote` variant. Exhaustive `match` statements on either enum will need a new arm.

### Consumer Notes

- To quote an order, allocate a zero-lamport, Hawkeye-owned scratch account with a System Program instruction earlier in the same transaction. The buffer must be large enough for the copied state, or the quote fails with `AccountDataTooSmall`. Then decode the result with `decode_hawkeye_return_data`.
- A quote checks matching only. It does not check taker margin or placement permissions, and `cancel_existing` orders are reported as unsupported. Passing `reference_price_ticks = 0` turns off slippage calculation.
- An unparseable `max_balance` string becomes `Some(0)` in `SpotCollateral`, not `None`. Treat `Some(0)` as "no usable cap" only if that suits your use case.

## v0.6.9 - 2026-09-28

Source Phoenix commit: `fdaeef82f69e2b6672f7aba6eb8d8fc1755f13a6`

### Summary

- Adds Flight onboarding instruction builders to `ix` (and the `sdk` re-exports): `create_set_onboarder_signer_ix` (with `SetOnboarderSignerParams`) sets a builder's onboarder signer, and `create_onboard_trader_ix` (with `OnboardTraderParams`) onboards a trader from the builder's own backend, co-signed by the onboarder key.
- Adds PDA helpers `get_flight_trader_onboarding_authority_address` and `get_flight_trader_onboarding_permission_address`.
- Adds `UpdateBuilderOnboarding`, `SetOnboarderSigner` and `OnboardTrader` to `FlightInstruction`, with matching entries in the SDK instruction fixtures.
- `cli`: adds a `flight set-onboarder-signer` command. `flight view` now also shows the builder's onboarder signer, remaining onboardings, and maker and taker fee discounts.
- The workspace version moves from 0.6.8 to 0.6.9 across all Rise crates. READMEs document the new builders.

### Breaking Changes

- `FlightInstruction` gains three new variants (`ALL.len()` goes from 9 to 12). Downstream code that matches on it exhaustively will need new arms.

### Consumer Notes

- `create_onboard_trader_ix` requires non-empty `global_trader_index` and `active_trader_buffer`. `build()` returns `EmptyGlobalTraderIndex` or `EmptyActiveTraderBuffer` otherwise.
- The onboarding transaction needs two signers: the onboarder signer and the payer. The builder authority is a signer only for `set_onboarder_signer`.
- `flight view` now reads the extended builder-state layout, with the onboarder fields at byte offsets 88–129. Accounts that don't have those bytes will fail to decode.

## v0.6.8 - 2026-09-28

Source Phoenix commit: `65e3246ec6c43bc24dc33cbfba075dfd8d561134`

```markdown
### Summary

- `types` adds a `twap` module with `TwapSnapshot`, `TwapAccountSnapshot`, `TwapOrderSnapshot`, and `TwapOrdersQueryParams`. They are also re-exported from the prelude. `TwapOrderSnapshot` includes dust execution parameters: `n_dust_orders` and `dust_order_size`.
- `api` adds `TradersClient::get_twap(authority, params)`. It calls `GET /v1/trader/{authority}/twap` and returns a `TwapSnapshot`.
- `TwapOrdersQueryParams` filters by `trader_pda_index`, `trader_subaccount_index`, and `asset_id`. It serializes in camelCase and omits unset filters. Zero values are sent as filters.
- The workspace `solana-pubkey` requirement is widened from `3.0` to `>=3.0.0, <5`. Fresh resolution now picks 4.x.

### Breaking Changes

- None identified in the synced diff. The new TWAP types and client method are additive. Both `solana-pubkey` majors re-export the same `solana_address::Address` as `Pubkey`, so the public `Pubkey` type is unchanged.

### Consumer Notes

- `TwapOrderSnapshot::n_dust_orders` is `Option<u64>` and is `None` when an older server omits it. `Some(0)` also covers legacy final-child dust. `dust_order_size` is `None` when the server sends null or omits the field. `n_child_orders` counts all executions, including dust executions.
- If your project is pinned to `solana-pubkey` 3.0, you can avoid a duplicate crate in your graph. Run `cargo update -p solana-pubkey@<4.x version> --precise 3.0.0`, provided nothing else in your graph requires 4.x. See the new "Solana Pubkey Compatibility" section in `CRATES.md`.
- The TWAP types are behind the `serde` feature of `types`. The `utoipa` schema derives are available under the `utoipa` feature.
```

## v0.6.7 - 2026-09-28

Source Phoenix commit: `61d16d073d0b2b9be7b0737a3dacf9b1773de7bd`

### Summary

- Adds TWAP dust-order support (`ix`, `types`): child orders can now be split into a smaller trailing "dust" tranche via `TwapIocOrderPacketBuilder::dust_order_size`/`n_dust_orders`, with new validation errors and a matching `PlaceTwapOrderRequest`/`TwapChildOrderParams`/`IsolatedTwapOrderParams`/`TwapMarginType` request shape exported from `types`.
- `accounts` gains explicit perp-vs-spot tagging for asset-map slots (`PerpAssetMetadata::is_perp()`, `asset_map_variant()`, `ASSET_MAP_VARIANT_PERP`/`ASSET_MAP_VARIANT_SPOT`), and `events::SpotCollateralFlow` gains a `Liquidation` variant for SPL spot liquidation debits.
- Removes the exchange-wide `disable_position_authority_swap` kill switch on spot collateral (`accounts`, `events`); only the trader's own preference bit now gates position-authority `SwapNative`.
- Removes the unused `SetMultiArenaAdditionalNodesWatermark`/`SetMultiArenaNumNodesPerArena` instruction discriminants (`ix`).
- Workspace dependency specs for `solana-*` crates and `borsh` switch from tilde (`~`) to caret pins, allowing newer compatible minor versions to resolve.

### Breaking Changes

- Removed `SPOT_COLLATERAL_FLAG_DISABLE_POSITION_AUTHORITY_SWAP`, `SpotCollateralMetadata::position_authority_swap_disabled()` (both `accounts` and `events` crates), and the corresponding field on the owned FFI view. Code checking this flag must remove the check; the exchange-wide gate no longer exists, only the trader preference bit does.
- Removed `PhoenixInstruction::SetMultiArenaAdditionalNodesWatermark` and `PhoenixInstruction::SetMultiArenaNumNodesPerArena` enum variants (`ix`). Any matches on `PhoenixInstruction::ALL` or exhaustive matches over the enum need updating.
- `PerpAssetMap::iter()` now also skips slots tagged as spot-collateral (`ASSET_MAP_VARIANT_SPOT`), not just tombstoned ones. Consumers that relied on `iter()` returning every non-tombstoned slot as perp metadata will see fewer entries once spot-collateral variants are written into the map.
- `solana-*` and `borsh` dependency requirements in `rust/Cargo.toml` moved from tilde (`~x.y`) to caret (`x.y`) pins, so downstream builds may now resolve newer minor versions of these transitive dependencies than before.

### Consumer Notes

- `TwapIocOrderPacket`'s wire encoding now uses 16 of the previously-reserved padding bytes for `dust_order_size`/`n_dust_orders`; packets built or deserialized without these fields default to zero dust, preserving prior behavior.
- New `PlaceTwapOrderParamsBuilder` validation returns `PhoenixIxError::InvalidTwapDustOrderSize` / `InvalidTwapOrderCounts` for malformed dust configurations.
- New `SpotCollateralFlow::Withdraw` doc coverage now includes `WithdrawSpot` in addition to `WithdrawNativeSol`.

## v0.6.6 - 2026-09-23

Source Phoenix commit: `fb668c205addd8ab46faf23f335c4e092d2340c1`

### Summary

- Market symbol handling across `api`, `sdk`, `cli`, and `types` moved from forcing all symbols to uppercase to resolving/preserving the exchange's actual canonical casing (motivated by mixed-case markets like `kBONK`). `ExchangeView::get_market` and `PhoenixMetadata` now do exact-match-then-case-insensitive-fallback lookups, and the HTTP client gained an async canonical-symbol resolver used by the `candles`, `funding`, and `markets` routes.
- WebSocket subscription routing (`SubscriptionKey::routing_key`, `ws_client`) now de-dupes case-insensitively while preserving the requested/canonical case in wire payloads (e.g. `marketStatsV2`, legacy market-stats events) instead of always uppercasing.
- CLI commands (`api`, `export`, `ws`, `trade`, `rpc`) no longer uppercase symbol arguments locally; symbols pass through as typed and are resolved against exchange metadata instead.
- Combined patch bump to `0.6.6` across `math`, `ix`, `core`, `events`, `litesvm-test`, and `sdk`/`api` components; these otherwise only received comment/formatting cleanups, no functional API changes.

### Breaking Changes

- Code relying on the SDK always uppercasing symbols (e.g. passing `"sol"` and getting back `"SOL"` in REST paths, WS payloads, or `PhoenixMetadata`/`ExchangeView` lookups) will now see the exchange's real canonical case instead.
- Case-insensitive symbol resolution is no longer guaranteed to succeed: if a symbol matches multiple entries case-insensitively (ambiguous casing) in the exchange market list, `ExchangeView::get_market` and the HTTP client's canonical resolution now return `None`/leave the symbol unresolved rather than always matching an uppercase form.
- `PhoenixSubscription::market`, `SubscriptionKey::market`/`market_stats_v2`, and CLI symbol arguments no longer normalize case before use — consumers that depended on implicit uppercasing must canonicalize symbols themselves or rely on the new resolution helpers.

### Consumer Notes

- If all your market symbols are already uppercase-only, behavior is effectively unchanged.
- Prefer resolving symbols via `PhoenixMetadata::get_market`/`ExchangeView::get_market` or the HTTP client's canonical-symbol resolution rather than assuming a fixed case convention.
- No published crate's MSRV, edition, or public struct/enum shape changed in this release; the `rust-version`/`edition` bump under `programs/close-position-and-withdraw` is an internal example program, not part of the published `phoenix-rise` crates.

## v0.6.5 - 2026-09-22

Source Phoenix commit: `084d5bebe3936d01d73cd183e65c9126c4aa0ea6`

### Summary

- Adds canonical trigger tracking (`triggers`) to trader-state subaccounts, keyed by market symbol, covering markets that have triggers configured but no open position.
- Introduces new `sdk`/`types` trigger types: `TraderStateConditionalTrigger`, `TraderStateConditionalTakeProfitTrigger`, `TraderStateConditionalStopLossTrigger`, `TraderStateTriggerRow`, `TraderStateTriggerSnapshot`, and `TraderStateTriggerDelta`, exposing attached order sequence number, max/fillable/filled size in lots, and percent-based sizing for advanced conditional TP/SL.
- `Position` and `TraderStatePositionRow` gain `conditional_take_profit_triggers` / `conditional_stop_loss_triggers` fields alongside the existing legacy TP/SL trigger fields, for compatibility on position rows.
- `cli` `ws` command now prints per-market trigger rows in both JSON and human output.
- Shared Rust release version bump across `sdk`, `api`, `core`, `ix`, `math`, `types`, `accounts`, `events`, `cli`, and `litesvm-test` components: `0.6.4` -> `0.6.5`.

### Breaking Changes

- `Position`, `TraderStatePositionRow`, `TraderStateSubaccountSnapshot`, and `TraderStateSubaccountDelta` each gain new public fields (`conditional_take_profit_triggers`, `conditional_stop_loss_triggers`, and/or `triggers`). Any consumer constructing these structs directly via struct literals (rather than via provided constructors/deserialization) must add the new fields to compile.
- On the wire, missing `triggers`/conditional trigger fields on incoming snapshot/delta/position payloads default to empty, so existing deserialization is not broken; this only affects direct struct construction in Rust code.

### Consumer Notes

- New trigger fields deserialize with `#[serde(default)]`, so older server payloads without `triggers` or conditional trigger arrays continue to decode fine, with those collections defaulting to empty.
- `SubaccountState::triggers` is a `HashMap<String, TraderStateTriggerRow>`; snapshots fully replace this map, while deltas use `TraderStateRowChangeKind::Updated`/`Closed` to update or remove individual market entries — apply the same replace/patch semantics you already use for positions and splines.
- Position-level `conditional_take_profit_triggers`/`conditional_stop_loss_triggers` are compatibility fields returned alongside the canonical `triggers` map; prefer the canonical map for markets that may have triggers without an open position.

## v0.6.4 - 2026-09-22

Source Phoenix commit: `b1a3b8f6bfce1a9e6fb77465d77dc82b3879e065`

### Summary

- 0.6.4 deprecates the legacy stop-loss instruction/API flow across the shared Rust workspace (`sdk`, `api`, `ix`, `core`, `cli`, `types`) in favor of conditional orders.
- `PhoenixHttpClient`/`OrdersClient` `place_stop_loss_order` and `cancel_stop_loss_order`, the `ix` crate's `create_place_stop_loss_ix`/`create_cancel_stop_loss_ix`, and `PhoenixTxBuilder::build_stop_loss_orders`/`build_cancel_bracket_leg`/`build_cancel_bracket_leg_from_account` are now marked `#[deprecated]`; use the conditional-order builders (`create_place_position_conditional_order_ix`, `create_cancel_conditional_order_ix`, `place_position_bracket_order`) instead.
- `BracketLegOrders::to_tp_sl_config` / `try_to_tp_sl_config` / `try_to_tp_sl_config_for_side` are deprecated in favor of `place_position_bracket_order` or the `*_with_conditionals` requests.
- `types::TpSlOrderConfig`, `PlaceStopLossOrderRequest`, `CancelStopLossOrderRequest`, and the `tp_sl` field on the isolated order request types are now documented as deprecated, pointing to the conditional-order request types.
- CLI `trade place-stop-loss` / `trade cancel-stop-loss` subcommands are documented as deprecated; no behavior change.

### Breaking Changes

- None identified in the synced diff. All changes are additive `#[deprecated]`/doc annotations; existing signatures, fields, and wire formats are unchanged.

### Consumer Notes

- Existing calls to the stop-loss APIs continue to work but will now emit Rust deprecation warnings; add `#[allow(deprecated)]` at call sites if you need to silence these before migrating.
- Migrate to the conditional-order flow: `buildPlacePositionConditionalOrder`/`buildCancelConditionalOrder` (or `place_position_conditional_order`/`cancel_conditional_order` in Rust), and prefer `place_position_bracket_order` or the `*_with_conditionals` request variants over `TpSlOrderConfig`.
- No functional or wire-format changes in this release — safe to bump the dependency without code changes, though you should plan the deprecated-API migration before the legacy stop-loss path is removed in a future release.

## v0.6.3 - 2026-09-22

Source Phoenix commit: `43aba601e5ce668b168aff936d48a55aafa81985`

### Summary

- `math`: `portfolio_value()` now includes `unsettled_funding` in the computed portfolio value, alongside quote-lot collateral, unrealized PnL, and spot collateral notional.
- `math`: Zero-balance spot collateral positions are now valued as zero without requiring a pricing-market metadata lookup.
- `math`: Fixed `BasisPoints::apply_to_quote_lots` to no longer spuriously return `None` when the intermediate `u64` product overflows but the final floored quotient would fit — it now falls back to a `u128` computation to match on-chain floor semantics across the full `QuoteLots` range.

### Breaking Changes

- `math`: `SpotCollateralInput.index_price = None` no longer falls back to the pricing market's mark price for nonzero balances. Callers must now supply an explicit index price for any nonzero spot collateral balance; a missing price returns a valuation error instead of silently using the mark price.
- `math`: `TraderPortfolioMargin::portfolio_value()` now factors in `unsettled_funding`, changing the numeric result for portfolios with nonzero unsettled funding compared to `0.6.2`.

### Consumer Notes

- If you were relying on `index_price: None` to use the market mark price for spot collateral valuation, pass the index price explicitly going forward — the fallback is gone and omitting it now errors for nonzero balances.
- Recheck any downstream code that compares or asserts on `portfolio_value()` output, since it now includes unsettled funding.
- No public API signatures changed beyond the version bump (`0.6.2` -> `0.6.3` across all Rise Rust crates); this is a behavioral/patch-level update.

## v0.6.2 - 2026-09-18

Source Phoenix commit: `888c5453b1f7394ab1b2f4a26d1ce24eee06a0dd`

### Summary

- Released `phoenix-rise` 0.6.2, bumping all Rise Rust crates (`sdk`, `accounts`, `api`, `core`, `events`, `ix`, `math`, `types`, `litesvm-test`, `cli`) from 0.6.0.
- Fixed a bug in `accounts`'s spline collection active-region slicing: the region count was previously added to the start offset instead of used as the slice's end index, which could return more (or different) regions than intended.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- Spline collection consumers that read active tick regions will see corrected slice results after this update; if your code compensated for the previous slicing behavior, re-verify it against 0.6.2.
- Update all `phoenix-rise-*` crate dependencies to `0.6.2` together — this is a single shared workspace version bump, not independent per-crate releases.

## v0.6.0 - 2026-09-18

Source Phoenix commit: `e9a7915d9161e5dae5057f91a2a507cc2e528939`

### Summary

- Bumps the shared Rise Rust workspace (`sdk`, `accounts`, `api`, `core`, `events`, `ix`, `math`, `types`) from `0.5.7` to `0.6.0`, alongside a coordinated upgrade of Solana crate dependencies to their v4 lines (`solana-account`, `solana-rpc-client`, `solana-rpc-client-api`, `solana-message`, `solana-transaction`, `solana-transaction-status-client-types`) and related crates (`solana-signature`, `solana-instruction`, `solana-keypair`, `solana-transaction-error`).
- Relaxes the `borsh` requirement from an exact pin (`=1.6.0`/`1.5`) to `~1.7`, and bumps `indexmap` to `=2.14.2`.
- Upgrades `litesvm` from `=0.11.0` to `=0.16.0` in the `litesvm-test` harness.
- Adds an explicit `svm.warp_to_slot(0)` call in `SdkLocalnetContext` to restore the slot-0 baseline that litesvm test fixtures assume, since litesvm `>= 0.16` now starts the clock at a mainnet-like slot.

### Breaking Changes

- Consumers building against the Rise Rust crates now require Solana crates from the v4 line (e.g. `solana-account ~4.3`, `solana-rpc-client 4.2.2`, `solana-rpc-client-api 4.2.2`, `solana-message ~4.4`, `solana-transaction ~4.1`, `solana-transaction-status-client-types 4.2.2` with `agave-unstable-api`) instead of v3 — code depending directly on these crates' v3 APIs will need to update.
- `litesvm` is now pinned to `=0.16.0` (from `=0.11.0`); downstream test code using `litesvm` directly, or relying on the previous default clock/slot behavior, may need to adjust (see the added `warp_to_slot(0)` workaround in `litesvm-test`).
- `borsh` moved from an exact `=1.6.0` pin to `~1.7`, and `indexmap` moved to `=2.14.2`; downstream crates pinning exact older versions of these may hit resolver conflicts.

### Consumer Notes

- If you pin Solana crate versions directly in your `Cargo.toml`, update them to match the new v4-line requirements to avoid duplicate-version resolution issues.
- If you use `phoenix-rise-litesvm-test`'s `SdkLocalnetContext` (or `litesvm` directly) in tests that assume slot 0 at startup, be aware the underlying `litesvm` upgrade changes the default starting slot — the harness now compensates internally, but custom test setups using `litesvm` outside this harness may need the same `warp_to_slot(0)` adjustment.
- No public API signatures changed in this release; this is primarily a dependency/toolchain upgrade.

## v0.5.7 - 2026-09-18

Source Phoenix commit: `9ea8a32688dff1ca9f6ce869c52f022b07c94257`

```markdown
### Summary

- `MarketPublicMetadata` (types) gained a `classifications` field (`Vec<String>`) for tagging markets, e.g. `"pre-ipo"`, `"new-listing"`; the exchange cache (api) now surfaces it alongside existing metadata.
- `TradeHistoryDelta` and `TradeHistoryItem` (types) gained a `parent_twap_id` field for correlating fills that belong to the same parent TWAP order.
- Shared Rust package version bumped 0.5.6 → 0.5.7; no functional changes to build/workspace configuration.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- Both new fields are additive and optional: `classifications` defaults to an empty array and `parentTwapId` defaults to `None`, and neither is emitted in serialized JSON when empty/absent, so existing serializers/deserializers continue to work unchanged.
- When populated, expect `classifications: string[]` on market metadata responses and `parentTwapId: string` on trade history entries (camelCase on the wire).
```

## v0.5.6 - 2026-09-15

Source Phoenix commit: `cdd5303fea774b0609f2402dee4e387dbf90714a`

### Summary

- Bumped the shared Rise Rust crate version from `0.5.5` to `0.5.6` across all
  components (`sdk`, `math`, `accounts`, `api`, `core`, `events`, `ix`, `types`,
  `cli`, `litesvm-test`).
- Added
  `ix::native_sol::create_deposit_native_sol_ixs(payer, trader_wallet, lamports, sync_params)`,
  a helper that returns the ordered instruction set for depositing native SOL:
  trader reallocation, then a System Program transfer, then `SyncNative`.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- Reallocation runs before the transfer so newly deposited SOL isn't consumed by
  a higher rent floor; pass the same wallet for `payer` and `trader_wallet` when
  there's no separate rent sponsor.
- The trader account must already exist, or be registered earlier in the same
  transaction, before calling this helper.
- Passing `lamports == 0` returns `PhoenixIxError::InvalidDepositAmount` rather
  than building a no-op transfer.
- `create_realloc_trader_ix` remains available on its own for callers building
  custom instruction bundles.

## v0.5.5 - 2026-09-15

Source Phoenix commit: `9a60c7ba487c57c88a8769607120204ffc3bc3de`

### Summary

- Added `ix::native_sol::create_realloc_trader_ix` and a new
  `PhoenixInstruction::ReallocTrader` variant, letting a trader account reserve
  extra non-position capacity (e.g. for native SOL) without increasing its
  perpetual-position slot count.
- `accounts` now exposes richer mark-price data: `OracleParameters` gained
  `book_hard_stale_multiplier` and `oracle_hard_stale_multiplier`, and
  `PerpAssetMetadata` gained `finalized_mark_price`.
- `types::ix` isolated-order request types (`PlaceIsolatedLimitOrderRequest`,
  `PlaceIsolatedLimitOrderWithConditionalsRequest`,
  `PlaceIsolatedMarketOrderRequest`, `PlaceIsolatedMarketOrderV2Request`) gained
  an optional `transfer_spot_collateral_amounts` map for funding isolated
  positions with native/spot collateral keyed by symbol.
- Shared Rust release across `math`, `sdk`, `ix`, `accounts`, `types`, and
  `litesvm-test` components.

### Breaking Changes

- `accounts::OracleParameters` gained two new required fields
  (`book_hard_stale_multiplier`, `oracle_hard_stale_multiplier`); code that
  builds this struct with an exhaustive literal or destructures it exhaustively
  will need updating.
- `accounts::PerpAssetMetadata` gained a new required field
  (`finalized_mark_price`) for the same reason.

### Consumer Notes

- `PhoenixInstruction` gained a new `ReallocTrader` variant; downstream code
  that exhaustively matches on `PhoenixInstruction` should add a case for it.
- The new `transfer_spot_collateral_amounts` fields default to empty and are
  skipped during serialization when empty, so existing callers and stored
  payloads remain wire-compatible.
- Call `create_realloc_trader_ix(payer, trader_wallet, trader_account)` ahead of
  operations that need extra non-position capacity on a trader account; it grows
  capacity by at most one entry, succeeds as a no-op when capacity already
  suffices, and never shrinks.

## v0.5.4 - 2026-09-15

Source Phoenix commit: `6f2ee74196c4494d2899c3a677a99255fe59458e`

### Summary

- Bump the shared Rise Rust workspace release to `0.5.4` (`sdk`, `accounts`,
  `api`, `core`, `events`, `ix`, `math`, `litesvm-test`, `cli`, `types`).
- Fix a concurrency bug in `api`'s HTTP transport: the client now compares only
  credential fields (access token, refresh token, PoP key) when deciding whether
  a session needs reloading, instead of the full session snapshot. Previously,
  derived expiry timestamps and the per-request PoP counter — which change
  independently of a real credential refresh — could mask a
  concurrently-refreshed session or trigger spurious reload skips.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- No public API signatures changed; `PhoenixApiClient` consumers making
  concurrent requests should see auth refresh correctly detected across requests
  instead of being masked by unrelated expiry/counter drift.
- Update to `0.5.4` to pick up the auth refresh fix.

## v0.5.3 - 2026-09-15

Source Phoenix commit: `e902efb7e98b284b8a56cae5c98df492a7d0ac4b`

### Summary

- Shared Rust workspace release (`accounts`, `api`, `core`, `events`, `ix`,
  `litesvm-test`, `math`, `types`) bumping `0.5.2` → `0.5.3`.
- Adds a batched, multi-symbol WebSocket market-stats channel (`marketStatsV2`)
  alongside atomic isolated-market-order v2 requests with TP/SL triggers, and
  scale-order-set "continuation" packets for ladders split across transactions.
- Tightens scale-order-set id encoding, exchange-active/maintenance-state
  checks, stop-loss execution price validation, and quote-withdrawal collateral
  math to match on-chain behavior.

### Breaking Changes

- `accounts`: `GlobalConfig::is_exchange_active` /
  `GlobalConfiguration::is_exchange_active` now take a `LastRestartSlot` enum
  instead of `Option<u64>`, and also fold in a new maintenance-flag check —
  callers must update call sites and expect maintenance mode to make the
  exchange inactive even with a matching restart slot.
- `ix`: `scale_set_id` is now capped at 127 (bit 7 is reserved to tag
  continuation packets); ids 128–255 are now rejected with
  `PhoenixIxError::InvalidScaleSetId` instead of being accepted.
- `events`: `AdminParameterUpdateKind::AcknowledgedRestartSlot { .. }` is
  removed and replaced by a reserved, non-emitted variant — code matching on the
  old variant no longer compiles.
- `types`/`events`: new enum variants (`ServerMessage::MarketStatsV2`,
  `SubscriptionRequest::MarketStatsV2`, `MarketEvent::Reserved70`) will break
  exhaustive `match` statements in consumer code.
- `ix`: `PlaceStopLossParamsBuilder::build()` no longer silently defaults
  execution price to `0`; it now defaults to `trigger_price` and returns
  `PhoenixIxError::InvalidExecutionPrice` if the resolved execution price is
  `0`.

### Consumer Notes

- `math`: `TraderPortfolioMargin::calculate_transferable_collateral` now lets
  spot collateral back the withdrawal margin bound while still capping payout at
  quote-only collateral, so computed withdrawable amounts can change (generally
  increase) versus 0.5.2.
- New `subscribe_to_market_stats_v2` / `PhoenixSubscription::market_stats_v2`
  (all-markets or filtered) on `PhoenixWSClient`/`PhoenixClient`; the legacy
  single-symbol `market` subscription is now implemented on top of this channel
  with no API change.
- New `PlaceIsolatedMarketOrderV2Request` plus
  `build_isolated_market_order_tx_v2_with_request` /
  `..._enhanced_v2_with_request` for atomic isolated entry with
  `greaterTrigger`/`lessTrigger` TP/SL (requires a backend release with
  PRO-829); existing `PlaceIsolatedMarketOrderRequest` call sites remain
  source-compatible.
- `MultiLimitOrderParamsV2Builder::scale_set_continuation` marks ladder legs
  sent in follow-up transactions; `TraderView` gains a defaulted
  `withdrawable_quote_collateral` field and `PnlPoint` gains
  `cumulative_maker_fee`.
- `ExchangeStateSnapshot` and `ExchangeDeltaOp::ExchangeStatusChanged` gain a
  defaulted `running_state: ExchangeRunningState`
  (Unknown/Active/Maintenance/Suspended).

## v0.5.2 - 2026-09-15

Source Phoenix commit: `bb97cff3b8d6ca1cb131fcdd43d2a8baa471c23a`

### Summary

- Added a new `place_multi_limit_order_v2` instruction end-to-end: `ix` gains
  `CondensedOrderV2`, `MultipleOrderPacketV2`,
  `MultiLimitOrderParamsV2`/builder, and `create_place_multi_limit_order_v2_ix`;
  `core` gains `PhoenixTxBuilder::build_multi_limit_order_v2` and
  `build_multi_limit_order_v2_with_params`.
- V2 moves `slide`/`reduce_only` from packet-wide fields to a per-order
  `CondensedOrderFlags` byte on each leg, adds a fixed-width optional
  `last_valid_slot` (`OptionalNonZeroU64`), and adds a `scale_set_id` for
  caller-assigned scale-order ladders.
- The existing `place_multi_limit_order` (V1) instruction, types, and builder
  methods are unchanged and remain available alongside V2; V1 and V2 share an
  identical account list.
- Bumped the shared Rise Rust workspace crates (`sdk`, `core`, `ix`, `math`,
  `types`, `accounts`, `api`, `events`, `litesvm-test`, `cli`) from 0.5.1 to
  0.5.2.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `CondensedOrderV2::last_valid_slot` serializes as a raw `u64`, not a Borsh
  `Option`: `0` (including an explicit `Some(0)`) means "no expiry."
- `PhoenixTxBuilder::build_multi_limit_order_v2` applies one
  `slide`/`reduce_only` pair to every leg; use
  `build_multi_limit_order_v2_with_params` with hand-built `CondensedOrderV2`
  legs for per-order flags.
- V2 instruction data uses a distinct discriminant
  (`place_multi_limit_order_v2`) from V1, so existing V1 integrations are
  unaffected by adopting V2 selectively.

## v0.5.1 - 2026-09-14

Source Phoenix commit: `da9e74fe54eb6430cedc59ff530feaa5da8130e9`

### Summary

- **accounts**: `GlobalConfig`/`GlobalConfiguration` decode a new
  `acknowledged_restart_slot` field and expose
  `is_exchange_active(last_restart_slot)` to compute effective exchange activity
  across a restart interlock.
- **api**: `PhoenixHttpClient` adds a shared, client-wide rate-limit cooldown
  (`RateLimitCooldownConfig`) so independent retryable GET requests can wait out
  an active `Retry-After` window instead of hammering the server in parallel.
- **events**: `AdminParameterUpdateKind` gains an
  `AcknowledgedRestartSlot { previous, new }` variant.
- **ix**: adds a new `AcknowledgeRestart` instruction and discriminant.

### Breaking Changes

- **accounts**: `GlobalConfigPrefixRaw`/`GlobalConfiguration` grew by 8 bytes
  (1096 -> 1104) to add `acknowledged_restart_slot`; downstream code that
  constructs `GlobalConfiguration` via struct literal, or hardcodes the previous
  prefix length/offsets, must be updated.
- **events / ix**: `AdminParameterUpdateKind` and `PhoenixInstruction` each
  gained a new public enum variant (`AcknowledgedRestartSlot`,
  `AcknowledgeRestart`); exhaustive `match` statements over these enums in
  downstream code will fail to compile until updated.
- **api**: the new rate-limit cooldown is **enabled by default** on
  `PhoenixHttpClient`. Independent GET calls made on a shared/cloned client can
  now block waiting on another request's `Retry-After` cooldown, a behavior
  change even though existing code still compiles. Use
  `.disable_rate_limit_cooldown()` (builder) or
  `RateLimitCooldownConfig::disabled()` to restore the previous per-request-only
  behavior.

### Consumer Notes

- **api**: `Retry-After`-driven retry delays for the request that hit the 429
  now always add small positive jitter (100–115%) instead of using the raw
  header value verbatim.
- **api**: new `RateLimitCooldownConfig` is exported from
  `phoenix_rise_api`/`phoenix_rise::api`; configure it via
  `PhoenixHttpClientBuilder::with_rate_limit_cooldown_config` /
  `with_rate_limit_cooldown_enabled`, or the client's
  `set_rate_limit_cooldown_*` setters.
- **accounts**: prefer the new `acknowledged_restart_slot()` /
  `is_exchange_active(last_restart_slot)` accessors over reading raw
  `exchange_status` bits when checking exchange activity after a restart.

## v0.5.0 - 2026-09-14

Source Phoenix commit: `7e656050cf75743be140e7fb1ab849056f002e62`

### Summary

- Combined Rust release across `api`, `ix`, `types`, and `sdk` components
  (`0.4.1` → `0.5.0`): adds a canonical `candles_v2` client/type surface,
  propagates scale-order ids through order/event/history types, adds market
  search aliases, and removes two exchange-status instruction variants.

### Breaking Changes

- `ix`: removed the `PhoenixInstruction::SetExchangeStatusBits` and
  `DisableExchangeCapabilities` enum variants and their discriminants. Any
  downstream `match`/`from_instruction_name`/fixture code referencing these
  variants will no longer compile or will fail lookups.
- `types`/`api`: added public fields —
  `MarketPublicMetadata.search_aliases: Vec<String>`, and
  `scale_set_id: Option<u8>` on `trader::TraderStateMarketLimitOrderEvent`,
  `trader_http::LimitOrder`, `trader_http::OrderHistoryItem`, and
  `api::trader_state::LimitOrder`. Deserialization is unaffected (fields
  default), but code that constructs these structs with exhaustive struct
  literals must add the new field.
- `sdk` example `builder_onboarding_tx`: signer model changed.
  `--trader-keypair-path`/`TRADER_KEYPAIR_PATH` was removed; the example now
  only signs locally with `--fee-payer-keypair-path`/`FEE_PAYER_KEYPAIR_PATH`
  (default `~/.config/solana/id.json`), and takes an optional
  `--trader-authority`/`TRADER_AUTHORITY` public key (defaulting to the fee
  payer) instead of a trader keypair.

### Consumer Notes

- New `candles_v2` surface: `PhoenixHttpClient::get_candles_v2` /
  `CandlesClient::get_candles_v2`, with `CandlesV2InitialQueryParams` (explicit
  millisecond `from`/`to` window) or `CandlesV2CursorQueryParams` (opaque
  cursor), returning `CandlesV2Response` with `ApiCandleV2` bars (mark-price
  OHLC, external-source metadata, `is_final`) and `CandlesV2Page` pagination.
  Defaults to 1,000 bars per page, up to 10,000; the legacy
  `get_candles`/`ApiCandle` endpoint and shape are unchanged, with only doc
  clarification (timestamps are milliseconds) and an added `PartialEq` derive.
- `scale_set_id` lets consumers identify orders placed together as legs of a
  scale (ladder) order batch via `PlaceMultiLimitOrderV2`.

## v0.4.1 - 2026-09-14

Source Phoenix commit: `cd96962f181a2e28c87ec592db4f20460c6b510e`

### Summary

- Added Flicker/TWAP instruction support in `ix`: PDA derivation helpers and
  builders for creating, placing, executing, cancelling, and closing TWAP
  orders, plus the `FlickerInstruction` discriminant enum and
  `FLICKER_PROGRAM_ID` export.
- `api`'s `SpotCollateral` and `types`' `TraderStateSpotCollateralSnapshot` now
  carry a `decimals` field for the asset's native-unit decimals.
- `types`' `PlaceIsolatedMarketOrderRequest` gained an optional
  `min_quote_lots_to_fill` field.
- Added `PhoenixIxError::InvalidTwapOptionalU64` in `ix`, returned when a TWAP
  optional u64 field is explicitly set to `0`.
- Workspace crates bumped to `0.4.1`.

### Breaking Changes

- `SpotCollateral` (`api`) and `TraderStateSpotCollateralSnapshot` (`types`)
  gained a required `decimals: u8` field with no default — struct literals and
  deserialized payloads that don't supply it will fail to compile or
  deserialize.

### Consumer Notes

- The new `ix::twap` builders (`CreateTwapAccountParams`,
  `PlaceTwapOrderParams`, `ExecuteTwapOrderParams`, `CancelTwapOrderParams`,
  `CloseInactiveTwapAccountParams`, `TwapIocOrderPacket`) follow the existing
  builder pattern and auto-derive PDA/program-id fields when left unset.
- TWAP optional u64 fields (`price_in_ticks`, `num_quote_lots`, `match_limit`,
  `last_valid_slot`, min/max price-in-ticks, collateral quote-lots) must be
  `None` or strictly greater than zero; passing `Some(0)` returns
  `PhoenixIxError::InvalidTwapOptionalU64`.
- `min_quote_lots_to_fill` on `PlaceIsolatedMarketOrderRequest` is optional and
  serializes with `#[serde(default)]`, so existing callers that omit it are
  unaffected.

## v0.4.0 - 2026-09-14

Source Phoenix commit: `72cb240e31283f3518f5f5f9ee4f2a64bfd89be2`

### Summary

- Adds native SOL spot collateral end-to-end: on-chain fields
  (`GlobalConfig::native_sol_spot_metadata`, `Trader::native_sol_collateral`),
  six new instructions in `ix` (`sync_native`, `withdraw_native_sol`,
  `transfer_native_sol`, `transfer_native_sol_from_child_to_parent`,
  `liquidate_native_sol`, `swap_native`), three new `events`
  (`SpotCollateralDeposited`/`Withdrawn`/`Liquidated`), and valuation support in
  `math`/`api` (`SpotCollateralParams`, `TraderPortfolio::spot_collaterals`,
  `PhoenixClient` pulling `/v1/collateral/assets`).
- Adds a trader preference to disable `SwapNative` when signed by the position
  authority (`accounts::TraderPreferenceKind::DisablePositionAuthoritySwap`).
- `api`'s `PhoenixClient`/`PhoenixHttpClient` gain `get_spot_collaterals`, fold
  spot collateral into `PhoenixMetadata` on connect and via WS deltas, and
  expose `SubaccountState::to_trader_portfolio_with_metadata` for margin that
  includes SOL collateral.
- `types::TraderView` and trader-state snapshots gain `spot_collaterals` balance
  views; `PlaceIsolatedMarketOrderRequest` gains an optional
  `min_base_lots_to_fill`.

### Breaking Changes

- `math::RiskAction::Withdrawal` is renamed to `WithdrawQuoteCollateral`, and
  `ADL` is deprecated in favor of `View`; update any exhaustive matches on
  `RiskAction`.
- New enum variants break exhaustive `match` arms downstream:
  `api::MarginTrigger::SpotCollateralsUpdated`,
  `types::ExchangeDeltaOp::SpotCollateralsUpdated`,
  `events::MarketEvent::SpotCollateralDeposited/Withdrawn/Liquidated`,
  `events::AdminParameterUpdateKind::SpotCollateralConfig`,
  `api::PhoenixClientError::Metadata`, and
  `accounts::TraderPreferenceKind::DisablePositionAuthoritySwap`
  (`ALL_TRADER_PREFERENCE_KINDS` grew from 1 to 2 entries).
- `math::TraderPortfolio` gained a required
  `spot_collaterals: Vec<SpotCollateralInput>` field — struct literals built
  without `..Default::default()` will no longer compile.
- The `GlobalConfig` account layout grew (previously-reserved bytes now hold
  `native_sol_spot_metadata`), and the total instruction count grew from 96 to
  102 — code asserting exact raw byte offsets/sizes or exact instruction-enum
  counts needs updating.

### Consumer Notes

- Spot collateral is valued at the bound perp market's index price with a linear
  balance-dependent discount curve; see `math::spot_collateral` for the exact
  formula and rounding rules.
- `PhoenixClient::new` fails open (values SOL at zero) if
  `/v1/collateral/assets` is unreachable, but fails construction if a successful
  response references an unknown perp market — treat that as a server/client
  version mismatch.
- Admin instructions for configuring/activating native SOL are deliberately not
  exposed in `ix::native_sol`; only user- and liquidation-facing instructions
  are public.

## v0.3.6 - 2026-09-14

Source Phoenix commit: `8b82f4267044e45cd33050bf86138e845866e049`

### Summary

- Rust Rise packages (`api`, `ix`, `math`, `sdk`, `types`, `workspace`) bump
  `0.3.5` → `0.3.6`.
- Adds a new exchange snapshot/delta WebSocket channel (`api`) and a
  `SharedExchangeCacheStore`, letting `PhoenixFlightClient` resolve the live
  Phoenix root authority and wrap orders signed by a trader's position authority
  (delegate key), with a new `AuthorizedTransferCollateral` instruction to move
  the builder fee in that path.
- Fixes bounded quantity types (`math`) to validate range on Borsh
  deserialization and to return typed values from
  `lower_bound()`/`upper_bound()` instead of raw primitives.
- Adds raw per-asset spot collateral balances to trader-state snapshots/deltas
  (`types`, `api`).

### Breaking Changes

- `PhoenixFlightClient::try_wrap_order_instruction` gained a required
  `use_position_authority: bool` parameter; existing call sites must be updated
  to pass `false` for owner-signed orders or `true` for
  position-authority-signed orders.
- `ScalarBounds::lower_bound()` / `upper_bound()` now return `Self` instead of
  the inner primitive type.
- New variants were added to public enums that downstream code may match
  exhaustively: `PhoenixInstruction::AuthorizedTransferCollateral`,
  `PhoenixIxError::MissingRootAuthority`, and
  `ServerMessage`/`SubscriptionRequest::Exchange`.
- `ExchangeSnapshotMessage`, `ExchangeEncodedSnapshotMessage`, and
  `ExchangeDeltaMessage` dropped their `channel: String` field (the channel is
  now carried by the outer message envelope); code reading that field directly
  will fail to compile.
- Bounded quantity types built via `basic_u64_struct_with_bounds!` (e.g.
  margin/risk-factor types) now enforce their bounds when Borsh-deserialized and
  return a `std::io::Error` for out-of-range values instead of silently
  accepting them.

### Consumer Notes

- To use position-authority Flight wraps, obtain a populated store via
  `PhoenixWSClient::exchange_store()` and construct the client with
  `PhoenixFlightClient::from_exchange_store(...)`; clients built with
  `PhoenixFlightClient::new(...)` have no store and such wraps fail with
  `MissingRootAuthority`.
- A new client-level `PhoenixFlightClient::with_fee_bps_override(...)` sets a
  default builder fee override applied by `try_wrap_order_instruction`; the
  existing per-call `try_wrap_order_instruction_with_fee_bps_override(...)`
  argument still takes precedence.
- `TraderStateSubaccountSnapshot`/`TraderStateSubaccountDelta` gained a
  `spot_collaterals: Vec<TraderStateSpotCollateralSnapshot>` field; it defaults
  to empty via serde so existing payloads and consumers remain compatible.
- Exchange-channel subscriptions should request `ExchangeSnapshotEncoding::Json`
  explicitly (via `ExchangeSubscriptionRequest`); the server otherwise defaults
  to a `base64+zstd` encoding this SDK does not decode.

## v0.3.5 - 2026-07-10

Source Phoenix commit: `3343d458b9162061b4f810408bee2b14bcdf6af0`

```markdown
### Summary

- `ix`: added `usdc_mint()`, an environment-aware USDC mint resolver that reads
  `PHOENIX_ENV` once (via `resolve_usdc_mint_for_env`) and returns
  `BETA_USDC_MINT` when set to `beta`, defaulting to the existing mainnet
  `USDC_MINT` otherwise.
- `ix`: added the new `BETA_USDC_MINT` constant for the beta deployment's USDC
  mint address.
- `core`, `cli`: internal call sites (`tx_builder`, `flight` command) now
  resolve the USDC mint via `usdc_mint()` instead of the static `USDC_MINT`
  constant, so transactions built through these paths follow `PHOENIX_ENV`.

### Breaking Changes

- None identified in the synced diff. `USDC_MINT` remains exported and still
  resolves to the mainnet address.

### Consumer Notes

- If your process may target the beta deployment, prefer calling `usdc_mint()`
  over the static `USDC_MINT` constant, and set `PHOENIX_ENV=beta` before first
  use — the resolved value is cached for the life of the process.
- No action needed for mainnet-only consumers; behavior and the `USDC_MINT`
  constant are unchanged.
```

## v0.3.4 - 2026-07-08

Source Phoenix commit: `6051225fb045fbb5b6a454bd445e7fc2e31e5722`

### Summary

- Reworked the per-market liquidation price calculation (`math`) to search for
  the actual tick boundary where an account becomes liquidatable, replacing the
  previous closed-form algebraic approximation. This correctly returns the
  current mark price when an account is already liquidatable and properly
  accounts for target-market limit-order maintenance instead of treating it as a
  fixed outside term.
- Added a new projected liquidation price API — `projected_liquidation_price`,
  `ProjectedLiquidation`, `ProjectedLiquidationFill`,
  `ProjectedLiquidationParams`, and
  `TraderPortfolioMargin::projected_liquidation_prices` — that simulates a
  trader's own position-side resting orders filling along the adverse price
  path, for risk/explainer displays distinct from the static Hawkeye-compatible
  liquidation price.
- Added `initial_margin_for_asset_with_mark_price` and
  `calculate_liquidation_price_usd_with_target_limit_order_maintenance` as
  supporting public entry points for callers that need to supply an explicit
  mark price or an additional target-market limit-order maintenance term.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `calculate_liquidation_price_usd` keeps its existing signature and default
  behavior (it now delegates to the new maintenance-coefficient variant with
  `0.0`), so existing callers are unaffected.
- Liquidation prices from portfolio margin calculations are now solved as an
  exact tick boundary rather than a closed-form approximation, so returned
  values may shift slightly (now quantized to the market's tick size) and should
  be more accurate, particularly for accounts with resting orders in the target
  market.
- Consumers wanting "what if my resting orders fill before liquidation"
  estimates can adopt `TraderPortfolioMargin::projected_liquidation_prices`
  alongside the existing static liquidation price fields; it's an additive risk
  estimate, not a replacement for the static value.

## v0.3.3 - 2026-07-08

Source Phoenix commit: `25625a376965069d216ba53f8bbf0457f097a927`

### Summary

- `math`: `LimitOrderMarginState` now tracks best bid/ask prices and reduce-only
  order size separately, and initial margin calculations reserve additional
  collateral for the mark-to-market loss a resting order would incur if it
  filled at its limit price (adverse fill loss), including for reduce-only
  orders.
- `math`: Reduce-only order sizes are now capped to the lots that can actually
  reduce the trader's existing position before they factor into margin.
- `math`: Added a `is_zero()` helper on the numeric wrapper types generated by
  the shared `basic_num!` macro (e.g. `BaseLots`, `SignedBaseLots`,
  `QuoteLots`).

### Breaking Changes

- `LimitOrderMarginState::new(...)` has been removed. Construct margin state via
  the new `LimitOrderMarginState::from_orders(orders, position)`, which takes an
  iterator of the new `LimitOrderSummary` type plus the trader's current signed
  base-lot position.
- `LimitOrderMarginState` gained new fields (`lowest_ask`, `highest_bid`,
  `total_reduce_only_ask_base_lots`, `total_reduce_only_bid_base_lots`) and can
  no longer be built with a struct literal (an internal marker field enforces
  construction through `from_orders`/`empty`). Any code building this struct
  directly will need to switch to the canonical constructors.
- `LimitOrder::aggregate_margin_state(orders)` now requires an additional
  `position: SignedBaseLots` argument.
- Margin requirements can increase for accounts with resting limit orders priced
  away from mark (bids above mark / asks below mark), since margin now reserves
  the potential adverse-fill loss on those orders — including reduce-only
  orders, which previously contributed no margin at all.

### Consumer Notes

- New public type `LimitOrderSummary` (side, price, remaining size, reduce-only
  flag) is the input to `LimitOrderMarginState::from_orders`.
- New accessors `LimitOrderMarginState::total_ask_base_lots()`,
  `total_bid_base_lots()`, `lowest_ask()`, and `highest_bid()` are available for
  inspecting aggregated order state.
- If you compute margin off this crate, re-verify expected collateral values for
  accounts with resting orders away from mark price, since totals may now be
  higher than before.

## v0.3.2 - 2026-07-07

Source Phoenix commit: `84c3feb1a2f5d9b4e94f9372a706b0e3e3c88b0e`

### Summary

- Added a margin/liquidation simulation API to `math`:
  `TraderPortfolio::simulate_position_fill`, `simulate_margin`, and
  `simulate_margin_scenarios`, along with supporting types
  (`MarginSimulationAction`, `SimulateMarginParams`,
  `SimulateMarginScenariosParams`, `MarginScenario`, `SimulatedMargin`,
  `SimulatedPositionFill`, `SimulatedMarginScenarios`, etc.) for projecting
  fills, order changes, funding, and mark-price moves against cross or isolated
  margin.
- Added a standalone closed-form liquidation price solver,
  `calculate_liquidation_price_usd`, with its
  `CalculateLiquidationPriceUsdInput` input struct.
- Fixed the sign of unsettled funding in `math` margin calculations so a
  positive value means funding owed _to_ the trader and a negative value means
  funding owed _by_ the trader, matching the on-chain contract's
  `calculate_funding_payment` semantics.

### Breaking Changes

- The unsettled-funding sign fix changes the numeric sign of `unsettled_funding`
  returned from portfolio and position margin calculations in `math`; consumers
  reading this field must update their sign handling.
- `PhoenixStateError` gained a new variant, `InvalidMarginSimulationInput`;
  downstream exhaustive matches on this enum need an added arm.

### Consumer Notes

- Use the new simulation API to preview the margin and liquidation-price impact
  of fills, order placement/cancellation, collateral adjustments, funding
  settlement, and mark-price moves before submitting transactions.
- `simulate_margin_scenarios` lets you branch several labeled scenarios off one
  shared baseline for efficient side-by-side what-if comparisons.
- Isolated-margin simulations require all actions to target a single symbol;
  mixing symbols now errors with `InvalidMarginSimulationInput`.

## v0.3.1 - 2026-06-30

Source Phoenix commit: `cf8419bc21f9f539306198f7c08d0aca14a39580`

### Summary

- Adds trader preference bits (initially `disable_collateral_sweep`) to trader
  registration/account state, plus a new zero-copy `accounts::orderbook` view
  for reading FIFO best bid/ask and book sides directly from account bytes.
- Reworks numeric fields across `accounts` views (orderbook, perp asset map,
  trader, stop-losses, conditional orders, spline collection, withdraw queue)
  from raw integers to typed quantity wrappers (`Ticks`, `BaseLots`,
  `QuoteLots`, `SignedBaseLots`, `SignedQuoteLots`, etc.).
- Unifies the `Side` enum on a single `phoenix-rise-math::Side` shared by `ix`,
  `events`, and `types`, removing duplicate per-crate definitions.
- Adds a `solana-signature` dependency to `api`/`litesvm-test` and bumps the
  shared Rise Rust crates to `0.3.1`.

### Breaking Changes

- `Side` now serializes as lowercase `"bid"`/`"ask"` (previously
  `"Bid"`/`"Ask"`); deserialization still accepts the old PascalCase as an
  alias, but consumers matching on serialized output must update.
- Account-view fields that were previously raw `u64`/`i64`/`i128` (prices, lot
  sizes, funding fields, withdraw-throttle budgets, etc.) are now typed wrappers
  — call `.as_inner()` instead of using them as plain integers.
- CPI `RegisterTraderArgs` and ix `RegisterTraderParams` gained a new
  `trader_preference_bits: u32` field; existing struct literals must add it (or
  switch to the new `RegisterTraderArgs::new(...)` constructor/builder methods).
- `trader::Trader` is now the primary zero-copy trader view; some position
  count/capacity accessors moved off `TraderHeader` onto `Trader`, and owned
  `Trader.max_positions` changed from `u64` to `u32`.

### Consumer Notes

- Use `accounts::trader::Trader::try_from_account_bytes` for a combined
  header+positions view, and the new `trader::preferences` helpers
  (`TraderPreferenceKind`, `TraderPreferences`, `TraderPreferenceFlags`) to
  read/set `disable_collateral_sweep`.
- Build preference-aware registrations with
  `RegisterTraderArgs::new(...).with_disable_collateral_sweep(true)` (or
  `.with_trader_preference(...)`) instead of hand-assembling the bitfield.
- The new `accounts::orderbook::Orderbook` view covers FIFO limit orders only
  (no spline liquidity); use Hawkeye's BBO view when full market liquidity is
  needed. `accounts::permission::PermissionAccount::try_from_buffer` adds a
  zero-copy alternative to the existing copying decoder.

## v0.3.0 - 2026-06-29

Source Phoenix commit: `08743d17a258f1f4d6e744bd0dc5d64bc0a9d580`

### Summary

- Bumps all `phoenix-rise` workspace crates from `0.2.0` to `0.3.0`.
- Upgrades all `solana-*` workspace dependencies from the `~2.x` range to `~3.x`
  (e.g. `solana-pubkey ~2.4` → `~3.0`, `solana-rpc-client ~2.3` → `~3.1`).
- Pins `litesvm` at `=0.11.0` (up from `0.7`) and adds the `borsh` feature to
  the workspace `solana-pubkey` dependency.

### Breaking Changes

- **Solana SDK `~2.x` → `~3.x` (all components)**: Every `solana-*` dependency
  across `accounts`, `ix`, `core`, `sdk`, `workspace`, and example `programs`
  has been bumped to the `~3.x` range. Downstream crates that pin any `solana-*`
  dependency to `~2.x` will encounter dependency-resolution conflicts and must
  be updated.
- **`core` — `SimulationFailed` error variant inner type changed**:
  `PhoenixHawkeyeClientError::SimulationFailed` now stores a converted
  (`err.into()`) type rather than the raw Solana `~2.x` error, reflecting a type
  change in the Solana 3.x RPC API. Any `match` arm destructuring the inner
  value must be updated to the new Solana 3.x error type.
- **`solana-pubkey` gains `borsh` feature**: The workspace `solana-pubkey`
  dependency now enables `borsh` in addition to `curve25519` and `serde`. This
  pulls in `borsh` transitive dependencies for all crates that share the
  workspace pin; crates that previously relied on `borsh` being absent from this
  dep tree may see new transitive crate additions.

### Consumer Notes

- Update all `solana-*` entries in your `Cargo.toml` or workspace to the `~3.x`
  range to stay compatible with this release.
- The minimum supported Rust version for example programs is now `1.89.0`
  (edition `2024`); the main workspace `rust-version` was already `1.89.0`.
- `litesvm` is now pinned at `=0.11.0`. Test helpers that seed accounts via
  `airdrop` must use at least `890_880` lamports (the rent-exempt minimum for an
  empty account) rather than `1`; the newer `litesvm` enforces rent-exemption
  requirements that the prior version did not.
- The `events` source-parity tests comparing `phoenix-rise-events` against the
  internal Phoenix exchange source have been removed. Serialization
  compatibility is still covered by
  `representative_event_serialization_matches_phoenix_exchange`.

## v0.2.0 - 2026-06-29

Source Phoenix commit: `d087e01780d6f8cfadb10005c6607f7de59d3de2`

### Summary

- **Rust SDK split into focused crates**: `phoenix-rise` is now a facade over
  dedicated crates: `phoenix-rise-accounts`, `phoenix-rise-api`,
  `phoenix-rise-core`, `phoenix-rise-events`, `phoenix-rise-ix`,
  `phoenix-rise-math`, `phoenix-rise-types`, and `phoenix-rise-litesvm-test`.
  The facade still imports as `phoenix_rise`, but the implementation and feature
  graph are now organized by use case.
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
  - The old broad `core` feature is now a compatibility alias for `tx-builder`.
  - `sdk` now means account views, instruction builders, math, and domain types,
    without API transport or RPC-backed transaction helpers.
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
  `phoenix-rise-litesvm-test` directly and import `SdkLocalnetContext`,
  `default_sdk_localnet_fixture`, and related helpers from that crate.
- **Standalone permission instruction module was removed from `ix`**: callers
  using the old `phoenix_rise::ix::permission::*` builders should migrate to the
  current delegated onboarding / permission flows exposed by the SDK examples
  and the account/PDA helpers in `phoenix_rise::accounts`.
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

- Added builder-initiated trader onboarding support: two new `ExchangeClient`
  methods — `build_register_ixs` and `send_register_ixs` — let a builder
  register and onboard a trader without a referral code, using
  `POST /v1/exchange/build-register-ixs` and
  `POST /v1/exchange/send-register-ixs`.
- Six new public types exported from the crate root (under the `sdk` feature):
  `BuildRegisterIxsRequest`, `BuildRegisterIxsResponse`,
  `SendRegisterIxsRequest`, `SendRegisterIxsResponse`, `ApiInstructionResponse`,
  and `ApiAccountMeta`.
- New `builder_onboarding_tx` example (requires `solana-keypair` feature)
  demonstrates the full build-sign-submit flow; run with `--trader-keypair-path`
  and an optional `--fee-payer-keypair-path`.
- Documentation updated to clarify the three distinct onboarding paths:
  access-code (`/v1/invite/activate`), referral-code
  (`/v1/referral/activate-tx`), and builder (`/v1/exchange/build-register-ixs` +
  `/v1/exchange/send-register-ixs`).

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- To use the new builder onboarding APIs, call
  `client.exchange().build_register_ixs(...)` with a `BuildRegisterIxsRequest`
  (trader authority, fee payer, optional `max_positions`), sign the returned
  instructions locally, then submit the base64-encoded signed transaction via
  `client.exchange().send_register_ixs(...)`. The API validates, co-signs,
  simulates, and broadcasts the transaction.
- `ApiInstructionResponse` and `ApiAccountMeta` are the deserialized instruction
  shapes returned by `build_register_ixs`; downstream code that builds
  transactions from the response will need to import these types.
- The `max_positions` field on both request types is optional (serialized only
  when `Some`); defaults to 128 on the server side.
- `trader_pda_index` and `trader_subaccount_index` on `SendRegisterIxsRequest`
  are also optional; omit them to accept server defaults.

## v0.1.15 - 2026-06-25

Source Phoenix commit: `823bb5e64efafb221f37a7e0a8f8720da843741c`

### Summary

- Added new public exports for inspecting trader activation status before
  building a referral activation transaction: `ReferralActivationTraderStatus`
  enum, `ReferralActivationTraderStatusError` error enum,
  `fetch_referral_activation_trader_status` async function,
  `has_referral_activation_capabilities` const function, and
  `referral_activation_trader_status` function.
- The referral activation transaction now handles missing trader accounts
  automatically: when the trader account does not exist, the built transaction
  includes `register_trader` followed by delegated onboarding in a single atomic
  transaction. No separate registration step is required.
- `ReferralActivationTraderStatus::should_include_register_trader()` returns
  `true` only for the `Missing` variant, making it straightforward to
  conditionally include the registration instruction when constructing
  activation transactions manually.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- The `referral_activation_tx` example no longer requires
  `--register-if-missing`; the flag is accepted but silently ignored for
  backward compatibility. Remove it from any example invocations you have
  documented.
- If you call `fetch_referral_activation_trader_status` and receive
  `ReferralActivationTraderStatus::Missing`, pass
  `should_include_register_trader() == true` (and a valid `max_positions` in the
  range 32–128) to your transaction builder. Accounts in the `Registered` or
  `Activated` states skip registration and only set capabilities.
- `--max-positions` in the example binary now enforces a minimum of 32
  (previously 1). This affects the example only and does not change the library
  API.

## v0.1.14 - 2026-06-24

Source Phoenix commit: `f2a0ac7b66eae85ef8ddb3f21ff68ce6f7063754`

### Summary

- Added two new unauthenticated referral activation endpoints to `InviteClient`:
  `get_referral_activation_permission()`
  (`GET /v1/referral/activation-permission`) and `activate_referral_tx()`
  (`POST /v1/referral/activate-tx`), enabling a wallet-side transaction-signing
  flow for referral activation.
- New public types `ActivateReferralTxRequest`, `ActivateReferralTxResponse`,
  and `ReferralActivationPermissionResponse` are re-exported from the crate root
  under the `sdk` feature flag.
- Added `ExchangeStatusView` struct (fields: `active`, `gated`,
  `withdrawals_available`) and a corresponding `ExchangeClient::get_status()`
  method (`GET /exchange/status`).
- Added `withdrawals_available: bool` field to `ExchangeStateSnapshot` and the
  `ExchangeStatusChanged` variant of `ExchangeDeltaOp`; both default to `true`
  when the field is absent from the wire payload (backward-compatible
  deserialization).
- Added a `referral_activation_tx` example (requires `solana-keypair` feature)
  demonstrating the full end-to-end activate-tx wallet integration flow.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- If you pattern-match on `ExchangeDeltaOp::ExchangeStatusChanged`, you must now
  bind or ignore the new `withdrawals_available` field; omitting it is a compile
  error.
- `ExchangeStateSnapshot` gains a `withdrawals_available` field; struct literal
  construction (if any) must be updated to include it.
- `ExchangeStatusView` is a new type exported from `phoenix_rise_types`; no
  migration needed unless a local type of the same name exists.
- The `activate_referral_tx` flow requires the trader account to exist on-chain
  before calling `/v1/referral/activate-tx`; use `--register-if-missing` (or
  call `register_trader` first) if the account may not exist yet.

## v0.1.13 - 2026-06-23

Source Phoenix commit: `d1ccda811fc68e0cbf1fbf5d3e6d0235d508c3db`

### Summary

- The `phoenix-rise` crate was consolidated from a multi-root workspace layout
  (`sdk/src/`, `ix/src/`, `math/src/`, `types/src/`) into a standard
  single-crate layout (`src/`). Public module paths (`phoenix_rise::ix`,
  `::math`, `::types`) are unchanged; this is a transparent restructure for
  existing crate consumers.
- New instruction builders for permission account management:
  `create_permission_ix` and `set_permission_delegated_ix`, with supporting
  constants `TRADER_ONBOARDING_PERMISSION` and `TRADER_MANAGEMENT_PERMISSION`
  (in `phoenix_rise::ix`).
- New trader capability constants exported from `phoenix_rise::ix`:
  `TRADER_CAPABILITY_HOT`, `TRADER_CAPABILITY_CAN_PLACE_LIMIT`,
  `TRADER_CAPABILITY_CAN_PLACE_MARKET`, `TRADER_CAPABILITY_CAN_RISK_INCREASE`,
  `TRADER_CAPABILITY_CAN_DEPOSIT`, `TRADER_CAPABILITY_CAN_WITHDRAW`, and
  `REQUIRED_TRADER_CAPABILITIES`. Also adds `is_trader_ready` helper.
- New `delegated_trader_management_onboarding` example demonstrating the full
  off-chain integrator flow for granting and consuming delegated
  trader-onboarding permissions.

### Breaking Changes

- If you depended on `phoenix-rise-types` as a standalone crate, it is no longer
  published. Switch to `phoenix_rise::types` (requires the `types` feature flag
  on `phoenix-rise`).

### Consumer Notes

- Update your `Cargo.toml` to `phoenix-rise = "0.1.13"`.
- Delegated onboarding flows that previously required manual capability bitmask
  construction can now use the exported `TRADER_CAPABILITY_*` constants and
  `is_trader_ready` to check readiness without hardcoding bit values.
- The new `permission` instruction builders (`create_permission_ix`,
  `set_permission_delegated_ix`) are needed for integrators who programmatically
  manage delegated trader-onboarding budgets; see the
  `delegated_trader_management_onboarding` example for the end-to-end flow.

## v0.1.12 - 2026-06-22

Source Phoenix commit: `a419e23d7d1b2a3e37696d76e85fac7f0a023f5e`

### Summary

- Three new cancellation/maintenance helpers added to `PhoenixTxBuilder`:
  `build_cancel_all_orders`, `build_cancel_up_to`, and `build_uncross_crank`
  (the crank is permissionless — no trader signer required).
- Matching low-level params types and instruction constructors exported from
  `phoenix_rise_ix`: `CancelAllParams`/`create_cancel_all_ix`,
  `CancelUpToParams`/`create_cancel_up_to_ix`,
  `UncrossCrankParams`/`create_uncross_crank_ix`.
- New granular Cargo features: `sdk`, `types`, and `test-fixture`. The default
  feature set changed from `[]` to `["sdk"]`.
- New `test-fixture` feature exposes `phoenix_rise::test_fixture` with
  `LiteSVM`-backed `SdkLocalnetContext`, fixture deserialization types, and a
  bundled `default-localnet.json` for in-process localnet tests.
- `ix` program constants (`PHOENIX_PROGRAM_ID`, `PHOENIX_LOG_AUTHORITY`,
  `PHOENIX_GLOBAL_CONFIGURATION`) now compile under `target_os = "solana"`,
  making the `ix` sub-crate usable in on-chain programs.

### Breaking Changes

- **Default features changed from `[]` to `["sdk"]`**: consumers who previously
  added `phoenix-rise` without `default-features = false` had near-zero
  transitive deps; they now pull in the full SDK set (`reqwest`, `tokio`,
  `parking_lot`, etc.). Add `default-features = false` to restore a minimal
  build.
- **`solana-keypair` and `ed25519-dalek` re-exports now also require `sdk`**:
  `PhoenixWalletSessionManager`, `PhoenixSolanaKeypairAuthSigner`,
  `default_solana_keypair_path`, and `PhoenixEd25519ServiceAuthSigner` are gated
  on `all(feature = "sdk", feature = "solana-keypair/ed25519-dalek")`. Consumers
  using `default-features = false` with only those features enabled will see
  missing-item compile errors; add `features = ["sdk"]` to restore the exports.

### Consumer Notes

- `rust_decimal = []` was previously an empty stub; it now activates
  `dep:rust_decimal`. The feature was always advertised but did nothing; it now
  pulls in the crate.
- `utoipa` and `opentelemetry` features now implicitly enable `types`/`sdk`
  respectively, so enabling them without `default-features = false` is safe but
  may add transitive deps if you were relying on those features being lighter.
- The `test-fixture` feature is intended for test harnesses only; it adds
  `litesvm 0.7` and `solana-commitment-config` as optional deps and is not
  covered by the `sdk` default.

## v0.1.11 - 2026-06-22

Source Phoenix commit: `fd9d044ad0e76e6bcbef1333b1ebc8648f511b7a`

### Summary

- **Flight per-order fee override**: `ProxyInstructionParams::builder()` now
  accepts `.fee_bps_override(bps: u64)` (0–10 000). When set,
  `create_proxy_instruction_ix` emits the `proxy_instruction_with_fee_override`
  variant with a different discriminant and a Borsh `Option<u64>` prefix before
  the inner instruction data. `PhoenixFlightClient` gains a matching
  `try_wrap_order_instruction_with_fee_bps_override(ix, trader_wallet, fee_bps_override: Option<u64>)`
  method; the existing `try_wrap_order_instruction` is unchanged and delegates
  to it with `None`.
- **WebSocket subscription event channel**: `WsSubscriptionEvent` is now a
  public export. Call `client.subscription_event_receiver()` (once) to receive
  `Status` and `Error` lifecycle events for every subscription acknowledgement
  or rejection. New `PhoenixWSClient::new_with_connection_status_and_auth`
  constructor combines authenticated and connection-status modes.
- **Automatic auth reconnect on WS close**: The client now detects auth-related
  close codes (4401, 4403, 1008) and reason strings (`access_token_expired`,
  `invalid_access_token`, etc.), refreshes the session, reconnects, and
  resubmits active subscriptions transparently.
- **`ExchangeMarketConfig` gains
  `stats_snapshot: Option<MarketStatsSnapshot>`**: The new field is
  `#[serde(default)]` so deserialization of existing payloads is unchanged, but
  Rust struct literal construction must now include `stats_snapshot: None`.
- **New `JsSafeI64` type** added to `phoenix_rise::types::js_safe_ints` (mirrors
  `JsSafeU64` for signed values; serialises as a decimal string).

### Breaking Changes

- `AdminChallengeRequest` and `AdminLoginRequest` are removed from
  `phoenix_rise::types::auth`. Code referencing these types will fail to
  compile.
- `CreateServiceAccountRequest.role` and `ServiceAccountDto.role` fields are
  removed. Struct construction and pattern matching on these types must be
  updated.
- `ExchangeMarketConfig` requires a new `stats_snapshot` field for struct
  literal construction (e.g. in tests). Add `stats_snapshot: None` to any
  existing struct literals.

### Consumer Notes

- The `fee_bps_override` builder method validates the range at `build()` time
  and returns `PhoenixIxError::InvalidFeeBpsOverride` for values above 10 000.
  Pass `None` (or omit the call) to continue using the builder's registered fee.
- `subscription_event_receiver()` takes ownership of the channel and can only be
  called once per client instance; subsequent calls return `None`.
- `ServerMessage` now has a `SubscriptionStatus(SubscriptionStatusMessage)`
  variant — match arms with `ServerMessage::Other` that previously caught
  subscription-status frames will now route through the new variant instead.

## v0.1.10 - 2026-06-17

Source Phoenix commit: `e01d6bf79f112b8f9c7c6e7e3ad84fb83050eb43`

### Summary

- Added the **Phoenix Hawkeye** read-only view layer: instruction builders,
  typed return-data structs, a decode pipeline, and a high-level async RPC
  client for querying margin, per-asset risk, liquidation prices, BBO, and
  funding state via simulated transactions.
- New top-level exports from `phoenix_rise`: `PhoenixHawkeyeClient`,
  `HawkeyeSimulation<T>`, `PhoenixHawkeyeClientError`, `HawkeyeReturnData`,
  `HawkeyeReturnDataError`, all `View*Return` structs, `Hawkeye*ViewAccounts`
  account-list structs, low-level instruction builders
  (`create_hawkeye_view_*_ix`), and decode helpers
  (`decode_hawkeye_return_data`, `decode_hawkeye_return`).
- `PhoenixTxBuilder` gains nine new `build_hawkeye_view_*` methods that
  construct Hawkeye simulation instructions from a `PhoenixMetadata` context,
  with both PDA-derived and explicit-trader variants.
- Three new mandatory runtime dependencies: `solana-rpc-client-api ~2.3`,
  `solana-transaction-error ~2.2`, and
  `solana-transaction-status-client-types ~2.3`;
  `solana-compute-budget-interface ~2.2` is also added. Downstream `Cargo.toml`
  files that pin Solana crate ranges tightly may need adjustments.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `PhoenixHawkeyeClient::new(rpc_url, &metadata)` is the recommended entry
  point; it defaults to an unsigned simulation fee payer
  (`HAWKEYE_SIMULATION_FEE_PAYER`) and a 1.4M CU limit, both overridable via
  builder methods.
- All Hawkeye view calls are **read-only simulations** — no fee payer signature
  required and no state is mutated on-chain.
- Return data is versioned (`HAWKEYE_RETURN_VERSION = 1`); callers should assert
  `version` matches if they hard-code struct layouts.
- `ViewBboReturn::best_bid_ticks()` / `best_ask_ticks()` return `Option<u64>`
  based on presence flags — a `None` means no resting orders on that side, not
  an error.

## v0.1.9 - 2026-06-16

Source Phoenix commit: `00d34904a9c9e981dcc62666906e2985a98624d9`

### Summary

- **New `opentelemetry` feature flag**: adds opt-in W3C trace-context
  propagation for outbound HTTP requests and WebSocket handshakes. Enable with
  `phoenix-rise = { features = ["opentelemetry"] }`. When active,
  `PhoenixHttpClientBuilder` exposes `with_trace_context_provider(...)` and
  `PhoenixWSClient` gains `new_with_trace_context_provider` and
  `new_with_connection_status_and_trace_context_provider` constructors.
- **`TransferCollateralParamsBuilder` gains an optional `permission_account`**:
  pass a secondary position-authority permission account via
  `.permission_account(pubkey)`. When set, it is appended as a writable account
  at the end of the instruction's account list.
- **`PlaceMarketOrderDelegated` removed from Flight routing**:
  `is_flight_routable_instruction` now returns `false` for delegated market
  orders. The discriminant function remains public but is no longer matched by
  the Flight client.
- Dependency updates: `proptest` → `1.11.0`, `getrandom` → `0.4.2`, and several
  transitive `windows-sys` pins resolved to earlier versions.

### Breaking Changes

- **`is_flight_routable_instruction` behavior change**:
  `PlaceMarketOrderDelegated` instructions are no longer considered
  flight-routable. If your code wrapped delegated market orders via
  `PhoenixFlightClient::try_wrap_order_instruction`, those instructions must now
  be submitted directly rather than through Flight.

### Consumer Notes

- The `opentelemetry` feature does not install a global propagator automatically
  — callers must call `opentelemetry::global::set_text_map_propagator(...)`
  (e.g., with `TraceContextPropagator`) before trace headers will be injected
  into requests.
- The `permission_account` builder method is additive and optional; existing
  `TransferCollateralParams` construction code requires no changes.

## v0.1.8 - 2026-06-16

Source Phoenix commit: `6487be852f3a382717f345a43fd956af3fba1766`

### Summary

- Upgraded `rand` from 0.9 to 0.10 (now a workspace dependency); the `rand::Rng`
  trait import in math tests is replaced by `rand::RngExt`.
- Rate-limit retry logic now applies ±15% jitter to the fallback delay when no
  `Retry-After` header is present, reducing thundering-herd retries.
- `Retry-After` header parsing now accepts both integer seconds and HTTP-date
  (`RFC 2822`) values; previously only integer seconds were recognized.
- The `max_delay` field on `RateLimitRetryConfig` now bounds only the jittered
  fallback delay; explicit `Retry-After` values are bounded by `max_total_wait`
  instead (doc clarification, behavior change for large `Retry-After` values).

### Breaking Changes

- **`rand` 0.10 workspace dependency**: if your crate depends on `phoenix-rise`
  and also uses `rand`, you may need to align to `rand = "0.10"`. The
  `rand::Rng` trait is replaced by `rand::RngExt` in 0.10 — any code importing
  `rand::Rng` via a shared dependency tree will need updating.
- **`RateLimitRetryConfig::max_delay` semantics changed**: previously capped all
  retry delays including explicit `Retry-After` header values; now only caps the
  jittered fallback delay. If you set a small `max_delay` to bound
  `Retry-After`-driven waits, use `max_total_wait` instead.

### Consumer Notes

- No changes to public API surface beyond the `RateLimitRetryConfig`
  doc/semantic fix above.
- The jitter on fallback delays (±15%) is applied automatically; no
  configuration required or available.
- If your server sends `Retry-After` as an HTTP-date string, the client will now
  correctly parse and honor it.

## v0.1.7 - 2026-06-16

Source Phoenix commit: `65fc5aad3e13c249bcc606b410059b8adbc61e2d`

### Summary

- **Delegated market orders**: New `MarketOrderDelegatedParams` /
  `create_place_market_order_delegated_ix` instruction builder and
  `PhoenixTxBuilder::place_market_order_delegated` for signing via a wallet that
  differs from the trader account authority. Delegated market-order instructions
  are now routable through `PhoenixFlightClient`.
- **Session manager abstraction**: New `PhoenixSessionManager` trait with two
  built-in implementations — `PhoenixMemorySessionManager` (any `AuthSession`)
  and `PhoenixWalletSessionManager` (Solana keypair login, `solana-keypair`
  feature). Concurrent refreshes are coalesced. Pass via
  `PhoenixHttpClientBuilder::with_session_manager`.
- **Authenticated WebSocket handshake**: `PhoenixClient::new_from_env_with_auth`
  / `from_env_with_auth` / `from_env_with_http_client` propagate the HTTP
  client's auth lifecycle to WebSocket connections; the handshake now sends
  `Authorization` and `Sec-WebSocket-Protocol` JWT headers and auto-retries on
  401/403/409.
- **WS connection status observable**: `PhoenixClient::connection_status()` and
  `subscribe_connection_status()` expose a `watch::Receiver<WsConnectionStatus>`
  for the full client's reconnection state.
- **`MaxLiquidationSizeUpdated` WS event**: New
  `ExchangeMarketParameterUpdate::MaxLiquidationSizeUpdated` variant tracked by
  `ExchangeCacheMarketChangeKind::MaxLiquidationSize`.

### Breaking Changes

- **`TradersClient` API replaced**: `get_trader(authority)`,
  `get_trader_internal(authority, pda_index)`, and
  `get_trader_state(authority, pda_index)` are removed. Use
  `get_trader_by_pubkey(&trader_pda)` instead, which takes the trader PDA
  directly. The convenience top-level methods `PhoenixHttpClient::get_traders`
  and `get_trader_state` are also removed.
- **`get_permission_address` removed** from the public re-export list in
  `phoenix_rise`.
- **New `ExchangeMarketParameterUpdate::MaxLiquidationSizeUpdated` variant**:
  exhaustive `match` arms over this enum must add a handler for the new variant.
- **New `PhoenixWsError::Authentication` variant**: exhaustive `match` arms over
  this error type must add a handler.

### Consumer Notes

- `PhoenixEnv::from_api_url(api_url)` is a new convenience constructor that
  derives the WebSocket URL automatically, including preserving reverse-proxy
  path prefixes (e.g. `https://gateway.example.com/phoenix` →
  `wss://gateway.example.com/phoenix/v1/ws`).
- `PhoenixHttpClient::from_url_with_wallet_keypair(url, keypair)`
  (`solana-keypair` feature) creates a fully authenticated client in one step.
- `PhoenixHttpClient::post_json` is now public.
- `is_auth_recovery_error` is now a public export for downstream error
  classification.
- `async-trait 0.1` is a new direct dependency of `phoenix-rise`.

## v0.1.6 - 2026-06-11

Source Phoenix commit: `443ff8dfd64a9e7d35960b8b1946b3248d6681e5`

- Package: `phoenix-rise`
- Target repo version: 0.1.5
- Phoenix version: 0.1.5 -> 0.1.6

### Summary

- **New:** `PhoenixHttpClient` and `TradesClient` now expose
  `get_user_liquidation_history()`, hitting
  `GET /v1/users/{authority}/liquidation-history`. Use
  `UserLiquidationHistoryQueryParams` (with builder methods `with_pda_index`,
  `with_subaccount_index`, `with_symbol`, `with_limit`, `with_cursor`) to filter
  results.
- **New types re-exported from crate root:** `UserLiquidationHistoryPoint`,
  `UserLiquidationHistoryResponse`, `UserLiquidationHistoryKind`,
  `UserLiquidationHistoryType`, `UserLiquidationHistoryRole`.
- **New optional basis-point fields on `ExchangeRiskFactors`:** each existing
  `f64` percentage field (`maintenance`, `backstop`, `high_risk`, `upnl`,
  `upnl_for_withdrawals`, `cancel_order`) now has a companion `Option<u16>`
  `_bps` field. `ExchangeLeverageTier` gains
  `limit_order_risk_factor_bps: Option<u16>`. All are
  `#[serde(default, skip_serializing_if = "Option::is_none")]`; existing
  deserialization is unaffected.
- **WebSocket parameter-update events extended:** `CancelRiskFactorUpdated`,
  `UpnlRiskFactorUpdated`, and `UpnlRiskFactorForWithdrawalsUpdated` variants of
  `ExchangeMarketParameterUpdate` now carry optional `previous_bps`/`new_bps`
  fields (additive, backward-compatible).

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- **Prefer `_bps` fields for risk factor arithmetic.** The server now populates
  them (e.g., `maintenance_bps: Some(5000)` = 50%). When present, these should
  be considered authoritative; the legacy `f64` fields may carry
  basis-point-scale values from the API rather than percentages in some contexts
  — divide by 100 only for human-readable display, as shown in the updated
  `http_client` example.
- `ExchangeRiskFactors` struct literals in your own code now require the six new
  `Option<u16>` fields; either supply `None` or use `..Default::default()` (if
  you derive `Default`) to stay forward-compatible.

## v0.1.5 - 2026-06-10

Source Phoenix commit: `f0c4a154e63048a8517e72e8a6a8b806e3768cd0`

- Package: `phoenix-rise`
- Target repo version: 0.1.4
- Phoenix version: 0.1.2 -> 0.1.5

### Summary

- **Delegated trader onboarding**: New `OnboardTraderDelegatedParams` /
  `create_onboard_trader_delegated_ix` and `get_permission_address` PDA helper
  are now re-exported from the crate root, enabling a delegated authority to
  onboard traders without requiring the trader's own signature.
- **Market calendar APIs**: Three new `PhoenixHttpClient` methods —
  `get_next_market_calendar_transition(symbol)`, `get_market_calendar(symbol)`,
  and `get_commodity_market_calendar()` — backed by new types
  (`NextMarketCalendarTransition`, `MarketCalendarResponse`,
  `CommodityMarketCalendarResponse`, and supporting schedule types).
- **Market public metadata**: `ExchangeMarketConfig` and
  `ExchangeMarketSnapshot` now carry an optional
  `metadata: Option<MarketPublicMetadata>` field (name, description, logo URI,
  CoinGecko ID, etc.). Live updates arrive via a new
  `ExchangeDeltaOp::MarketMetadataUpdated` delta op and can be queried from
  `PhoenixExchangeCacheStore` via `market_metadata*` helpers.
- **Forward-compatible exchange delta deserialization**: `ExchangeDeltaOp` and
  `ExchangeMarketParameterUpdate` now deserialize unrecognized server variants
  as `Unknown` (via `#[serde(other)]`) instead of returning a deserialization
  error.
- **Auth hardening**: Access-token expiry is now derived from the JWT `exp`
  claim (with server-supplied `expires_in` as fallback); expired tokens are
  omitted from refresh request `Authorization` headers; `session_missing` server
  error is now treated as `InvalidRefreshToken`.
- **MSRV lowered** from 1.86.0 to 1.84.0.

### Breaking Changes

- **`ExchangeMarketConfig` and `ExchangeMarketSnapshot` gained a new `metadata`
  field.** Any code that constructs these structs with named-field syntax
  (including exhaustive struct patterns) must add `metadata: None`.
- **`ExchangeWsMarkPriceParameters` gained two new fields**
  (`book_hard_stale_multiplier: u8`, `oracle_hard_stale_multiplier: u8`).
  Named-field construction must add both (both default to `0` on
  deserialization).
- **`ExchangeDeltaOp` and `ExchangeMarketParameterUpdate` gained `Unknown`
  variants; `ExchangeCacheMarketChangeKind` gained `Metadata`.** Exhaustive
  `match` arms on any of these enums will fail to compile and must add a
  wildcard or explicit `Unknown`/`Metadata` arm.
- **`ServiceAccountDto` gained `description: Option<String>`.** Named-field
  struct construction must add this field.

### Consumer Notes

- The new `PhoenixHttpClient::get_json` / `get_json_with_query` methods provide
  a typed escape hatch for endpoints not yet covered by named helpers.
- `CommodityMarketStateView` and
  `CommodityMarketDaySchedule`/`CommodityMarketHoursRange`/`CommodityMarketCalendarView`/`CommodityMarketCalendarResponse`
  are newly exported from the crate root, completing the commodity-calendar
  surface.
- The repository URL in Cargo metadata has changed from
  `Ellipsis-Labs/rise-public` to `Ellipsis-Labs/rise`; update any documentation
  links accordingly.
