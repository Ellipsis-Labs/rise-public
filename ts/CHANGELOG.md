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

- **Registry moved to GitHub Packages**: `publishConfig.registry` is now `https://npm.pkg.github.com` with `access: restricted`. Add `@ellipsis-labs:registry=https://npm.pkg.github.com` to your `.npmrc` and authenticate with a GitHub token.
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
