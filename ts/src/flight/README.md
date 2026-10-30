# Flight Routing

Proxy supported Phoenix order-placement instructions through the Flight program.

> **Use embedded wallets for Flight integrations.** Provision an embedded wallet
> per user rather than routing Flight orders through the user's raw/external
> wallet. A raw wallet shares its Phoenix trader state across every dapp the
> user trades on, causing positions, balances, and orders to bleed across
> integrations. Embedded wallets isolate per-integration state.

## Summary

Configure `flight` on `createPhoenixClient(...)`:

- Local instruction builders return Flight proxy instructions:
  `client.ixs.buildPlaceLimitOrder(...)`,
  `client.ixs.buildPlaceMarketOrder(...)`, `client.ixs.buildPlaceStopLoss(...)`,
  `client.ixs.buildPlacePositionConditionalOrder(...)`,
  `client.ixs.buildPlaceAttachedConditionalOrder(...)`,
  `client.ixs.buildPlaceLimitOrderWithConditionals(...)`,
  `client.ixs.placeLimitOrder(...)`, `client.ixs.placeMarketOrder(...)`, and
  `client.ixs.placePositionConditionalOrder(...)`.
- Server-built order helpers inherit the same Flight builder defaults:
  `client.api.orders().placeIsolatedLimitOrder(...)`,
  `client.api.orders().placeIsolatedMarketOrder(...)`,
  `client.api.orders().placeIsolatedLimitOrderEnhanced(...)`,
  `client.api.orders().placeIsolatedMarketOrderEnhanced(...)`,
  `client.api.orders().placeIsolatedLimitOrderWithConditionals(...)`,
  `client.api.orders().placeStopLossOrder(...)`,
  `client.api.orders().placeAttachedConditionalOrder(...)`, and
  `client.api.orders().placePositionConditionalOrder(...)`.
- Post-only orders remain native Phoenix instructions.

Recommended client configuration:

```ts
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  pdaCache: { maxEntries: 1024 },
  exchangeMetadata: {
    stream: true,
  },
  flight: {
    builderAuthority: "Builder111111111111111111111111111111111",
    builderPdaIndex: 0,
    builderSubaccountIndex: 0,
  },
});
```

Configuration rules:

- Use `apiUrl` (`baseUrl` is a deprecated alias) and keep
  `pdaCache: { maxEntries: 1024 }` (the default) plus
  `exchangeMetadata: { stream: true }` enabled for long-lived processes.
- Configure Flight once on the client with `builderAuthority` and optional
  `builderPdaIndex` / `builderSubaccountIndex` (both default to `0`). `rise`
  derives the builder trader account from those values.
- Set `feeBpsOverride` only when a route must emit Flight's
  `proxy_instruction_with_fee_override` instruction instead of using the
  builder's registered fee.
- **Signing as a position authority ⇒ declare it.** Wraps never infer the
  collateral-transfer tail from the wrapped instruction: owner-signed
  `PlaceMarketOrderDelegated` settles the builder fee via a plain transfer
  on-chain, and the tail write-locks a global permission account, so it is
  opt-in by signer kind only. High-level `client.ixs` order methods derive this
  automatically when the effective signer (`positionAuthority ?? authority`)
  differs from `authority`.
- Position-authority wraps derive the collateral-transfer permission account
  from the Phoenix root authority in the client's exchange metadata snapshot at
  wrap time. The root authority is never configured manually or cached, so
  on-chain authority rotations (`exchangeKeysUpdated` websocket deltas or HTTP
  refreshes) apply automatically.

## Configure Flight On The Client

```ts
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  pdaCache: { maxEntries: 1024 },
  exchangeMetadata: {
    stream: true,
  },
  flight: {
    builderAuthority: "Builder111111111111111111111111111111111",
    builderPdaIndex: 0,
    builderSubaccountIndex: 0,
  },
});
```

The fee-collecting builder trader derives from `builderAuthority`,
`builderPdaIndex ?? 0`, and `builderSubaccountIndex ?? 0`. Root authority
requires no manual configuration; position-authority wraps read it from the
exchange metadata snapshot at wrap time so on-chain rotations never produce a
stale permission PDA.

## Position-Authority (Delegate-Signed) Orders

The trader PDA always derives from `authority` (the owner). When a delegate
signs instead, pass `positionAuthority` on high-level order methods. Mirroring
the API server DTOs, the effective signer is `positionAuthority ?? authority`,
and the wrap selects the position-authority path strictly by comparing that
signer with `authority`:

```ts
const ix = await client.ixs.placeMarketOrder({
  authority: ownerWallet, // trader account owner; trader PDA derives from it
  positionAuthority: delegateWallet, // signs the transaction
  symbol: "SOL-PERP",
  orderPacket,
});
```

When `positionAuthority` differs from `authority`, the wrap appends the
collateral-transfer authority (scoped to the Phoenix program address) and
permission account (derived from the exchange snapshot's Phoenix root authority)
so the builder fee settles via `AuthorizedTransferCollateral`.
`client.ixs.placeMarketOrderDelegated(...)` applies the same check using
`traderWallet ?? positionAuthority ?? authority`.

## Build A Flight-Wrapped Limit Order Ix

```ts
import { Side, createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  pdaCache: { maxEntries: 1024 },
  exchangeMetadata: {
    stream: true,
  },
  flight: {
    builderAuthority: "Builder111111111111111111111111111111111",
    builderPdaIndex: 0,
    builderSubaccountIndex: 0,
  },
});

await client.exchange.ready();

const orderPacket = await client.ixs.orderPackets.buildLimitOrderPacket({
  symbol: "SOL-PERP",
  side: Side.Bid,
  priceUsd: "135.87",
  baseUnits: "0.25",
});

const ix = await client.ixs.buildPlaceLimitOrder({
  authority: "Authority111111111111111111111111111111111",
  symbol: "SOL-PERP",
  orderPacket,
});
```

With `flight` configured, `ix` is a Flight proxy instruction wrapping the inner
Phoenix `place_limit_order` instruction.

## Build A Flight-Wrapped Market Order Ix

```ts
import { Side, createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  pdaCache: { maxEntries: 1024 },
  exchangeMetadata: {
    stream: true,
  },
  flight: {
    builderAuthority: "Builder111111111111111111111111111111111",
    builderPdaIndex: 0,
    builderSubaccountIndex: 0,
  },
});

await client.exchange.ready();

const orderPacket = await client.ixs.orderPackets.buildMarketOrderPacket({
  symbol: "SOL-PERP",
  side: Side.Ask,
  baseUnits: "0.50",
  priceLimitUsd: "135.80",
});

const ix = await client.ixs.buildPlaceMarketOrder({
  authority: "Authority111111111111111111111111111111111",
  symbol: "SOL-PERP",
  orderPacket,
});
```

The client derives the builder fee-collector trader automatically from the
configured `builderAuthority` and indexes.

## Use The Server-Built Order API With Flight Defaults

Typed `client.api.orders()` helpers inject configured `flight` defaults
automatically:

```ts
const instructions = await client.api.orders().placeIsolatedLimitOrder({
  authority: "Authority111111111111111111111111111111111",
  symbol: "SOL",
  side: "Bid",
  pdaIndex: 0,
  price: 135.87,
  quantity: 0.25,
});
```

Omitted `flightBuilderAuthority` and `flightFeeCollectorTrader` fields default
to the client's `flight` config. If you override `flightBuilderAuthority`
without `flightFeeCollectorTrader`, `rise` derives the fee-collector trader for
the override builder using the configured builder PDA and subaccount defaults.

## Build A Flight Market Order Ix

[`examples/06-flight-market-order.ts`](../../examples/06-flight-market-order.ts)
builds a Flight-wrapped market-order instruction from CLI arguments via
`client.ixs.placeMarketOrder(...)`:

```bash
bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]
```

Set `POSITION_AUTHORITY=<DELEGATE_ADDRESS>` to pass a delegate as
`positionAuthority` and use the position-authority wrap path:

```bash
POSITION_AUTHORITY=<DELEGATE_ADDRESS> bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]
```

## Wrap A Native Order Ix Manually

Wrap a pre-built native Phoenix order instruction with
`flight.wrapInstructionWithFlight(...)`:

```ts
import { buildPlaceLimitOrderIxResolved, flight } from "@ellipsis-labs/rise";

const innerIx = buildPlaceLimitOrderIxResolved({
  exchange: resolvedExchange,
  market: resolvedMarket,
  trader: resolvedTrader,
  orderPacket,
});

const wrappedIx = await flight.wrapInstructionWithFlight({
  phoenixInstruction: innerIx,
  signer: "Authority111111111111111111111111111111111",
  phoenixProgramAddress: client.pda.getProgramAddress(),
  flight: {
    builderAuthority: "Builder111111111111111111111111111111111",
    builderPdaIndex: 0,
    builderSubaccountIndex: 0,
    feeBpsOverride: 5n,
  },
  resolveFeeCollectorTraderAddress: (traderPdaIndex, subaccountIndex) =>
    client.pda.getTraderAddress({
      authority: "Builder111111111111111111111111111111111",
      traderPdaIndex,
      subaccountIndex,
      phoenixProgramAddress: client.pda.getProgramAddress(),
    }),
});
```

`wrapInstructionWithFlight(...)` wraps supported placement instructions and
returns unsupported instructions unchanged. `signer` is the effective
transaction signer (`positionAuthority ?? ownerAuthority`), not necessarily the
trader owner.

When `signer` is a position authority, set `usePositionAuthority: true` and
provide `resolveRootAuthority` (sourced from live exchange metadata) to append
the collateral-transfer permission account. The wrapper never infers this tail
from the instruction itself, so owner-signed delegated market orders wrap
without it:

```ts
const wrappedPositionAuthorityIx = await flight.wrapInstructionWithFlight({
  // ... same fields as above ...
  usePositionAuthority: true,
  resolveRootAuthority: async () => {
    await client.exchange.ready();
    return client.exchange.snapshot().exchange.currentAuthorities
      .rootAuthority as Authority;
  },
});
```

## Wrap An Existing `PhoenixInstructionClient`

To integrate Flight into an existing `PhoenixInstructionClient` stack without
`createPhoenixClient(...)`, wrap it with `flight.PhoenixFlightClient`:

```ts
import { flight } from "@ellipsis-labs/rise";

const flightClient = new flight.PhoenixFlightClient(instructionClient, {
  builderAuthority: "Builder111111111111111111111111111111111",
  builderPdaIndex: 0,
  builderSubaccountIndex: 0,
});

const wrappedIx = await flightClient.tryWrapOrderInstruction(
  nativeOrderIx,
  "Authority111111111111111111111111111111111"
);
```

`tryWrapOrderInstruction(ix, signer, usePositionAuthority = false)` mirrors the
Rust SDK's `try_wrap_order_instruction`:

- `signer` is the wallet signing the wrapped instruction.
- `usePositionAuthority` declares that `signer` is a position authority
  (`signer !== ownerAuthority`). Derive this from the signer, never the
  instruction: `false` never appends the collateral-transfer tail (including for
  owner-signed delegated market orders); `true` always appends it.
- Unsupported instructions return unchanged as a plain
  `InstructionsWithAccountsAndData`.
- When `usePositionAuthority: true`, `instructionClient.exchange` must be
  present; `PhoenixFlightClient` reads the Phoenix root authority from that
  snapshot on every wrap and throws if metadata is unavailable.
