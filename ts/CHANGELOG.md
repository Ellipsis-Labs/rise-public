# Changelog

Entries are drafted by Phoenix Rise sync PRs. Review and edit each
entry in this repo before merging.

## v0.4.27 - 2026-06-09

Source Phoenix commit: `32611fa7f803cf601cab67b6ca5dce5d4a70fc75`

- Package: `@ellipsis-labs/rise`
- Target repo version: 0.4.19
- Phoenix version: unknown -> 0.4.27

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

- Package: `@ellipsis-labs/rise`
- Target repo version: 0.4.27
- Phoenix version: 0.4.27 -> 0.4.28

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

- Package: `@ellipsis-labs/rise`
- Target repo version: 0.4.28
- Phoenix version: 0.4.28 -> 0.4.30

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

## v0.4.33 - 2026-06-11

Source Phoenix commit: `d1c6f3dea8582f451d616bc42b1a083f9fa04000`

- Package: `@ellipsis-labs/rise`
- Target repo version: 0.4.30
- Phoenix version: 0.4.32 -> 0.4.33

### Summary

- **New `PlaceMarketOrderDelegated` instruction** — adds `buildPlaceMarketOrderDelegatedIx`, `buildPlaceMarketOrderDelegated`, and `placeMarketOrderDelegated` exports (plus `buildPlaceMarketOrderDelegatedIxResolved` and associated types). The instruction is Flight-routable. When `traderWallet` is omitted it falls back to `positionAuthority` then `authority`.
- **New `getUserLiquidationHistory`** on `V1TradesClient` — fetches per-user liquidation events (market-order, backstop, ADL) from `/v1/users/{pubkey}/liquidation-history`. Typed discriminated union `UserLiquidationHistoryPoint` and supporting schemas are now public exports.
- **New trader-PDA collateral history methods** — `getTraderPdaCollateralHistory` and `getAllTraderPdaCollateralHistory` on `V1CollateralClient` fetch via `/v1/traders/{pubkey}/collateral-history`, keyed on the trader's public key rather than an authority + PDA index.
- **HTTP endpoints migrated to `/v1/` prefix** — exchange, market fills, trades history, PnL, funding, order history, and collateral history now call versioned paths. The SDK handles this transparently; see Breaking Changes if you use a custom transport.

### Breaking Changes

- **`V1TradersClient.getTraderState` removed** — the method and its `TraderStateRequest`/`TraderStateResponse` types have been deleted. Migrate to `getTraderStateSnapshot` (`/v1/trader/state/{authority}`), which returns a structured snapshot response including `traderPdaIndex` and `snapshot.subaccounts`.
- **`V1ExchangeClient` and `V1MarketsClient` now call `/v1/view/exchange*` paths** — `getExchange`, `getMarket`, `getMarkets`, `getStatus`, and `getKeys` no longer hit the bare `/exchange/*` routes. Custom transports or mock servers that intercept exact endpoint strings must be updated.
- **`V1TradesClient.getMarketFills` now calls `/v1/trades/{symbol}/fills`** (was `/market/{symbol}/fills`). Same requirement for custom transports.
- **Several `V1TradersClient` and `V1TradesClient` history methods moved to `/v1/` prefix** — affects `getTraderPnl` (`/v1/users/{authority}/pnl`), `getTraderTradesHistory` (`/v1/trader/{authority}/trades-history`), `getTraderFundingHistory` (`/v1/trader/{authority}/funding-history`), and `getTraderOrderHistory` (`/v1/trader/{authority}/order-history`).

### Consumer Notes

- `buildPlaceMarketOrderDelegated` / `placeMarketOrderDelegated` accept an optional `traderWallet` and `permissionAccount`; when both are omitted the resolved `positionAuthority` (or `authority`) is used for both, matching the primary-position-authority signing pattern.
- `getTraderPdaCollateralHistory` takes the trader's **public key** directly (no `pdaIndex`); the existing `getTraderCollateralHistory(authority, { pdaIndex })` remains available for legacy authority-based lookups at its new `/v1/trader/…` path.
- All `UserLiquidationHistoryPoint` variants normalise `slot`, `slotIndex`, `eventIndex`, `timestamp`, and `subaccountIndex` to `number` regardless of whether the API returns them as strings or numbers.
