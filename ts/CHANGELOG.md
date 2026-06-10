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
- **`PERMISSION_ACCOUNT` discriminant corrected**: The on-chain discriminant for permission accounts changed from `"account:permission"` to `"account:permission_account"`. `decodePermission` will now correctly parse real on-chain permission accounts.
- **New example**: `examples/07-onboard-trader-delegated.ts` demonstrates the full delegated onboarding flow, including permission validation and conditional trader registration.

### Breaking Changes

- **`PERMISSION_ACCOUNT` discriminant changed**: If your code calls `decodePermission` or checks `ACCOUNT_DISCRIMINANTS.PERMISSION_ACCOUNT` directly, the underlying hash value has changed. Accounts parsed with the old discriminant (`sha2("account:permission")`) will no longer decode correctly — re-fetch and decode permission accounts after upgrading.

### Consumer Notes

- The `TraderCapabilityToggleTarget` enum and internal `buildSetTraderCapabilitiesDelegated*` names are intentionally absent from the public surface; use `buildOnboardTraderDelegated` instead.
- Delegated onboarding always enables all six trader capabilities (limit orders, market orders, risk-increasing and risk-reducing trades, deposit, and withdrawal) in a single instruction — there is no partial-capability variant in this release.
- The `permissionAccount` address must be derived from the current risk authority and the onboarder's key; see the new example for the full derivation pattern using `client.pda.getPermissionAddress`.
