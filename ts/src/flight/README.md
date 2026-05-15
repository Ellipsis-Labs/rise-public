# Flight Routing

This document shows how to use `rise` with the Flight program so limit and
market order instructions are proxied through Flight and the configured builder
trader collects builder fees.

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
- `client.ixs.placeLimitOrder(...)` returns a Flight proxy instruction
- `client.ixs.placeMarketOrder(...)` returns a Flight proxy instruction
- `client.api.orders().placeIsolatedLimitOrder(...)` and
  `client.api.orders().placeIsolatedMarketOrder(...)` inherit the same Flight
  builder defaults

Only limit and market order placement are Flight-routed today. Post-only orders
remain native Phoenix instructions.

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
- Keep `exchangeMetadata: { stream: true }` enabled for long-running apps that
  build orders against live market metadata.
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

## Use The Isolated Order API With Flight Defaults

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
  authority: "Authority111111111111111111111111111111111",
  phoenixProgramAddress: client.pda.getProgramAddress(),
  flight: {
    builderAuthority: "Builder111111111111111111111111111111111",
    builderPdaIndex: 0,
    builderSubaccountIndex: 0,
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

`wrapInstructionWithFlight(...)` only wraps supported order placement
instructions. Non-order instructions are returned unchanged.

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

const wrappedIx = await flightClient.tryWrapFlightInstruction(
  nativeOrderIx,
  "Authority111111111111111111111111111111111"
);
```

This is the lower-level path if you are integrating Flight into an existing ix
builder stack rather than using `createPhoenixClient(...)`.
