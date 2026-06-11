# @ellipsis-labs/rise

`rise` is the TypeScript SDK for Phoenix public APIs, exchange metadata,
instruction building, Flight routing, and live streams.

It gives you:

- a unified client for HTTP, auth, session storage, and WebSocket streams
- typed route clients for candles, markets, funding, traders, trades, and more
- optional end-user auth with managed or externally controlled sessions
- an exchange metadata cache for bootstrapping market parameters
- reusable store engines for market stats, trader state, and orderbooks
- built-in WebSocket adapters plus typed custom subscription registration
- low-level transport helpers for custom HTTP endpoints

## Main Entry Points

```ts
import {
  auth,
  createPhoenixClient,
  createPhoenixWsClient,
  get,
  registerSubscription,
} from "@ellipsis-labs/rise";
```

The most common constructor is `createPhoenixClient(...)`, which returns:

- `client.api`: typed HTTP route clients
- `client.pda`: PDA/address helper surface with memoized derivations by default
- `client.auth`: auth client, only when auth is configured
- `client.sessionManager`: shared auth session manager, only when auth is configured
- `client.streams`: typed WebSocket streams, unless `ws: false` is configured
- `client.exchange`: client-owned exchange metadata cache with API/RPC source management
- `client.orderPackets`: async order-packet builders backed by cached market metadata
- `client.ixs`: async instruction builders backed by cached exchange metadata and PDA resolution
- `client.rpc`: explicit on-chain read surface for raw account-backed data

Pass `pdaCache: false` to `createPhoenixClient(...)` if you need to disable PDA
memoization on `client.pda`, or `pdaCache: { maxEntries: 1024 }` if you want to
spell out the default bounded LRU cache for long-running processes.

## Directory Structure

- `src/api/`: typed HTTP route clients for `candles`, `collateral`,
  `exchange`, `funding`, `invite`, `markets`, `notifications`, `orderbook`,
  `orders`, `splines`, `traders`, and `trades`, plus shared helpers in
  `api/utils/`
- `src/auth/`: auth client, session manager, storage adapters, runtime helpers,
  and retry/backoff logic
- `src/accounts/`: Phoenix and Flight account decoders plus reusable fetchers
- `src/core/`: low-level Phoenix instruction builders, account resolution
  helpers, and shared client types
- `src/ixs.ts`: exchange-aware async instruction client built on `src/core/`
  and PDA derivation
- `src/orderPackets.ts`: synchronous order-packet builders from explicit market
  params
- `src/exchange-cache/`: API/RPC metadata bootstrap, selectors, store logic,
  and projected market views
- `src/flight/`: Flight account fetchers, proxy-instruction helpers,
  register-builder/update-fee builders, fee-collector derivation, and the
  Flight routing guide in `src/flight/README.md`
- `src/ws/`: websocket transport, built-in adapters, and custom subscription
  registration
- `src/market-data/`, `src/orderbooks/`, `src/trader-state/`: long-lived live
  stores for exchange data and trader state
- `src/rpc.ts`, `src/pdas.ts`, and `src/pdaClient.ts`: raw account reads and
  address-derivation helpers
- `src/primitives/`, `src/types/`, and `src/margin/`: shared SDK value types,
  wire types, and margin helpers
- `src/builders.ts` and `src/flows.ts`: higher-level async helpers that compose
  packet builders, ix builders, and common deposit/withdraw/order flows
- `src/_exports/`: curated stable re-export bundles for schemas, SDK helpers,
  instruction builders, and public types
- `examples/`, `scripts/`, and `tests/`: runnable examples, task-specific
  scripts, and test coverage

## Examples

Use the examples as the fastest reference for intended SDK usage:

- [examples/README.md](./examples/README.md): entry point for the runnable TS
  examples
- [examples/01-http-client.ts](./examples/01-http-client.ts): basic HTTP client
- [examples/02-ws-fills.ts](./examples/02-ws-fills.ts): websocket fills stream
- [examples/03-build-limit-order-ix.ts](./examples/03-build-limit-order-ix.ts):
  metadata-backed packet and ix construction
- [examples/04-trader-state-store.ts](./examples/04-trader-state-store.ts):
  live trader-state store wiring
- [examples/05-cancel-all-conditional-orders.ts](./examples/05-cancel-all-conditional-orders.ts):
  advanced conditional-order cancellation workflow
- [examples/06-flight-market-order.ts](./examples/06-flight-market-order.ts):
  Flight-routed market order placement with builder fee routing
- [examples/phoenix-client-example.ts](./examples/phoenix-client-example.ts):
  broader API walkthrough
- [examples/phoenix-ws-example.ts](./examples/phoenix-ws-example.ts):
  interactive websocket demo

## Recommended Client Setup

`createPhoenixClient()` works with no config and defaults `apiUrl` to
`https://perp-api.phoenix.trade`.

For a long-running app that uses exchange-aware helpers, start with:

```ts
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  pdaCache: { maxEntries: 1024 },
  exchangeMetadata: {
    stream: true,
  },
});
```

If you route supported placement flows through Flight, configure it once on the
client:

> **Use embedded wallets for Flight integrations.** Integrators are strongly
> encouraged to provision an embedded wallet per user rather than routing
> Flight orders through the user's raw/external wallet. A raw wallet shares
> its Phoenix trader state with every other dapp the user trades on, which
> results in bad UX (positions, balances, and orders bleeding across
> integrations). Embedded wallets isolate per-integration state.

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

Recommended defaults:

- Prefer `apiUrl`; `baseUrl` is a deprecated alias.
- Keep PDA memoization enabled for long-lived processes.
- Enable `exchangeMetadata: { stream: true }` when you use `client.exchange`,
  `client.orderPackets`, or `client.ixs` over time and want websocket deltas
  applied after the initial snapshot bootstrap.
- Set `rpcUrl` when you use `client.rpc` or want RPC-backed exchange metadata
  fallback if the API snapshot is unavailable. If you omit it, rise will also
  read `NEXT_PUBLIC_SOLANA_RPC_URL` or `SOLANA_RPC_URL`.
- If you use Flight consistently, configure it once on the client with
  `builderAuthority`, plus optional `builderPdaIndex` and
  `builderSubaccountIndex`. rise derives the builder trader account from those
  values and defaults both indexes to `0`.

For short runnable onboarding examples, see:

- [examples/01-http-client.ts](./examples/01-http-client.ts)
- [examples/02-ws-fills.ts](./examples/02-ws-fills.ts)
- [examples/03-build-limit-order-ix.ts](./examples/03-build-limit-order-ix.ts)
- [examples/04-trader-state-store.ts](./examples/04-trader-state-store.ts)
- [examples/README.md](./examples/README.md)

## Quick Start

### Public HTTP only

```ts
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient();

const exchange = await client.api.exchange().getExchange();
const metadata = await client.exchange.ready();
const markets = await client.api.markets().getMarkets();
const candles = await client.api.candles().getCandles("SOL");
const trader = await client.api.traders().getTrader("TRADER_PUBKEY");
```

### Unified HTTP + auth + WebSocket client

```ts
import { auth, createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  auth: true,
  authConfig: {
    storage: new auth.LocalStorageAuthSessionStorage(),
  },
  exchangeMetadata: {
    stream: true,
  },
});

await client.auth!.loginWithPrivyToken(privyToken);

const notifications = await client.api
  .notifications()
  .getNotificationsByTrader("TRADER_PUBKEY");

for await (const update of client.streams!.marketStats("SOL")) {
  console.log(update);
  break;
}

const solMarket = client.exchange.market("SOL-PERP");
```

By default `rise` creates a lazy websocket transport for `client.streams`.
Pass `ws: false` to disable it. When the owned transport needs a URL, it
derives it from `apiUrl`, which defaults to
`https://perp-api.phoenix.trade`. In lazy mode, the owned transport uses a
30-second idle close by default.

`client.exchange` still bootstraps from the HTTP snapshot on first use even
when `client.streams` is available. Opt into exchange websocket follow-up with
`exchangeMetadata: { stream: true }` when you want live exchange deltas applied
after bootstrap.

The heavier live stores stay explicit:

- `createPhoenixMarketData({ exchange, marketStats, allMids? })`
- `createPhoenixTraderStateManager({ api, traderState? })`
- `createTraderStateStore(client, config?)`
- `createPhoenixOrderbookManager({ api, l2Book? })`

See [examples/04-trader-state-store.ts](./examples/04-trader-state-store.ts)
for a runnable trader-state store example backed by
`client.streams.traderState(...)`.

Prefer leaf selectors against `resource.store` in React. Avoid selectors that
allocate a fresh wrapper object on every read.

## Exchange Metadata

`rise` now treats exchange metadata as a first-class client subsystem.

```ts
const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  exchangeMetadata: {
    stream: true,
  },
});

await client.exchange.ready();

const context = client.exchange.instructionContext("SOL-PERP");
const source = client.exchange.source();
const packet = await client.ixs.orderPackets.buildLimitOrderPacket({
  symbol: "SOL-PERP",
  side: Side.Bid,
  priceUsd: "135.87",
  baseUnits: "0.25",
});
const ix = await client.ixs.placeLimitOrder({
  authority,
  symbol: "SOL-PERP",
  orderPacket: packet,
});

client.exchange.subscribe((event) => {
  if (event.type === "marketAdded") {
    console.log("new market", event.symbol);
  }
});
```

Default behavior:

- API-first metadata bootstrap from `/v1/exchange/snapshot`
- exchange websocket live updates when `ws` is configured
- `rpcUrl` enables `client.rpc`
- if `rpcUrl` is omitted, rise also checks `NEXT_PUBLIC_SOLANA_RPC_URL` and
  `SOLANA_RPC_URL`
- when an RPC URL is available, exchange metadata can fall back to RPC even in
  the default API-first mode
- lazy startup: no metadata fetch happens until `client.exchange` is used

For Flight-specific order-routing examples, see
[src/flight/README.md](./src/flight/README.md).

## Flame Deposit Funding

For sponsored USDC deposits that should avoid creating user-owned rent accounts,
build a Flame proxy-funding transaction instead of the direct deposit flow:

```ts
import { buildFlameDepositFundingFlow } from "@ellipsis-labs/rise";

const deposit = await buildFlameDepositFundingFlow(
  {
    authority,
    amount: 1_000_000n,
    traderPdaIndex: 0,
    feePayer,
    sponsorshipToken,
  },
  ixClient
);
```

The returned instructions create the user's Flame proxy USDC ATA and transfer
USDC from the user's wallet ATA into that proxy ATA. They do not perform the
Phoenix collateral deposit directly; the Flame crank/indexer completes that
asynchronously after the proxy account is funded.

`buildDepositFlow` remains the direct Ember + Phoenix deposit path.

## Sync Order Packet Builders

If you already have market units in hand, `rise` also exposes synchronous packet
builders that do not need the client:

```ts
import {
  buildLimitOrderPacketFromMarketParams,
  buildMarketOrderPacketFromMarketParams,
  Side,
} from "@ellipsis-labs/rise";

const limitPacket = buildLimitOrderPacketFromMarketParams(
  {
    side: Side.Bid,
    priceUsd: "135.87",
    baseUnits: "0.25",
  },
  {
    tickSize: 100,
    baseLotsDecimals: 2,
  }
);

const marketPacket = buildMarketOrderPacketFromMarketParams(
  {
    side: Side.Ask,
    baseUnits: "0.50",
    priceLimitUsd: "135.80",
  },
  {
    tickSize: 100,
    baseLotsDecimals: 2,
  }
);
```

If you want the client to source market params from the exchange metadata cache,
use `client.ixs.orderPackets.*` instead:

```ts
const packet = await client.ixs.orderPackets.buildMarketOrderPacket({
  symbol: "SOL-PERP",
  side: Side.Bid,
  baseUnits: "0.50",
  priceLimitUsd: "136.10",
});
```

## Sync Resolved Ix Builders

If you already have resolved exchange, market, and trader accounts in hand,
`rise` also exposes synchronous resolved ix builders:

```ts
import { buildPlaceLimitOrderIxResolved } from "@ellipsis-labs/rise";

const ix = buildPlaceLimitOrderIxResolved({
  exchange: {
    phoenixProgramAddress,
    logAuthorityAddress,
    globalConfigurationAddress,
    perpAssetMap,
    globalTraderIndex,
    activeTraderBuffer,
  },
  market: {
    marketAddress,
    splineCollection,
  },
  trader: {
    authority,
    traderAccount,
  },
  orderPacket,
});
```

This is the intended non-async SDK layer. The same resolved surface also
includes deposit, withdraw, transfer, and market-order builders. `client.ixs.*`
is the async convenience layer on top of it.

## Common HTTP Usage

Route clients are grouped under `client.api`:

```ts
const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
});

const exchange = await client.api.exchange().getExchange();
const market = await client.api.markets().getMarket("SOL");
const traderState = await client.api
  .traders()
  .getTraderStateSnapshot("AUTHORITY");
const fills = await client.api.trades().getMarketFills("SOL");
const funding = await client.api.funding().getFundingRateHistory("SOL");
```

Available groups include:

- `candles()`
- `collateral()`
- `exchange()`
- `funding()`
- `invite()`
- `markets()`
- `notifications()`
- `orders()`
- `traders()`
- `trades()`

## Authentication

Configure `auth` when you want the client to attach bearer tokens, refresh
sessions, or authenticate WebSocket subscriptions.

### Browser-managed sessions

```ts
import { auth, createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  auth: true,
  authConfig: {
    storage: new auth.LocalStorageAuthSessionStorage(),
  },
});

await client.auth!.loginWithPrivyToken(privyToken);

const snapshot = await client.sessionManager!.exportSnapshot();
console.log(snapshot);
```

### External session control

Use this when your app owns the session lifecycle and only wants `rise` to read
the current session. In this mode `rise` will not automatically refresh,
clear, or otherwise mutate session state on your behalf. If you explicitly
provide or import a session through your configured session manager, that
manager may still write the session to its backing storage.

```ts
import { auth, createPhoenixClient } from "@ellipsis-labs/rise";

const sessionManager = new auth.AuthSessionManager(
  new auth.MemoryAuthSessionStorage()
);

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  auth: true,
  authConfig: {
    sessionManager,
    sessionControl: "external",
  },
});

await sessionManager.importSnapshot(snapshotFromYourApp);
```

When `auth: true`, `rise` hydrates from the configured storage automatically.
It does not force an eager login flow. Built-in route clients use the shared
session opportunistically when one exists. For custom transport calls, use
`RequestOptions.auth` to disable auth or require it explicitly.

## WebSocket Streams

If you only need streams, you can construct the WS client directly:

```ts
import { createPhoenixWsClient } from "@ellipsis-labs/rise";

const streams = createPhoenixWsClient({
  url: "wss://perp-api.phoenix.trade/v1/ws",
  authMode: "anonymous",
});

for await (const update of streams.orderbook("SOL")) {
  console.log(update);
  break;
}

streams.close();
```

Built-in stream adapters include:

- `allMids`
- `candles`
- `exchange`
- `exchangeStatus`
- `fills`
- `fundingRate`
- `l2Book`
- `markPrice`
- `market`
- `marketStats`
- `notifications`
- `orderbook`
- `orderbookSnapshot`
- `traderState`

If you already own a raw websocket transport that implements the exported
`WsClient` interface, use `createPhoenixWsFacade(...)` instead of creating
another socket:

```ts
import { createPhoenixWsFacade, type WsClient } from "@ellipsis-labs/rise";

declare const sharedWs: WsClient;

const riseStreams = createPhoenixWsFacade({
  wsClient: sharedWs,
});

for await (const update of riseStreams.exchange()) {
  console.log(update);
  break;
}
```

## Custom WebSocket Subscriptions

`rise` exposes typed custom subscription registration on both
`createPhoenixWsClient(...)` and the top-level `registerSubscription(...)` /
`registerSubscriptions(...)` helpers.

```ts
import { createPhoenixWsClient } from "@ellipsis-labs/rise";
import z from "zod";

const streams = createPhoenixWsClient({
  url: "wss://perp-api.phoenix.trade/v1/ws",
  authMode: "anonymous",
});

const customPrices = streams.registerSubscription({
  subscriptionChannel: "custom-prices",
  schema: z.object({
    channel: z.literal("custom-prices"),
    symbol: z.string(),
    price: z.number(),
  }),
  buildKey: (symbol: string) => symbol,
  buildSubParams: (symbol: string) => ({ symbol }),
  getKeyFromMessage: (message) => message.symbol,
  processMessage: (message) => message.price,
});

for await (const price of customPrices("SOL")) {
  console.log(price);
  break;
}
```

## Custom HTTP Endpoints

`PhoenixHttpClient` implements the exported `HttpTransport` interface, so you
can reuse the built-in `get` / `post` / `put` / `patch` / `del` helpers for
custom endpoints.

```ts
import { createPhoenixClient, get } from "@ellipsis-labs/rise";
import z from "zod";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
});

const HealthSchema = z.object({
  ok: z.boolean(),
});

const health = await get(client.api, "/internal/health", HealthSchema, {
  auth: "disabled",
});
```

This is the cleanest way to add a small custom route without building another
wrapper client first. The same pattern works with `post`, `put`, `patch`, and
`del`.

## Errors

The main error types are:

- `PhoenixHttpError`: HTTP status, code, retry-after, and parsed response body
- `PhoenixAuthError`: auth/login/refresh/bootstrap failures

```ts
import {
  PhoenixAuthError,
  PhoenixHttpError,
  createPhoenixClient,
} from "@ellipsis-labs/rise";

try {
  const client = createPhoenixClient({
    apiUrl: "https://perp-api.phoenix.trade",
  });

  await client.api.markets().getMarkets();
} catch (error) {
  if (error instanceof PhoenixAuthError) {
    console.error("Auth failed", error.code);
  } else if (error instanceof PhoenixHttpError) {
    console.error("HTTP failed", error.status, error.code);
  } else {
    throw error;
  }
}
```

## Development

```bash
bun install
bun run build
bun run lint
bun run type-check
bun run test
```
