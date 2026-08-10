# Changelog

Entries are drafted by Phoenix Rise sync PRs. Review and edit each
entry in this repo before merging.

## v0.5.6 - 2026-08-10

Source Phoenix commit: `cac9d3de728d31bf20f2a9499f03866c305e2958`

### Summary

- Adds native SOL spot collateral support end-to-end: new account fields (`GlobalConfiguration.nativeSolSpotMetadata`, `Trader.nativeSolCollateral`, `Trader.disablePositionAuthoritySwap`), a full `NativeSol` instruction builder set (`buildSyncNativeIx`, `buildWithdrawNativeSolIx`, `buildTransferNativeSolIx`, `buildSwapNativeIx`, `buildLiquidateNativeSolIx`, `buildTransferNativeSolFromChildToParentIx`), margin/liquidation valuation helpers (`spotCollateralPrice`, `notionalSpotCollateral`, `discountedSpotCollateral`, `maxWithdrawableSpotCollateral`), and a `/v1/collateral/assets` client method plus `spotCollaterals` on exchange and trader-state snapshots/deltas.
- Adds draft-order margin sizing helpers (`computeDraftOrderMarginRequirementFrom{Inputs,Snapshot}`, `computeMaxDraftOrderSizeForAvailableMarginFrom{Inputs,Snapshot}`) and a new `getTraderTimeWeightedReturns` trader API method.
- Adds a sponsored/atomic Flame deposit flow (`buildFlameAtomicDepositFlow`, `buildFlameDepositToPhoenixIx`) for wallets holding no SOL, plus new address helpers (`EMBER_STATE_ADDRESS`, `BETA_USDC_MINT_ADDRESS`, `resolvePhoenixBuilderAddresses`, `getPhoenixNativeSolAuthorityAddress`).
- Reworks Flight builder-fee routing for delegate-signed orders: the collateral-transfer tail is now controlled by an explicit flag instead of being inferred, and owner-signed `PlaceMarketOrderDelegated` is now recognized as Flight-routable without the tail.

### Breaking Changes

- `wrapInstructionWithFlight` renamed its `authority` param to `signer` and now requires an explicit `usePositionAuthority` flag to opt into the collateral-transfer tail (previously inferred); the flight client's `tryWrapFlightInstruction` method was renamed to `tryWrapOrderInstruction`. Direct callers of Flight order wrapping must update to the new parameter names/shape.
- `PhoenixIxOperationContext.maybeWrapOrderIx` gained a required `usePositionAuthority` parameter, with a new separate `maybeWrapConditionalOrderIx` for conditional placements; custom implementations of this interface need updating.
- `ProxyInstructionParams` no longer infers the collateral-transfer tail from the wrapped instruction — pass the new `rootAuthority` field explicitly when signing as a position authority, or the tail (and its builder-fee collection) is silently omitted.

### Consumer Notes

- `createMarginCalculator` now accepts an optional second `spotCollaterals: SpotCollateralParams[]` argument; existing single-argument calls are unaffected.
- `ClientPlaceMarketOrderDelegatedInput`'s dedicated signer field is deprecated in favor of the shared `positionAuthority` field used across all placement inputs.
- New instruction discriminants were added for `AuthorizedTransferCollateral`, `SyncNative`, `WithdrawNativeSol`, `TransferNativeSol`, `TransferNativeSolFromChildToParent`, `LiquidateNativeSol`, `SwapNative`, and `DelegateTrader`.
- `PlaceIsolatedMarketOrderRequest` gained optional `minBaseLotsToFill`/`minQuoteLotsToFill` fields for partial-fill IOC/market orders.

## v0.4.67 - 2026-07-09

Source Phoenix commit: `39520ccb1d19d0f7610909dd2718dc7918d0ec22`

### Summary

- Added `getLatestMarketStats(symbol)` to `V1MarketsClient` for fetching the latest stats snapshot for a single market via `GET /v1/market/{symbol}/stats/latest`.
- Added `getLatestMarketsStats()` to `V1MarketsClient` for fetching the latest stats snapshot across all markets via `GET /v1/markets/stats/latest`.
- Exported new response types and Zod schemas: `LatestMarketStatsResponse`, `LatestMarketStatsResponseSchema`, `LatestMarketsStatsResponse`, `LatestMarketsStatsResponseSchema`.
- `LatestMarketStatsResponse` includes `mark_price`, `oracle_price`, `prev_day_mark_price`, `open_interest`, `day_volume_usd`, `day_volume_base`, `current_funding_rate`, `eight_hour_funding_rate`, and `annualized_funding_rate`.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `timestamp_ms` on the new response types is parsed as a `bigint` using the same big-integer schema used elsewhere in the SDK. It accepts `string`, `number`, or `bigint` input, but rejects non-integer numbers and numeric values beyond `Number.MAX_SAFE_INTEGER` — pass large timestamps as strings to avoid precision loss.
- `getLatestMarketsStats()` returns `{ markets: LatestMarketStatsResponse[] }`.

## v0.4.66 - 2026-07-08

Source Phoenix commit: `6051225fb045fbb5b6a454bd445e7fc2e31e5722`

### Summary

- Rewrote the SDK's liquidation-price search to use a numeric mark-price boundary search instead of the previous closed-form equation, fixing incorrect results for already-liquidatable positions and for same-market resting orders.
- Added a new "projected liquidation" API — `computeProjectedLiquidation`, `computeMarketProjectedLiquidationFromMargin`, `computeSubaccountProjectedLiquidationFromMargin`, `createProjectedLiquidationCalculator`, `aggregateLimitOrderStateFromOrders`, and related types — that estimates liquidation risk after a trader's own resting orders fill along the adverse price path.
- Limit-order margin best-bid/best-ask sentinels are now computed relative to the current mark price via the new `buildLimitOrderMarginStateFromOrdersAtMark`, so only orders that could actually cross at the current mark reserve fill-loss margin.

### Breaking Changes

- `computeMarketLiquidationPriceFromMargin`, `computeSubaccountLiquidationPricesFromMargin`, and `computeTraderLiquidationPricesFromMargin` now return the current mark price for already-liquidatable positions and otherwise use the new boundary-search algorithm — expect different numeric outputs than 0.4.65 for identical inputs.
- `buildMarketMarginInputsFromSnapshot` no longer populates `limitOrderMargin` on its returned `MarketMarginInputs`; margin/liquidation computation now derives limit-order sentinels internally from `limitOrders` and the market's mark price. Code reading `.limitOrderMargin` directly off this helper's output will now see `undefined`.
- Same-market limit-order maintenance margin is now mark-price-dependent (only crossing-side resting orders count toward fill-loss margin), changing computed margin/liquidation values for accounts with resting orders on the far side of the mark.

### Consumer Notes

- Treat the existing static liquidation-price functions as the canonical current-state ("Hawkeye-compatible") value; use the new projected-liquidation functions only for scenario/risk displays, not for liquidation-eligibility or program parity checks.
- `calculateLiquidationPriceUsd` gained an optional `targetLimitOrderMaintenanceCoefficient` input, defaulting to `0` for backward compatibility.
- `buildLimitOrderMarginStateFromOrders` keeps its prior signature/behavior for existing callers; switch to `buildLimitOrderMarginStateFromOrdersAtMark` where mark-relative sentinels are needed.

## v0.4.65 - 2026-07-07

Source Phoenix commit: `25625a376965069d216ba53f8bbf0457f097a927`

### Summary

- Margin calculations now reserve additional collateral for resting limit orders priced adversely to the mark price (an "adverse fill-loss" reserve), mirroring the on-chain `program-core` margin logic.
- Reduce-only order margin reserves are now capped by the size of the opposite-side position they can actually reduce, preventing over-reservation for large reduce-only orders against small positions.
- `LimitOrderMarginState` and the internal margin aggregation helpers gained new fields (best bid/ask, reduce-only totals) to support the fill-loss calculation.

### Breaking Changes

- `LimitOrderMarginState` now requires `numAskOrders`, `numBidOrders` (previously optional), plus new required fields `lowestAsk`, `highestBid`, `totalReduceOnlyAskBaseLots`, and `totalReduceOnlyBidBaseLots`. Code that constructs this type directly (e.g. passing `limitOrderMargin` as a computed-margin input) must supply all of these fields or compilation/runtime behavior will differ.
- `buildLimitOrderMarginStateFromOrders` gained a second parameter, `basePositionLots`, used to cap reduce-only reserves against the opposite position. It defaults to `0n`, so existing calls still compile, but omitting it means reduce-only orders are treated as fully reducing a zero position (capped to zero) rather than uncapped as before.

### Consumer Notes

- Accounts with resting limit orders priced away from mark should expect `initialMarginQuoteLots` and `limitOrderMarginQuoteLots` from `computeSubaccountMarginFromInputs` (via `createMarginCalculator`) to increase, since adverse-fill-loss is now included in the reserve.
- If you rely on `buildLimitOrderMarginStateFromOrders` for reduce-only sizing, pass the account's current `basePositionLots` (signed, base lots) as the second argument to get correctly capped reserves instead of the new zero-position default.
- Callers that only supply `limitOrders` (not a precomputed `limitOrderMargin`) need no changes — the new fields are derived automatically from order data and position size.

## v0.4.64 - 2026-07-06

Source Phoenix commit: `84c3feb1a2f5d9b4e94f9372a706b0e3e3c88b0e`

### Summary

- Added a liquidation price calculator (`calculateLiquidationPriceUsd`) plus helpers to compute per-market, per-subaccount, and per-trader liquidation prices from margin results or raw inputs (`compute*LiquidationPricesFrom{Inputs,Margin}`, `computeMarketLiquidationPriceFromMargin`).
- Added a margin simulation API (`simulateMarginFromInputs`, `simulateMarginScenariosFromInputs`, `simulatePositionFillFromInputs`) with actions for fills, position closes, limit orders, collateral/funding adjustments, and mark-price moves, including projected liquidation prices.
- `MarginCalculator` (from `createMarginCalculator`) gained corresponding instance methods: `computeTraderLiquidationPricesFromInputs`, `computeSubaccountLiquidationPricesFromInputs`, `simulateMargin`, `simulateMarginScenarios`, `simulatePositionFill`.
- Fixed leverage-tier interpolation (`getLeverageConstant`, `getLimitOrderRiskFactor`) to use floating-point interpolation matching the Rust program instead of bigint truncation.
- Internal `marginParityExport.ts` dev script was refactored to reuse the new liquidation module; no public API impact.

### Breaking Changes

- Interpolated leverage and limit-order risk-factor values for position sizes strictly between leverage-tier bounds can now differ slightly from prior output, since `getLeverageConstant`/`getLimitOrderRiskFactor` switched from bigint-truncated interpolation to Rust-matching floating-point interpolation. Consumers asserting exact previous intermediate values should re-verify them.

### Consumer Notes

- New exports: `calculateLiquidationPriceUsd`, `computeMarketLiquidationPriceFromMargin`, `computeSubaccountLiquidationPricesFromInputs`/`FromMargin`, `computeTraderLiquidationPricesFromInputs`/`FromMargin`, `simulateMarginFromInputs`, `simulateMarginScenariosFromInputs`, `simulatePositionFillFromInputs`, and their associated types.
- The liquidation-price helpers throw on inconsistent margin totals (maintenance-margin underflow between portfolio and position-only figures) rather than silently clamping — validate margin snapshots before calling these in production paths.
- Margin simulation actions (`fillPosition`, `closePosition`, `placeLimitOrder`, `cancelOrder`, `cancelAllOrders`, `adjustCollateral`, `setCollateral`, `settleFunding`, `applyFundingPayment`, `setMarkPrice`, `moveMarkPrice`) support building what-if previews (e.g. order tickets, price ladders) against cross or isolated margin scopes.

## v0.4.63 - 2026-07-06

Source Phoenix commit: `014384041aa15c0f02a8a5277745b907ae510ee4`

### Summary

- Added an optional `options` parameter (`MarginCalculationOptions`) to `computeTraderMargin`, `computeSubaccountMargin`, `computeTraderMarginFromInputs`, `computeSubaccountMarginFromInputs`, and the corresponding `MarginCalculator` methods, letting callers pass per-symbol order leverage preferences via `orderLeverageLimitsBySymbol`.
- Order leverage limits are applied only to order/limit-order sizing math (initial margin from open orders, limit order margin requirements); position-only margin, withdrawal margin, maintenance/backstop/high-risk margin, and risk state/tier are always computed from protocol leverage tiers, unaffected by the new option.
- `MarginTotals` gains an optional `orderLeverageAdjustedInitialMarginQuoteLots` field, and `OrderMarginResult` gains an optional `orderLeverageAdjustedMarginRequirementQuoteLots` field; both are omitted entirely when no limit is supplied or when the adjusted value matches the protocol value.
- Limit values are validated defensively: non-finite, non-integer-safe, sub-1, or above-protocol-max values are silently ignored and fall back to protocol leverage; valid fractional values floor to the nearest integer leverage.
- Calling existing margin functions with no `options` argument (or an empty options object) produces byte-identical output to the prior release.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- All new behavior is opt-in via the new `options` parameter — no changes are required to existing integrations.
- Consumers that want to preview reduced-leverage order/margin requirements (e.g. surfacing a lower "order leverage" preference in UI) can pass `{ orderLeverageLimitsBySymbol: { "SOL-PERP": 5 } }` and read the new `orderLeverageAdjusted*` fields where present; treat their absence as "no adjustment applies."
- Do not rely on `orderLeverageAdjustedInitialMarginQuoteLots` for withdrawal eligibility or risk-state checks — those continue to use protocol-only margin figures.

## v0.4.62 - 2026-07-02

Source Phoenix commit: `e427d47ea42afce753295b0559ac3a0e8c505518`

### Summary

- `buildCancelOrdersById` / `cancelOrdersById` now accept an optional `priceInTicks` field on each order, letting you cancel orders using the tick price returned by `placeLimitOrder`/order-state APIs instead of converting through a USD price.
- Market tick-size and base-lot-decimals metadata is now only required when cancelling orders via the legacy `price` (USD) field; tick-native cancels work even when that market metadata isn't resolved.
- Added `getCancelOrdersByIdDecoder` and `getCancelOrdersByIdCodec` exports alongside the existing `getCancelOrdersByIdEncoder`, enabling round-trip decoding of `CancelOrdersById` instruction data.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- The `price` field on cancel-by-id orders is now deprecated in favor of `priceInTicks`; existing code using `price` continues to work unchanged, but new integrations should prefer `priceInTicks` to avoid tick/USD conversion entirely.
- If you build cancel-by-id instructions purely from `priceInTicks`, you no longer need to ensure `tickSize`/`baseLotsDecimals` are present in your resolved market context.

## v0.4.61 - 2026-07-02

Source Phoenix commit: `64e433145ad2776c55d9ca53762ecfa78ba3ed51`

### Summary

- Added an optional `feePayer` field to `PlaceMultiLimitOrderFlowParams`. For isolated-margin multi-limit orders that require registering a fresh child subaccount, this account now pays the trader-account rent and signs the register instruction, instead of always defaulting to `authority`.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- `feePayer` is optional and defaults to previous behavior (`authority`) when omitted, so existing calls to `buildPlaceMultiLimitOrderFlow` are unaffected.
- If you place isolated-margin multi-limit orders through sponsored and/or delegated sessions — where neither the sponsor nor the delegate can sign for `authority` — pass a `feePayer` to cover trader-account rent and the register signature; otherwise flows that need to register a new child subaccount in that setup will fail.

## v0.4.60 - 2026-07-01

Source Phoenix commit: `cf8419bc21f9f539306198f7c08d0aca14a39580`

### Summary

- Adds a trader preferences bitfield: new `TraderPreferenceKind` enum, `encodeTraderPreferences`/`decodeTraderPreferenceFlags` helpers, and related constants/types for setting and reading per-trader flags (currently just `disableCollateralSweep`).
- `decodeTrader` output now includes `traderPreferenceFlags`, `preferences`, and `disableCollateralSweep` alongside the existing raw `traderPreferenceBits`.
- `buildRegisterTraderIx`/`buildRegisterTraderIxResolved` accept new optional `traderPreferenceBits`, `traderPreferences`, and `disableCollateralSweep` params to set preferences at registration time.
- Fixes trader position-map decoding so entries carrying a non-zero upper-bits discriminant (previously silently dropped) are now correctly included as position entries.
- Tightens `RegisterTrader` param validation: `traderPdaIndex` and `traderSubaccountIndex` must now be integers within explicit bounds (0–255 and 0–100 respectively).

### Breaking Changes

- `RegisterTrader` instruction wire format changed: `maxPositions` is now encoded as a `u32` (was `u64`), and a new `u32` `traderPreferenceBits` field is inserted before `traderPdaIndex`/`subaccountIndex`. Any code building raw instruction bytes manually, or relying on a cached/stale IDL for this instruction, must update to the new layout.
- `buildRegisterTraderIx` now throws for a `traderSubaccountIndex` outside `0–100` (previously unvalidated) and for non-integer `traderPdaIndex`/`traderSubaccountIndex` values; calls that previously succeeded with such inputs will now throw.
- The `traderPdaIndex` validation error message changed from `"Trader PDA index must be between 0 and 255"` to `"Trader PDA index must be an integer between 0 and 255"` — code matching on the exact string will break.

### Consumer Notes

- To disable collateral sweep for a trader, pass `disableCollateralSweep: true` (or `traderPreferences: [TraderPreferenceKind.DisableCollateralSweep]`) to `buildRegisterTraderIx`/`buildRegisterTraderIxResolved`.
- Existing decoded `Trader` accounts gain `preferences.disableCollateralSweep` and `traderPreferenceFlags` (with `bits`, `enabled`, `reservedBits`, `hasReservedBits`) for inspecting preference state without manual bit-masking.
- If your app reads `positions.entries` from decoded `Trader` accounts, expect entry counts/contents to change for accounts holding position-map entries with a non-zero discriminant in the upper 32 bits — these are now surfaced as position entries where they were previously silently skipped.

## v0.4.59 - 2026-06-30

Source Phoenix commit: `e6a5f77bd8cbda7bddfa2e8687408ce87237aa0f`

### Summary

- Routine sync from Phoenix `0.4.58` to `0.4.59`. The synced diff only touches the package version bump and internal localnet test-harness/test files (`ts/tests/test-harness/localnet.ts`, `ts/tests/sdk-localnet-flows.test.ts`) — no changes to published package source were included.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- None identified in the synced diff.

## v0.4.58 - 2026-06-30

Source Phoenix commit: `1fdded8e2b1c9104c74a89a0f8ce79e4ad9a9873`

### Summary

- `Trader.maxPositions` and `SubaccountInfo.maxPositions` now decode as `number` instead of `bigint`, matching the on-chain layout change from a `u64` field to a `u32` field.
- `Trader` gains a new `traderPreferenceBits: number` field occupying the upper 32 bits that were previously part of `maxPositions`.
- `toMaxPositions()` now returns `number` instead of `bigint` (cross → `128`, isolated → `1`).
- `Trader.positions` entries are now decoded with a type-aware decoder that silently skips non-position entries (asset IDs ≥ `0xff000000`); `positions.len` reflects only decoded position entries rather than the raw stored count.

### Breaking Changes

- **`Trader.maxPositions`: `bigint` → `number`** — downstream code comparing or storing this field as `bigint` will fail TypeScript compilation; replace `128n`/`1n` literals with `128`/`1`.
- **`SubaccountInfo.maxPositions`: `bigint` → `number`** — same migration needed for any code that constructs or destructures `SubaccountInfo`.
- **`toMaxPositions()` return type: `bigint` → `number`** — callers passing the result to an instruction parameter expecting `bigint` must now wrap it with `BigInt(toMaxPositions(...))`.

### Consumer Notes

- Add `traderPreferenceBits: number` to any local types that mirror or extend `Trader`; it will be `0` for all existing accounts until the on-chain program populates it.
- If your code compares `positions.len` against the number of decoded entries, note that the two values can now differ when the position map contains header-extension slots; iterate `positions.entries` rather than relying on `positions.len` for the count.

## v0.4.57 - 2026-06-29

Source Phoenix commit: `d087e01780d6f8cfadb10005c6607f7de59d3de2`

### Summary

- New Spline instruction builders (`buildRegisterSplineIx`, `buildDeactivateSplineIx`, `buildUpdateSplinePriceIx`, `buildUpdateSplinePriceWithOrderingIx`, `buildUpdateSplineParametersWithOrderingIx`, `buildUpdateSplinePositionLimitsConfigIx`) plus all associated parameter types, account types, and codecs are now publicly exported.
- `decodePerpAssetMap`, `fetchPerpAssetMap`, and `getPerpAssetMapDecoder` are now exported from the package root.
- `HAWKEYE_DISCRIMINANTS` is now exported from `hawkeye.ts`; `FLIGHT_DISCRIMINANTS` gains five additional entries (`INIT`, `NAME_SUCCESSOR`, `CLAIM_SUCCESSOR`, `UPDATE_GLOBAL_STATE`, `UPDATE_BUILDER_STATUS`); `DISCRIMINANTS` gains five Spline entries.
- Two new test-fixture files (`sdk-account-fixtures.json`, `sdk-instruction-fixtures.json`) are now listed in the package `exports` map and accessible as subpath imports.

### Breaking Changes

- **`MarkPrice.oracleLastUpdatedTimestamps` is now a required field (`bigint[]`).** Previously this field was decoded but silently dropped; it is now part of the public `MarkPrice` interface. TypeScript builds that construct `MarkPrice` objects directly (e.g. in mocks or tests) or that use exhaustive destructuring will fail to compile until the field is added.

### Consumer Notes

- Spline instructions follow the same two-argument pattern as other update builders: `buildUpdateSpline*(accounts: SplineUpdateAccounts, params: ...)`. Register and deactivate take a single params object that includes account fields.
- `clientOrderId` in ordering variants is an optional `Uint8Array` of exactly 16 bytes; omitting it defaults to a zero-filled 16-byte array. `overrideSequenceNumber` defaults to `false` when omitted.
- `MAX_SPLINE_REGIONS` (= 10) is exported as a named constant; `buildUpdateSplineParametersWithOrderingIx` will throw at call-time if either `bidRegions` or `askRegions` exceeds this limit, or if both arrays are empty.
- `PositionSizeLimit` is a discriminated union `{ __kind: "Disabled" } | { __kind: "Limit"; value: PositionSizeLimits }`; pass `null` for `maxPositionSize` or `leverageDecreaseInBps` to leave that field unchanged (at least one must be non-null or the builder throws).

## v0.4.56 - 2026-06-26

Source Phoenix commit: `0034c63597c926482b11094a6ab661ffcc127896`

### Summary

- **Scale order API** (`scaleOrders.ts`) is now exported from the package root: `computeScaleOrderLevels`, `previewScaleOrder`, `scaleLevelsToMultipleOrderPacket`, `chunkScaleLevelsForTx`, their associated types (`ScaleOrderInput`, `ScaleOrderLevel`, `ScaleOrderPreview`, `ScaleOrderWarning`, `ScaleOrderWarningCode`, `ScaleLevelsToPacketOptions`), and constants (`MIN_SCALE_ORDERS`, `MAX_SCALE_ORDERS`, `MIN_SCALE_BIAS`, `MAX_SCALE_BIAS`, `DEFAULT_MIN_BASE_LOTS_PER_ORDER`, `DEFAULT_MAX_ORDERS_PER_TX`).
- **`buildPlaceMultiLimitOrderFlow`** and its four types (`PlaceMultiLimitOrderFlowParams`, `PlaceMultiLimitOrderFlowBatch`, `PlaceMultiLimitOrderFlowBatchInstructions`, `PlaceMultiLimitOrderFlowResult`) are now exported — a high-level flow that handles subaccount allocation, optional registration, collateral transfer, and post-placement sweep for both cross and isolated margin, splitting large ladders across transaction batches automatically.
- **`buildPlaceMultiLimitOrderIxResolved`** / **`BuildPlaceMultiLimitOrderIxResolvedInput`** added alongside the existing resolved instruction builders.
- **`ticksToUsdWithMarketParams`** is now exported from the package root — the inverse of `priceUsdToTicksWithMarketParams`, useful for converting on-chain tick prices back to human-readable USD for display or preview.
- **`AccountFetcherClient`** gains an optional `fetchMaybeAccounts` method; `PhoenixRpcAccountFetcherClient` now implements it, batching up to 100 addresses per `getMultipleAccounts` round-trip and returning `null` for missing accounts.

### Breaking Changes

- **`buildPlaceMultiLimitOrderIx` now marks `globalConfigurationAddress` writable.** The on-chain program requires this account writable; the previous readonly flag caused transactions to be rejected at the program loader. Any caller constructing this instruction manually should update their account-role assumptions.

### Consumer Notes

- Scale ladders larger than `DEFAULT_MAX_ORDERS_PER_TX` (30) produce multiple `PlaceMultiLimitOrderFlowBatch` entries in `PlaceMultiLimitOrderFlowResult.batches`; each batch must be submitted as a separate transaction. Batches are **not atomic across transactions** — a mid-ladder failure can leave a partial position, and for isolated margin, collateral may be funded but not yet swept back to the parent.
- For isolated margin without an explicit `subaccountIndex`, `buildPlaceMultiLimitOrderFlow` requires `transferAmount > 0` to fund the child subaccount; omitting it throws at construction time.
- The `fetchMaybeAccounts` addition to `AccountFetcherClient` is **optional** — existing client implementations that do not implement it continue to work via the per-address fallback path.
## v0.4.55 - 2026-06-25

Source Phoenix commit: `2f2bb2ba1256f28465278ff1c00d61f0ffc2c5f2`

### Summary

- Added two new methods to `V1ExchangeClient`: `buildRegisterIxs` (calls `POST /v1/exchange/build-register-ixs`) and `sendRegisterIxs` (calls `POST /v1/exchange/send-register-ixs`), enabling builders to register and onboard a trader without a referral code.
- Exported six new public types and their Zod schemas from both the package root (`index.ts`) and the public-api-schemas barrel: `BuildRegisterIxsRequest`, `BuildRegisterIxsResponse`, `SendRegisterIxsRequest`, `SendRegisterIxsResponse`, `RegisterIxInstruction`, and `RegisterIxAccountMeta`.
- Added `examples/10-builder-onboarding-tx.ts`, a runnable end-to-end example of the builder onboarding flow: fetch instructions, partially sign locally, submit to the API for co-signing, simulation, and broadcast.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- The builder onboarding flow is two steps: call `client.api.exchange().buildRegisterIxs({ traderAuthority, txFeePayer, maxPositions? })` to receive the instruction list, build and partially sign the transaction, then call `client.api.exchange().sendRegisterIxs({ transaction, traderAuthority, txFeePayer, ... })` to have the API validate, co-sign as onboarder, simulate, and send. `maxPositions` is optional on both calls (32–128; default 128).
- This onboarding path is distinct from the referral-code path (`POST /v1/referral/activate-tx`) and the access-code path (`POST /v1/invite/activate`). Use `build-register-ixs` / `send-register-ixs` when the builder controls the fee payer and no referral code is involved.

## v0.4.54 - 2026-06-25

Source Phoenix commit: `190b6af2b821bd0670b4a1aea8dad552bcd62482`

### Summary

- **Service-account authentication for server-side tools.** `PhoenixAuthClient` gains three new methods — `loginWithServiceAccountSigner`, `loginWithServiceAccountCredential`, and `loginWithServiceAccountFromEnv` — enabling non-interactive authentication using Ed25519 keypair credentials.
- **New public exports from `@ellipsis-labs/rise/auth`:** `PhoenixServiceAccountCredential`, `PhoenixServiceAccountAuthSigner`, `PhoenixServiceAccountAuthClient`, `PhoenixServiceAccountCredentialEnv`, `createServiceAccountAuthSigner`, `loginWithServiceAccountCredential`, `loginWithServiceAccountSigner`, `loginWithServiceAccountFromEnv`, `loadServiceAccountCredentialFromEnv`, and `loadServiceAccountCredentialFromPath`.
- **Env-var credential loading** mirrors the Rise Rust SDK: set `PHOENIX_SERVICE_ACCOUNT_CREDENTIAL` (path to a JSON file) or the split vars `PHOENIX_SERVICE_ACCOUNT_CLIENT_ID` / `PHOENIX_SERVICE_ACCOUNT_KEY_ID` / `PHOENIX_SERVICE_ACCOUNT_PRIVATE_KEY`. Legacy `PHOENIX_SERVICE_CLIENT_ID`, `PHOENIX_SERVICE_KEY_ID`, and `PHOENIX_SERVICE_PRIVATE_KEY` aliases are also accepted.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- Credential file loading (`PHOENIX_SERVICE_ACCOUNT_CREDENTIAL` / `loadServiceAccountCredentialFromPath`) uses `node:fs/promises` and is only available in Node.js and Bun environments; browser bundles that import `loadServiceAccountCredentialFromEnv` with a file path will throw at runtime. Split env-var loading has no Node dependency and works anywhere `process.env` is accessible.
- Partial split-env configuration (some but not all of `CLIENT_ID`, `KEY_ID`, `PRIVATE_KEY` present) throws `PhoenixAuthError` with code `incomplete_service_account_credential_env` rather than silently falling through.
- Private keys must be 32-byte Ed25519 seeds encoded as base64url (no padding); a mismatched length throws `PhoenixAuthError` with code `invalid_service_account_private_key`.

## v0.4.53 - 2026-06-25

Source Phoenix commit: `823bb5e64efafb221f37a7e0a8f8720da843741c`

### Summary

- **Automatic trader account detection**: `buildActivateReferralTxRequest` now accepts optional `rpc` or `rpcUrl` parameters. When provided and `includeRegisterTrader` is not set explicitly, the SDK queries the on-chain trader account and automatically prepends `register_trader` if the account does not exist.
- **Opt-in trader registration in a single transaction**: `buildReferralActivationTransaction` and `buildActivateReferralTxRequest` accept two new optional fields — `includeRegisterTrader: boolean` and `registerTraderMaxPositions?: bigint` (32–128, default 128) — to prepend `register_trader` before delegated onboarding.
- **New utility exports**: `getReferralActivationTraderState` inspects whether a trader PDA is `"missing"`, `"registered"`, or `"activated"`; `hasReferralActivationCapabilities` tests raw trader capability flags.
- **New types exported**: `ReferralActivationRpc`, `ReferralActivationTraderState`, `ReferralActivationTraderStatus`, and `ReferralActivationTransaction`.
- **Enriched result from `buildActivateReferralTxRequest`**: the returned object now includes `includeRegisterTrader: boolean` and `traderActivationState?: ReferralActivationTraderState`.

### Breaking Changes

- `ReferralActivationTransactionBuild.transaction` is now typed as `ReferralActivationTransaction` (`Transaction & TransactionWithLifetime`) instead of plain `Transaction`. Existing callers assigning the result to a `Transaction`-typed variable will still compile, but implementors of `ReferralActivationTransactionSigner` whose callback is explicitly annotated with the old `(transaction: Transaction, ...) => ...` signature may see a type mismatch when the annotation is updated to the new stricter type — update to `ReferralActivationTransaction` or remove the explicit annotation.

### Consumer Notes

- `rentPayer` is still deprecated, but its behavior changed: when `includeRegisterTrader: true` is set and no `feePayer` is provided, `rentPayer` is now used as the fallback fee payer for the registration instruction. Existing callers that pass `rentPayer` without `includeRegisterTrader: true` are unaffected — it continues to be ignored in that case.
- `registerTraderMaxPositions` must be between `32n` and `128n`; passing a value outside this range throws synchronously before any RPC call.
- To use auto-detection, pass `rpc` (a `createSolanaRpc` instance) or `rpcUrl` to `buildActivateReferralTxRequest` and omit `includeRegisterTrader`. The returned `traderActivationState` reflects what the SDK observed on-chain.

## v0.4.52 - 2026-06-25

Source Phoenix commit: `f9c0326d728b06047d21c64915a6a72f8bcb1afb`

### Summary

- Added `withdrawalsAvailable: boolean` to `ExchangeStatusView`, `ExchangeStateSnapshot`, `ExchangeStatusChangedOp`, and `ExchangeStatusPayload` — reflects whether the exchange withdraw queue is open for withdrawals (requires exchange to be active and withdraw throttle budget to be non-zero).
- RPC snapshot loading now fetches the withdraw queue header in parallel, using it to populate `withdrawalsAvailable`; a fetch failure is treated as available (`true`).
- `exchangeUpdated` events in the cache store now include `withdrawalsAvailable`, and a change in its value triggers a `"status"` exchange change notification.
- Wire adapters normalize the snake_case alias `withdrawals_available` from the server to `withdrawalsAvailable`, defaulting to `true` when the field is absent (backward-compatible with older server payloads).

### Breaking Changes

- `ExchangeStatusView`, `ExchangeStateSnapshot`, `ExchangeStatusChangedOp`, and `ExchangeStatusPayload` each gain a required `withdrawalsAvailable: boolean` field. Consumers who construct or assert on these types directly (e.g., in tests or typed fixtures) must add this field. Wire-parsed values from the server default to `true` when the field is missing, so runtime parsing is non-breaking.

### Consumer Notes

- Read `snapshot.exchange.withdrawalsAvailable` or `status.withdrawalsAvailable` to determine whether withdrawals are currently permitted before initiating a withdrawal flow.
- No action needed for pure subscribers: existing server messages without `withdrawalsAvailable` are handled transparently with a `true` default.

## v0.4.51 - 2026-06-24

Source Phoenix commit: `f437eb2cee153f012a3ab34e799623d34defb07f`

### Summary

- Added a transaction-based referral activation flow: `V1InviteClient` now exposes `getReferralActivationPermission()` (`GET /v1/referral/activation-permission`), `activateReferralTx(request)` (`POST /v1/referral/activate-tx`), and `buildActivateReferralTxRequest(params)`, which fetches permission and exchange state automatically, builds the Solana transaction, and invokes a caller-supplied signer callback before returning the ready-to-submit request.
- Exported low-level transaction helpers at the package root for integrators who need finer control: `buildReferralActivationTransaction`, `buildActivateReferralTxRequest`, `referralActivationExchangeAccountsFromSnapshot`, and `serializeReferralActivationSignedTransaction`.
- Added new public types and Zod schemas: `ActivateReferralTxRequest/Response`, `ActivateReferralTxStatus`, `ACTIVATE_REFERRAL_TX_STATUSES`, `ReferralActivationPermissionResponse`, `ReferralActivationExchangeAccounts`, `ReferralActivationTransactionBuild`, `ReferralActivationTransactionSigner`, and related builder param/result interfaces.
- Added `encodeBytesToBase64` to the internal base64 module; works in Node (via `Buffer`), Bun, and browser (via `btoa`).

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- The `buildActivateReferralTxRequest` convenience method on `V1InviteClient` makes parallel HTTP calls for the permission account and exchange snapshot; pass pre-fetched `permission` and/or `exchangeAccounts` in the params object to skip those fetches.
- The `signTransaction` callback in `BuildActivateReferralTxRequestParams` accepts a Kit `Transaction`, a base64 string, raw `Uint8Array`/`ArrayBuffer`, or any object with a `serialize()` method — covering both `@solana/kit` and legacy `@solana/web3.js` `VersionedTransaction` adapters.
- `rentPayer` on `BuildReferralActivationTransactionParams` is deprecated: the tx-based activation flow no longer creates trader accounts on-chain. Register the trader before calling this flow.

## v0.4.50 - 2026-06-23

Source Phoenix commit: `d1ccda811fc68e0cbf1fbf5d3e6d0235d508c3db`

### Summary

- New `buildSetPermissionDelegatedIx` instruction builder and `SetPermissionDelegated` codec (encoder/decoder) allow a delegated trader-management key to grant trader-onboarding permission to a third-party user key, without requiring the risk authority to sign directly.
- Two permission bitmask constants exported: `TRADER_ONBOARDING_PERMISSION` (`1n << 4n`) and `TRADER_MANAGEMENT_PERMISSION` (`1n << 7n`).
- New types exported: `SetPermissionDelegatedParams`, `SetPermissionDelegatedIx`, `SetPermissionDelegatedAccounts`.
- **Bug fix**: `buildCreatePermissionIx` now marks the `payer` account as writable (was previously readonly-signer); callers constructing transactions directly may see a changed account meta for this field.
- New example `08-delegated-trader-management-onboarding.ts` demonstrates the full external-team onboarding flow end-to-end.

### Breaking Changes

- `buildCreatePermissionIx`: the `payer` account is now `writable + signer` instead of `readonly + signer`. If you are serializing or comparing raw account metas from this instruction, the account flags will differ from 0.4.49.

### Consumer Notes

- To use delegated trader-management onboarding, call `buildSetPermissionDelegatedIx` with a `SetPermissionDelegatedParams` — the authority can be the risk authority itself or any key that holds `TRADER_MANAGEMENT_PERMISSION`. Pass `authorityPermissionAccount` as the authority's own address when it is the risk authority, or as the PDA from `getPermissionAddress` otherwise.
- `TRADER_ONBOARDING_PERMISSION` and `TRADER_MANAGEMENT_PERMISSION` are exported as `bigint` constants; use them for bitwise checks against `permission.permission` fields.
- The new codec exports (`getSetPermissionDelegatedInstructionEncoder/Decoder/Codec`) follow the same pattern as the existing `SetPermission` variants and are available for manual instruction construction or decoding.

## v0.4.49 - 2026-06-23

Source Phoenix commit: `e8c6dd02a2ac562e3d38cf3c4b3027b39ae8985f`

### Summary

- Updated example source-reference comments to reflect the current Rust example path (`rise/rust/examples/` instead of the former `rise/rust/sdk/examples/`) across `01-http-client.ts`, `02-ws-fills.ts`, `03-build-limit-order-ix.ts`, and `examples/README.md`.
- No changes to public TypeScript exports, HTTP client, WebSocket adapters, or generated types in this release.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- None identified in the synced diff.

## v0.4.48 - 2026-06-22

Source Phoenix commit: `a419e23d7d1b2a3e37696d76e85fac7f0a023f5e`

### Summary

- Added `UncrossCrank` instruction: triggers an uncross crank on an orderbook to match resting crossed orders. Exported as `buildUncrossCrankIx`, `buildUncrossCrank`, `uncrossCrank`, `buildUncrossCrankIxResolved`, and associated types/codecs (`UncrossCrankParams`, `UncrossCrankIx`, `UncrossCrankAccounts`, `UncrossCrankInstruction`).
- Promoted `CancelUpTo` to the full public SDK surface: `buildCancelUpTo`, `cancelUpToOrders`, `buildCancelUpToIxResolved`, `BuildCancelUpToIxResolvedInput`, `ClientCancelUpToInput` are now exported and available on `client.ixs`.
- `cancelAllOrders` now accepts an optional `traderSubaccountIndex` in its options (defaults to `0`; fully backward compatible).

### Breaking Changes

- **`buildCancelUpToIx` account-role change**: `globalConfigurationAddress` is now passed as **writable** instead of readonly. Any pre-built or cached `CancelUpTo` transaction assembled with `0.4.47` will fail on-chain due to the account-meta mismatch. Rebuild affected transactions after upgrading.

### Consumer Notes

- `buildUncrossCrankIxResolved` defaults `matchLimit` to `100n` when the parameter is omitted; pass an explicit value to override.
- `UncrossCrankParams.matchLimit` is typed as `bigint` (required, non-negative). The high-level `buildUncrossCrank` / `ClientUncrossCrankInput` accept `bigint | number` and coerce automatically.
- All new exports are available from the package root (`@ellipsis-labs/rise`); no sub-path import changes are required.

## v0.4.47 - 2026-06-22

Source Phoenix commit: `472a79816f0e119910a009795ae3c8cd7e39b054`

### Summary

- The default localnet fixture (`test-fixtures/default-localnet.json`) is now bundled in the npm package and exported as a named package path, enabling consumers to import it directly without a local copy.
- Market entries in the localnet fixture now include `defaultTakerFeeMicro` (350) and `defaultMakerFeeMicro` (50) fields for BTC, ETH, and SOL markets.
- A new test assertion verifies that the packaged fixture copy stays in sync with the canonical root fixture.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- **New package export**: `@ellipsis-labs/rise/test-fixtures/default-localnet.json` is now a stable named export. Consumers can import the default localnet fixture directly from the package instead of copying it or referencing it by relative path.
- **Fixture schema addition**: Each market object in the localnet fixture now carries `defaultTakerFeeMicro` and `defaultMakerFeeMicro` numeric fields. Code that deserializes the fixture into a typed struct should treat these as new optional fields; strict deserializers that reject unknown fields will need updating.
- The `test-fixtures/` directory is now included in the published `files` list; the fixture JSON is part of the public package surface and subject to semver considerations going forward.

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

## v0.4.38 - 2026-06-15

Source Phoenix commit: `978cab228c047f8e511a8075621846c679d331d5`

### Summary

- Bumped `ws` peer dependency minimum from `>=8.20.1` to `>=8.21.0`.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- If you pin `ws` to an exact version below `8.21.0`, you may see peer dependency warnings or conflicts. Update your lockfile or pin to `ws@8.21.0` or later.

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

## v0.4.36 - 2026-06-15

Source Phoenix commit: `8809c4d39c7070f6430c20dd5fe670ad86032523`

### Summary

- Version bumped from `0.4.35` to `0.4.36`.
- No functional code, API, or type changes are included in this sync — the diff contains only the version field update in `package.json`.

### Breaking Changes

- None identified in the synced diff.

### Consumer Notes

- None identified in the synced diff.

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
