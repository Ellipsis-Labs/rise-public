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
