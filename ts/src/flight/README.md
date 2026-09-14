# Flight Routing

This document shows how to use `rise` with the Flight program so supported
Phoenix placement instructions are proxied through Flight.

> **Use embedded wallets for Flight integrations.** Integrators are strongly
> encouraged to provision an embedded wallet per user rather than routing
> Flight orders through the user's raw/external wallet. A raw wallet shares
> its Phoenix trader state with every other dapp the user trades on, which
> results in bad UX (positions, balances, and orders bleeding across
> integrations). Embedded wallets isolate per-integration state.

## Summary

The recommended path is to configure `flight` on `createPhoenixClient(...)`.
When you do that:

- `client.ixs.buildPlaceLimitOrder(...)` returns a Flight proxy instruction
- `client.ixs.buildPlaceMarketOrder(...)` returns a Flight proxy instruction
- `client.ixs.buildPlaceStopLoss(...)` returns a Flight proxy instruction
- `client.ixs.buildPlacePositionConditionalOrder(...)` returns a Flight proxy
  instruction
- `client.ixs.buildPlaceAttachedConditionalOrder(...)` returns a Flight proxy
  instruction
- `client.ixs.buildPlaceLimitOrderWithConditionals(...)` returns a Flight proxy
  instruction
- `client.ixs.placeLimitOrder(...)` returns a Flight proxy instruction
- `client.ixs.placeMarketOrder(...)` returns a Flight proxy instruction
- `client.ixs.placePositionConditionalOrder(...)` returns a Flight proxy
  instruction
- `client.api.orders().placeIsolatedLimitOrder(...)`,
  `client.api.orders().placeIsolatedMarketOrder(...)`,
  `client.api.orders().placeIsolatedLimitOrderEnhanced(...)`,
  `client.api.orders().placeIsolatedMarketOrderEnhanced(...)`,
  `client.api.orders().placeIsolatedLimitOrderWithConditionals(...)`,
  `client.api.orders().placeStopLossOrder(...)`,
  `client.api.orders().placeAttachedConditionalOrder(...)`, and
  `client.api.orders().placePositionConditionalOrder(...)` inherit the same
  Flight builder defaults

Post-only orders remain native Phoenix instructions.

For most integrations, the recommended client settings are:

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

Recommended settings:

- Prefer `apiUrl`; `baseUrl` is only a deprecated alias.
- Prefer configuring Flight once on the client instead of repeating builder
  fields per request.
- Configure Flight with `builderAuthority`, plus optional `builderPdaIndex`
  and `builderSubaccountIndex`. rise derives the builder trader account from
  those values, with both indexes defaulting to `0`.
- Set `feeBpsOverride` only when you need an individual integration route to
  use Flight's `proxy_instruction_with_fee_override` instruction instead of the
  builder's registered fee.
- Keep `exchangeMetadata: { stream: true }` enabled for long-running apps that
  build orders against live market metadata.
- One rule for the collateral-transfer tail: **signing as a position
  authority ⇒ declare it**. Wraps never infer the tail from the wrapped
  instruction — a plain wrap of a delegated market order appends nothing,
  because owner-signed `PlaceMarketOrderDelegated` settles the builder fee
  via the plain transfer on-chain, and the tail write-locks a global
  permission account, so it is opt-in by signer kind only. On the high-level
  `client.ixs` order methods the declaration is derived for you: pass the
  delegate as `positionAuthority` and the wrap takes the position-authority
  path whenever the effective signer (`positionAuthority ?? authority`)
  differs from `authority`.
- Position-authority wraps append a collateral-transfer permission account
  derived from the Phoenix root authority. `rise` always resolves that root
  authority from the client's exchange metadata snapshot at wrap time — it
  is never configured manually and never cached, so on-chain authority
  rotations (streamed as `exchangeKeysUpdated` deltas in websocket mode, or
  picked up by HTTP refreshes otherwise) are reflected automatically.
- Keep PDA caching enabled. `pdaCache: { maxEntries: 1024 }` is the explicit
  default for long-lived processes.

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

The builder trader that collects fees is always derived from:

- `builderAuthority`
- `builderPdaIndex ?? 0`
- `builderSubaccountIndex ?? 0`

There is no root-authority configuration. When a wrap needs the Phoenix root
authority (position-authority orders), the client reads it from the exchange
metadata snapshot at wrap time, so a root-authority rotation on-chain never
leaves the client deriving a stale permission PDA.

## Position-Authority (Delegate-Signed) Orders

The trader PDA always derives from `authority` (the owner). When a delegate
key signs instead of the owner, pass it as `positionAuthority` on the
high-level order methods — mirroring the API server DTOs, the effective
signer is `positionAuthority ?? authority`, and the path is picked from the
pure comparison of that signer with `authority`, never from the wrapped
instruction:

```ts
const ix = await client.ixs.placeMarketOrder({
  authority: ownerWallet, // trader account owner; trader PDA derives from it
  positionAuthority: delegateWallet, // signs the transaction
  symbol: "SOL-PERP",
  orderPacket,
});
```

With `positionAuthority` set (and different from `authority`), the Flight
wrap appends the collateral-transfer authority and permission accounts so
the builder fee can move via `AuthorizedTransferCollateral`; the permission
account derives from the Phoenix root authority resolved from the exchange
snapshot, and the collateral-transfer authority is scoped to the exact
Phoenix program address. `client.ixs.placeMarketOrderDelegated(...)` follows
the same rule via its effective signer
(`traderWallet ?? positionAuthority ?? authority`).

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

With `flight` configured, `ix` is a Flight proxy instruction. The inner Phoenix
`place_limit_order` instruction is wrapped automatically by the client.

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

In this case the builder fee collector trader is derived automatically from the
configured builder authority and indexes.

## Use The Server-Built Order API With Flight Defaults

If you use the typed API order helpers, the client also injects Flight builder
defaults automatically when `flight` is configured:

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

The request can still override:

- `flightBuilderAuthority`
- `flightFeeCollectorTrader`

But if you omit them, the client fills them from its configured `flight`
defaults.

If you override `flightBuilderAuthority` but omit
`flightFeeCollectorTrader`, `rise` derives the fee collector trader for the
override builder using the configured builder PDA/subaccount defaults.

## Build A Flight Market Order Ix

[`examples/06-flight-market-order.ts`](../../examples/06-flight-market-order.ts)
is a minimal example that takes builder + trader CLI args and builds the
Flight-wrapped market-order instruction via `client.ixs.placeMarketOrder(...)`:

```bash
bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]
```

Set `POSITION_AUTHORITY=<DELEGATE_ADDRESS>` to build the delegate-signed
variant, where the delegate is passed as `positionAuthority` and the wrap
goes through the position-authority path:

```bash
POSITION_AUTHORITY=<DELEGATE_ADDRESS> bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]
```

## Wrap A Native Order Ix Manually

If you already built a native Phoenix order instruction and just want to wrap
it with Flight, use `flight.wrapInstructionWithFlight(...)`:

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

`wrapInstructionWithFlight(...)` only wraps supported placement instructions.
Unsupported instructions are returned unchanged. `signer` is the wallet that
signs the wrapped instruction — the effective signer
(`positionAuthority ?? ownerAuthority`), not necessarily the trader account's
owner.

If `signer` signs as the trader's position authority, declare it with
`usePositionAuthority: true` and pass `resolveRootAuthority` so the wrapper
can derive the collateral-transfer permission account (the declaration is
the only trigger — the wrapper never infers the tail from the instruction,
so owner-signed delegated market orders wrap plainly). Source the root
authority from live exchange metadata rather than a hardcoded value:

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

If you already have a `PhoenixInstructionClient`, you can wrap it with
`flight.PhoenixFlightClient` and reuse its builder trader configuration:

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

This is the lower-level path if you are integrating Flight into an existing ix
builder stack rather than using `createPhoenixClient(...)`.

`tryWrapOrderInstruction(ix, signer, usePositionAuthority = false)` is the
single wrap entry, an exact mirror of the Rust rise client's
`try_wrap_order_instruction`: `signer` is the wallet that signs the wrapped
instruction, and `usePositionAuthority` declares that the signer is the
trader's position authority. Derive the flag as `signer !== ownerAuthority`
when the owner is known — never from the instruction being wrapped. Left
`false` (owner-signed), the wrap never appends the collateral-transfer tail,
delegated market orders included; set `true`, it always appends it.
Unsupported instructions come back unchanged, so the result is a plain
`InstructionsWithAccountsAndData`, not necessarily a proxy instruction.

For position-authority wraps (`usePositionAuthority: true`), the wrapped
`instructionClient` must expose exchange metadata
(`instructionClient.exchange`) — `PhoenixFlightClient` resolves the current
Phoenix root authority from that snapshot on every wrap and throws if the
metadata is unavailable.
