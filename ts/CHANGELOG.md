# Changelog

Entries are drafted by Phoenix Rise sync PRs. Review and edit each
entry in this repo before merging.

## v0.4.27 - 2026-06-09

Source Phoenix commit: `32611fa7f803cf601cab67b6ca5dce5d4a70fc75`

### Summary

- Flame proxy-deposit flow added for sponsored USDC deposits that bypass user-owned rent accounts
- Market calendar APIs and per-market public metadata (`name`, `logoUri`, `calendar`, `coinGeckoId`, etc.) are now available in the HTTP client and exchange cache
- Auth robustness improved: expired sessions are proactively refreshed before WS connect, transient refresh failures retry with backoff, and `auth: "required"` HTTP requests fail immediately without a valid session

### Breaking Changes

- **`apiKey` removed** from `PhoenixHttpClientConfig` and `createPhoenixClient`. Any call site passing `apiKey` will now fail to compile; remove the field.
- **`activateInviteWithReferral` renamed to `activateReferral`** on the invite client. The backing route changed from `POST /v1/invite/activate-with-referral` to `POST /v1/referral/activate`, which now requires an active auth session for the same authority wallet.
- **`fromSnapshot` now reads `expiresAt` from the JWT `exp` claim** (not the snapshot's `expiresAt` field) and throws if `exp` is absent. Access tokens lacking `exp` are no longer accepted.
- **`PhoenixExchangeStoreState` interface gained three required fields** (`marketMetadataBySymbol`, `marketMetadataByAssetId`, `marketMetadataByPubkey`). Downstream code that constructs or types this shape directly (e.g. in tests) must add these fields.
- **`ExchangeCacheMarketChangeKind` union gained `"metadata"`**. Exhaustive `switch` or conditional chains over this type without a default branch will fail to compile unless updated.

### Consumer Notes

- **New Flame deposit helpers**: `buildFlameDepositFundingFlow`, `deriveFlameDepositAddresses`, `deriveFlameProxyAuthorityAddress`, `deriveFlameDepositAddress`, `FLAME_PROGRAM_ADDRESS`, `FlameProgramAddress`. Use `buildFlameDepositFundingFlow` for sponsored deposits; `buildDepositFlow` remains the direct Ember + Phoenix path.
- **New market calendar endpoints** on the markets client: `getMarketCalendar(symbol)`, `getMarketCalendarById(id)`, `getCommodityMarketCalendar()`, `getNextMarketCalendarTransition(symbol)`. Matching types (`MarketCalendarResponse`, `MarketCalendarRecord`, `MarketCalendarView`, `NextMarketCalendarTransition`, etc.) are now exported from the public surface.
- **Per-market public metadata** is now accessible via `selectExchangeMarketMetadata(symbol)` / `selectPhoenixExchangeMarketMetadata(symbol)`, `cache.marketMetadata(symbol)`, and the `metadata` field on `PhoenixExchangeMarketState`. The `ExchangeMarketConfig` response also carries an optional `metadata` field.
- **`PlaceMarketOrderFlowParams` gains optional `minBaseLotsToFill` and `minQuoteLotsToFill`** for fill-size guarantees on market orders. Omitting them preserves existing behavior (no minimum enforced).
- **`buildSplTokenTransfer`** is now exported for building raw SPL token transfer instructions.
- **Unknown WebSocket delta ops** are now normalized to `{ kind: "unknown", originalKind, payload }` instead of throwing, so clients survive new server-side delta kinds without crashing.

## v0.4.28 - 2026-06-10

Source Phoenix commit: `f0c4a154e63048a8517e72e8a6a8b806e3768cd0`

### Summary

- **New `OnboardTraderDelegated` instruction**: Adds `buildOnboardTraderDelegatedIx`, `buildOnboardTraderDelegated`, and `buildOnboardTraderDelegatedIxResolved` to the public surface. A delegated onboarder keypair can now onboard a trader (optionally registering the account first) without the risk authority signing directly.
- **New client method**: `client.ixs.buildOnboardTraderDelegated({ authority, traderAuthority, permissionAccount, traderPdaIndex?, traderSubaccountIndex? })` is available on all `PhoenixIxClient` instances.
- **New example**: `examples/07-onboard-trader-delegated.ts` demonstrates the full delegated onboarding flow, including permission validation and conditional trader registration.

### Consumer Notes

- The `TraderCapabilityToggleTarget` enum and internal `buildSetTraderCapabilitiesDelegated*` names are intentionally absent from the public surface; use `buildOnboardTraderDelegated` instead.
- Delegated onboarding always enables all six trader capabilities (limit orders, market orders, risk-increasing and risk-reducing trades, deposit, and withdrawal) in a single instruction — there is no partial-capability variant in this release.
- The `permissionAccount` address must be derived from the current risk authority and the onboarder's key; see the new example for the full derivation pattern using `client.pda.getPermissionAddress`.

## v0.4.30 - 2026-06-11

Source Phoenix commit: `7b0e722849fbc44d2d34d481d208db0ea951b78e`

### Summary

- **Risk factor values in `RiskFactors` are now in basis points.** `PhoenixProjectedMarket.riskFactors` fields (`maintenance`, `backstop`, `highRisk`, `upnl`, `upnlForWithdrawals`, `cancelOrder`) now carry bps values (e.g. `5000` = 50%) instead of the previous percentage scale (e.g. `5` = 5%). The cache store, projected-market selector, and margin params builder all consistently prefer the new bps representation.
- **Optional `*Bps` sibling fields added to `ExchangeRiskFactors` and `ExchangeLeverageTier`.** `maintenanceBps`, `backstopBps`, `highRiskBps`, `upnlBps`, `upnlForWithdrawalsBps`, `cancelOrderBps`, and `limitOrderRiskFactorBps` are now available on the HTTP API types; where present they take precedence over the legacy percentage fields.
- **WebSocket risk-factor update events carry optional bps deltas.** `cancelRiskFactorUpdated`, `upnlRiskFactorUpdated`, and `upnlRiskFactorForWithdrawalsUpdated` now include `previousBps`/`newBps` (normalized from server snake_case `previous_bps`/`new_bps`).
- **`buildMarketParamsFromSummary` now validates risk factor inputs strictly.** Passing a non-finite, negative, non-integer, or >10 000 bps value throws an explicit error instead of silently stringifying it.

### Breaking Changes

- **`buildMarketParamsFromSummary` can now throw.** If a `MarketSummary` contains risk factor values that are non-finite, negative, non-integer bps, or exceed 10 000 bps, the function throws instead of returning a params object. Callers that previously relied on unconditional success should wrap the call or validate inputs first.

### Consumer Notes

- The legacy percentage-scale fields on `ExchangeRiskFactors` (`maintenance`, `backstop`, etc.) remain present for backwards compatibility with older API responses; prefer the new `*Bps` fields when available.
- WS consumers parsing `cancelRiskFactorUpdated`, `upnlRiskFactorUpdated`, or `upnlRiskFactorForWithdrawalsUpdated` events can now read `newBps`/`previousBps` directly; the wire adapter normalizes snake_case `new_bps`/`previous_bps` from the server automatically.

## v0.4.31 - 2026-06-11

Source Phoenix commit: `443ff8dfd64a9e7d35960b8b1946b3248d6681e5`

### Summary

- Added `V1TradesClient.getUserLiquidationHistory(userPubkey, request?)` — a new paginated endpoint (`GET /v1/users/{pubkey}/liquidation-history`) returning typed liquidation events for three kinds: `market_order`, `backstop`, and `adl`.
- All `RiskFactors` fields (`maintenance`, `backstop`, `highRisk`, `upnl`, `upnlForWithdrawals`, `cancelOrder`) now carry **basis-point values** (e.g. `5000` = 50%) throughout the projected-market and margin layers; previously they held percentage values.
- `ExchangeRiskFactors` and `ExchangeLeverageTier` gain optional `*Bps` companion fields populated by the exchange cache and WebSocket store; the legacy percentage fields remain present.
- WebSocket risk-factor update events (`cancelRiskFactorUpdated`, `upnlRiskFactorUpdated`, `upnlRiskFactorForWithdrawalsUpdated`) gain optional `previousBps`/`newBps` fields, normalized from server snake_case.

### Consumer Notes

- New liquidation history types (`UserLiquidationHistoryPoint` and subtypes `UserMarketLiquidationHistoryPoint`, `UserBackstopLiquidationHistoryPoint`, `UserAdlLiquidationHistoryPoint`) are exported from the public surface; narrow on the `kind` discriminant to access type-specific fields.
- `ExchangeRiskFactors` percentage fields remain for backward compatibility on the exchange config/snapshot layer; prefer the new `*Bps` variants when present.
- Limit of 100 items per `getUserLiquidationHistory` request; use `nextCursor`/`prevCursor` for pagination.

## v0.4.32 - 2026-06-15

Source Phoenix commit: `3506fd24b235813df18d1897c9ff076417c19ee8`

### Summary

- Added a new `PlaceMarketOrderDelegated` instruction that allows placing market orders signed by a delegated wallet distinct from the trader account authority. Supports both an explicit `traderWallet`/`permissionAccount` pair and a default fallback to the position authority.
- Exported the full `PlaceMarketOrderDelegated` surface: low-level builder (`buildPlaceMarketOrderDelegatedIx`), codec helpers (`getPlaceMarketOrderDelegatedEncoder/Decoder/Codec`), higher-level builders (`buildPlaceMarketOrderDelegated`, `buildPlaceMarketOrderDelegatedIxResolved`), a fire-and-send helper (`placeMarketOrderDelegated`), and the associated TypeScript types.
- `PlaceMarketOrderDelegated` instructions are now recognized as Flight-routable, matching the same Flight-wrapping behavior as `PlaceMarketOrder`.
- `instructions.json` now includes canonical discriminant hex entries for both `PlaceMarketOrderDelegated` and `FlightPlaceMarketOrderDelegated`.

### Breaking Changes

- **`PhoenixIxClient` interface extended**: `buildPlaceMarketOrderDelegated` and `placeMarketOrderDelegated` methods are added to the `PhoenixIxClient` interface. Any downstream code that manually implements this interface will fail to compile until the two new methods are added.

### Consumer Notes

- When `traderWallet` and `permissionAccount` are omitted from `ClientPlaceMarketOrderDelegatedInput`, the SDK defaults both to `positionAuthority` (falling back to `authority`), so the method works as a drop-in for the primary position authority signing flow.
- The `traderWallet` account is encoded as `READONLY_SIGNER` (account index 3) and `permissionAccount` as writable (account index 4) — relevant if you inspect raw account lists.
- `buildPlaceMarketOrderDelegated` and `placeMarketOrderDelegated` are available on both the root package export and the `PhoenixIxClient` / `PhoenixIxOperations` client objects.

## v0.4.33 - 2026-06-15

Source Phoenix commit: `d1c6f3dea8582f451d616bc42b1a083f9fa04000`

### Summary

- `V1TradersClient.getTraderState()` has been removed; the replacement `getTraderStateSnapshot()` is the only trader state method going forward.
- Two new collateral history methods added to `V1CollateralClient`: `getTraderPdaCollateralHistory()` for single-page lookups by trader PDA pubkey, and `getAllTraderPdaCollateralHistory()` for auto-paginated full history.
- All internal HTTP routes have been updated to versioned `/v1/` prefixes; existing method call signatures are unchanged for consumers using the SDK client normally.

### Breaking Changes

- **`V1TradersClient.getTraderState(authority, request?)` removed** along with the `TraderStateRequest` interface and `TraderStateResponse`/`TraderStateResponseSchema` exports. Migrate to `getTraderStateSnapshot(authority, { traderPdaIndex })`, which returns a `TraderStateSnapshotResponse` with `traderPdaIndex` and `snapshot.subaccounts`.
- **Exchange endpoints migrated** from `/exchange/*` to `/v1/view/exchange/*`. Consumers building mock HTTP interceptors, test transports, or calling the API directly (bypassing the SDK client) must update these paths: `getExchange`, `getMarket`, `getStatus`, `getKeys`, and `getMarkets`.
- **Market fills endpoint** changed from `/market/{symbol}/fills` to `/v1/trades/{symbol}/fills`; **trader PnL endpoint** changed from `/trader/{authority}/pnl` to `/v1/users/{authority}/pnl`. Same impact scope as above.

### Consumer Notes

- `getTraderCollateralHistory(authority, { pdaIndex? })` continues to work for authority-based lookups via `/v1/trader/{authority}/collateral-history` (path now carries the `v1` prefix internally, but the method signature is unchanged).
- New `getTraderPdaCollateralHistory(traderPubkey, request?)` targets `/v1/traders/{traderPubkey}/collateral-history` — use this when you have the trader PDA pubkey directly rather than the authority.
- New `getAllTraderPdaCollateralHistory(traderPubkey, pageSize?, request?)` auto-paginates until `hasMore` is false and returns the flat event array; the default page size is 1000.

## v0.4.34 - 2026-06-15

Source Phoenix commit: `c25ccb579ff367b65aedf254320d6a54da56613f`

### Summary

- Added `MaxLiquidationSizeUpdated` exchange market parameter update event: new `interface MaxLiquidationSizeUpdated` with `kind: "maxLiquidationSizeUpdated"`, `previousBaseLots: bigint`, and `newBaseLots: bigint`.
- `MaxLiquidationSizeUpdated` is now included in the `ExchangeMarketParameterUpdate` discriminated union and recognized by the WebSocket wire adapter's known-kinds set and Zod schema.
- The exchange cache store now maps `"maxLiquidationSizeUpdated"` events to the `"maxLiquidationSize"` change kind and applies `update.newBaseLots` to `nextMarket.maxLiquidationSizeBaseLots`.
- `"maxLiquidationSize"` added to the `ExchangeCacheMarketChangeKind` union type.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- Consumers subscribing to exchange market parameter updates will now receive `maxLiquidationSizeUpdated` events. If you have exhaustive switch/discriminated-union handling over `ExchangeMarketParameterUpdate`, TypeScript will require you to handle the new `MaxLiquidationSizeUpdated` variant.
- If you use `ExchangeCacheMarketChangeKind` exhaustively (e.g., in a switch or mapped type), add a case for `"maxLiquidationSize"` to avoid compilation errors.

## v0.4.35 - 2026-06-15

Source Phoenix commit: `0499d27df18bb2694c5489a9395bec0a58d1d141`

### Summary

- Version bumped from `0.4.34` to `0.4.35`.
- Add-market setup payloads now encode an after-hours radius field. The field is a `u64` appended at the end of the instruction data and defaults to `0` (no after-hours radius).

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- The encoded size of add-market instructions has changed to 345 bytes to accommodate the new after-hours radius field. If your code hard-codes or validates the byte length of these instruction payloads, update it accordingly.
- The after-hours radius defaults to `0n` (disabled), so existing integrations that do not set this field should continue to work without modification.

## v0.4.36 - 2026-06-15

Source Phoenix commit: `8809c4d39c7070f6430c20dd5fe670ad86032523`

### Summary

- Version bumped from `0.4.35` to `0.4.36`.
- No functional code, API, or type changes are included in this sync — the diff contains only the version field update in `package.json`.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- None identified in the synced diff.

## v0.4.37 - 2026-06-15

Source Phoenix commit: `5a524ada417b9cce3f94096900c59b4ec5fc17ec`

### Summary

- Bumped `@ellipsis-labs/rise` from `0.4.36` to `0.4.37`.
- Upgraded Vite from `7.3.2` to `8.0.16` (major version bump) in the package's build tooling and pinned override.
- Vite 8 replaces `rollup` and `esbuild` as core bundler with `rolldown` (Rust-based) and adds `lightningcss` as a required CSS processor.

### Breaking Changes

- **Vite 8 bundler switch**: Vite 8 drops `rollup` and `esbuild` as core bundler dependencies in favor of `rolldown`. If your project extends the `@ellipsis-labs/rise` build config or shares a Vite instance, any use of `build.rollupOptions` or esbuild-specific plugin APIs may require updates.
- **`esbuild` is now an optional peer in Vite 8**: Consumers who relied on esbuild being available transitively through Vite will need to add it explicitly if their own config references esbuild transforms or plugins.

### Consumer Notes

- No public TypeScript API changes in this release — types, exports, and runtime behavior are unchanged.
- If your project uses Vite and this package's `vite` override propagates into your lock file, you will be upgraded to Vite 8. Review the [Vite 8 migration guide](https://vite.dev/guide/migration) for `rollupOptions` and plugin compatibility.
- `lightningcss` is now a first-class dependency of Vite 8 (no longer an optional peer); CSS processing behavior may differ subtly from esbuild's CSS pipeline.

## v0.4.38 - 2026-06-15

Source Phoenix commit: `978cab228c047f8e511a8075621846c679d331d5`

### Summary

- Bumped `ws` peer dependency minimum from `>=8.20.1` to `>=8.21.0`.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- If you pin `ws` to an exact version below `8.21.0`, you may see peer dependency warnings or conflicts. Update your lockfile or pin to `ws@8.21.0` or later.

## v0.4.39 - 2026-06-15

Source Phoenix commit: `6b96d79ca6d38a289150f382efe441c4116e395c`

### Summary

- **Version bump**: `@ellipsis-labs/rise` is now `0.4.39`.
- **Source files included in package**: The `src/` directory is now bundled in the published npm package alongside `dist/`. Consumers benefit from source availability for debugging and source maps without needing a separate step.
- **Changelog included in package**: `CHANGELOG.md` is now shipped with the npm package, making release notes available locally after install.
- **README documents changelog location**: A new "Changelog" section in the README links to the public Rise TypeScript changelog on GitHub.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- No API, type, or runtime behavior changes in this release — the changes are packaging-only.
- If your bundler or tooling scans `node_modules` for `.ts` source files, the newly included `src/` directory may be picked up; verify your `exclude` or `ignore` patterns if unexpected files appear in your build.

## v0.4.40 - 2026-06-16

Source Phoenix commit: `65fc5aad3e13c249bcc606b410059b8adbc61e2d`

### Summary

- **Fixed liquidation price calculation**: The formula for computing a position's liquidation price now correctly incorporates the market's `maintenanceMarginFactorBps` risk parameter. Previously the formula used a simplified approximation; it now derives the denominator and numerator using the actual maintenance margin coefficient, producing more accurate results across leverage tiers.
- **Fixed unsettled funding sign convention in snapshot helpers**: `buildMarginPositionStateFromSnapshot` no longer negates `unsettledFundingQuoteLots` when converting from a snapshot. The field is now passed through as-is, matching the margin-engine sign convention (positive = increases effective collateral).
- **Clarified `MarginPositionState.unsettledFundingQuoteLots` documentation**: The JSDoc on this field no longer instructs callers to flip the sign themselves; the snapshot helper now handles the value correctly without manual adjustment.

### Breaking Changes

- **Snapshot-sourced `unsettledFundingQuoteLots` values will differ**: Any code that previously compensated for the sign flip in `buildMarginPositionStateFromSnapshot` (e.g., by negating the field before passing it downstream) must remove that workaround. The value is now passed through with the margin-engine convention intact.
- **Liquidation price outputs will change**: Computed liquidation prices from `computeLiquidationPriceTicks` will differ from prior versions due to the corrected formula. Consumers who cache, display, or act on liquidation price values should expect updated results after upgrading.

### Consumer Notes

- If you construct `MarginPositionState` manually (not via snapshot helpers) and were previously accounting for the sign flip described in the old JSDoc, no change is needed — the type contract is unchanged; only the snapshot helper behavior is fixed.
- The liquidation price fix requires `maintenanceMarginFactorBps` to be present and positive in `MarketParams.riskFactors`; positions with a zero or missing value will return `undefined` for liquidation price (was previously a potentially incorrect value).

## v0.4.41 - 2026-06-16

Source Phoenix commit: `6487be852f3a382717f345a43fd956af3fba1766`

### Summary

- `PhoenixHttpClient` now automatically retries `GET` and `HEAD` requests that receive a `429 Too Many Requests` response. By default, up to 2 retries are attempted with a cumulative wait cap of 15 seconds, honoring the server's `Retry-After` header (both numeric seconds and HTTP-date formats). Pass `rateLimitRetry: false` to opt out.
- A new `rateLimitRetry` config field on `PhoenixHttpClientConfig` accepts a `RateLimitRetryConfig` object (`maxRetries`, `maxTotalWaitMs`, `fallbackDelayMs`) or `false` to disable retries entirely.
- `PhoenixHttpError` gains an `attempts` field (number of total attempts including the initial request) so callers can distinguish a first-shot 429 from one that exhausted retries.
- `RateLimitRetryConfig` is now exported from the package root (`@ellipsis-labs/rise`).
- `Retry-After` parsing is now unified and more accurate: supports both numeric seconds (including fractional) and HTTP-date strings; previously, `transport.ts` parsed only integers and clamped to a minimum of 1 second.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- **Default retry behavior is now active.** Existing code that calls `GET`/`HEAD` endpoints may now wait and retry on 429s (up to ~15 s total) instead of failing immediately. If your application handles 429s manually or has strict latency budgets, pass `rateLimitRetry: false` to restore the prior behavior.
- **`PhoenixHttpError.attempts` is additive** — no code changes required, but error-inspection code that checks for a fixed set of fields may now observe this new property on 429 errors.

## v0.4.42 - 2026-06-16

Source Phoenix commit: `ee8192d915d1d282849dc7e70d84db2e51cbba42`

### Summary

- Added optional `permissionAddress` field to `ClientTransferCollateralInput`, `ResolvedTransferCollateralIxInput`, and `TransferCollateralParams` to support secondary position authority delegation in `TransferCollateral` instructions. When provided, the address is appended as a writable account on the instruction.
- `PlaceMarketOrderDelegated` instructions are no longer wrapped by the Flight router (`wrapInstructionWithFlight` / `isFlightRoutableInstruction`); they are passed through unchanged.
- `FlightPlaceMarketOrderDelegated` has been removed from the exported `instructions.json` fixture.

### Breaking Changes

- **`FlightPlaceMarketOrderDelegated` removed from `instructions.json`**: Consumers that read this key from the package's `instructions.json` export will find it absent. Remove any reference to `instructions["FlightPlaceMarketOrderDelegated"]` in downstream code.
- **Delegated market orders no longer Flight-routed**: `isFlightRoutableInstruction` and `wrapInstructionWithFlight` no longer recognize `PLACE_MARKET_ORDER_DELEGATED` instructions. If your code relied on the Flight router wrapping delegated market orders, those instructions will now be returned unchanged. Update any logic that expected a Flight-wrapped delegated market order to handle the pass-through behavior.

### Consumer Notes

- The new `permissionAddress` field on `ClientTransferCollateralInput` and `ResolvedTransferCollateralIxInput` is optional and backward compatible; no changes required if you are not using position authority delegation for collateral transfers.
- If you pass a `permissionAddress`, it is included as the last account (writable) on the resulting `TransferCollateral` instruction — account index assumptions in any manual account-position logic should account for this trailing account.

## v0.4.43 - 2026-06-17

Source Phoenix commit: `e01d6bf79f112b8f9c7c6e7e3ad84fb83050eb43`

### Summary

- **Hawkeye program client**: Added a full read-only simulation client for the Hawkeye program (`RiSeVw3ZjNfsaXPRb4mgaqYaEEt41pNNJoDvVh7pgQj`). Call `client.rpc.hawkeye.viewMargin(...)`, `.viewMarginForAsset(...)`, `.viewLiquidationPrice(...)`, `.viewBbo(...)`, and `.viewFunding(...)` to query on-chain margin, risk state, liquidation price, BBO, and funding data via transaction simulation — no signature required.
- **New root-level exports**: Five instruction builders (`buildHawkeyeViewMarginIx`, `buildHawkeyeViewMarginForAssetIx`, `buildHawkeyeViewLiquidationPriceIx`, `buildHawkeyeViewBboIx`, `buildHawkeyeViewFundingIx`), `decodeHawkeyeReturnData`, `encodeHawkeyeSimulationTransaction`, `HAWKEYE_PROGRAM_ADDRESS`, `HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT`, and all associated TypeScript types (`HawkeyeReturnData`, `HawkeyeMarginReturn`, `HawkeyeAssetReturn`, `HawkeyeLiquidationPriceReturn`, `HawkeyeBboReturn`, `HawkeyeFundingReturn`, etc.) are now exported from the package root.
- **`PhoenixIxClient` additions**: Five new `buildHawkeyeView*` methods on the ix client automatically resolve trader accounts and asset IDs from authority/symbol, mirroring the existing client pattern.
- **`zod` pinned to `4.4.3`** (previously `^4.3.6`).

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `PhoenixRpcAccountFetcherClient.request` was widened from `private` to `public`; this is additive and does not break existing code, but the method is now formally part of the class surface.
- The `zod` dependency is now pinned to the exact version `4.4.3`. If your project declares `zod` as a direct dependency and previously resolved `^4.3.6` to a different patch, verify your lockfile resolves cleanly after upgrading to this version of `@ellipsis-labs/rise`.

## v0.4.44 - 2026-06-17

Source Phoenix commit: `2d72d1003db4c5e852d3eef26572cd24da068745`

### Summary

- Added opt-in fee-override support to Flight proxy instructions. When `feeBpsOverride` is set on `PhoenixFlightClientConfig` or `ProxyInstructionParams`, `buildProxyInstructionIx` and `wrapInstructionWithFlight` emit `proxy_instruction_with_fee_override` instead of the standard `proxy_instruction`, allowing an integration route to override the builder's registered fee.
- New `FLIGHT_DISCRIMINANTS.PROXY_INSTRUCTION_WITH_FEE_OVERRIDE` discriminant and associated codec helpers (`encodeProxyInstructionWithFeeOverrideData`, `getProxyInstructionWithFeeOverrideParamsEncoder/Decoder/Codec`, `getProxyInstructionWithFeeOverridePrefixEncoder`, `ProxyInstructionWithFeeOverrideParamsData`) are now exported from the package.
- `instructions.json` gains the deterministic `ProxyInstructionWithFeeOverride` entry for snapshot testing and tooling.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `feeBpsOverride` (`bigint | null`) is an optional, additive field on both `PhoenixFlightClientConfig` and `ProxyInstructionParams`; omitting it (or passing `null`) preserves existing behavior with no code changes required.
- When `feeBpsOverride` is provided, `buildProxyInstructionIx` validates that the value is in the range `0n–10_000n` (inclusive) and throws `"Fee bps override must be in the range 0..=10000"` if not.
- The fee-override path emits an on-chain instruction with a different discriminant (`PROXY_INSTRUCTION_WITH_FEE_OVERRIDE`), so infrastructure that inspects raw instruction bytes or discriminants (parsers, indexers, monitoring) should be updated to recognize the new variant.

## v0.4.45 - 2026-06-18

Source Phoenix commit: `5b7f20375f7eee51e309f9c4bc994609f5be2d1e`

### Summary

- **Auth-gated channel subscriptions are now deferred on external-session clients.** Subscriptions to the `events`, `notifications`, `trader`, `traderVolume`, and `transaction` channels will no longer be sent over an anonymous WebSocket connection. If no external session has been imported yet, the subscription message is held and replayed automatically on the authenticated connection once a session arrives.
- **Public subscriptions flow immediately over anonymous connections.** Channels not in the auth-required set (e.g. `allMids`, orderbook, trades) are sent to the server right away, even while the client is waiting for an external session to be imported — no unnecessary reconnect is triggered for these channels.
- **Reduced reconnect churn in `external` session control mode.** When a session has not yet been provided, the client now keeps the existing anonymous socket open instead of immediately scheduling a reconnect every time a subscription is registered, resulting in fewer connection disruptions during the authentication waiting period.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- If your application subscribes to auth-required channels (`events`, `notifications`, `trader`, `traderVolume`, `transaction`) before calling `sessionManager.importSnapshot(...)`, those subscribe messages will be buffered and sent once the authenticated connection is established. No code changes are required, but you should be aware that these subscriptions are not active until authentication completes.
- Public channel subscriptions registered before an external session is available will now be active on the anonymous socket immediately, so data for those channels will begin arriving sooner than before.

## v0.4.46 - 2026-06-22

Source Phoenix commit: `fd9d044ad0e76e6bcbef1333b1ebc8648f511b7a`

### Summary

- Added `MarketStatsSnapshot` interface and `MarketStatsSnapshotSchema` as new public exports from `@ellipsis-labs/rise`.
- `ExchangeMarketConfig` now includes an optional `statsSnapshot?: MarketStatsSnapshot` field containing slot, open interest, funding interval timestamp, and cumulative funding rate.
- Numeric fields (`openInterestBaseLots`, `fundingStartIntervalTimestamp`, `cumulativeFundingRate`) are coerced to strings at parse time, so the API may return either strings or numbers and the SDK normalizes them.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- Access per-market stats via `market.statsSnapshot` when calling `getMarkets()`. The field is optional and will be `undefined` if the server does not include it.
- `MarketStatsSnapshot` and `MarketStatsSnapshotSchema` are now available as named exports for consumers who want to validate or type this shape independently.
