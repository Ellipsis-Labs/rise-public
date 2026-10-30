# @ellipsis-labs/rise

TypeScript SDK for Phoenix perpetuals: HTTP route clients, exchange metadata,
instruction builders, Flight routing, and WebSocket streams.

- Unified client for HTTP, auth, session storage, and WebSocket streams
- Typed route clients (`candles`, `markets`, `funding`, `traders`, `trades`, and
  more)
- End-user and service-account auth with managed or external session control
- Exchange metadata cache with HTTP bootstrap, RPC fallback, and live stream
  updates
- Live store engines for market stats, trader state, and orderbooks
- Built-in WebSocket adapters plus typed custom subscription registration
- Low-level transport helpers (`get`, `post`, `put`, `patch`, `del`) for custom
  HTTP endpoints

## Install

```bash
npm install @ellipsis-labs/rise
```

## Changelog

Release notes:
[public Rise TypeScript changelog](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/CHANGELOG.md).

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

`createPhoenixClient(...)` returns:

- `client.api`: typed HTTP route clients
- `client.pda`: PDA/address helpers (memoized by default)
- `client.auth`: auth client (when `auth` is configured)
- `client.sessionManager`: shared auth session manager (when `auth` is
  configured)
- `client.streams`: typed WebSocket streams (unless `ws: false`)
- `client.exchange`: exchange metadata cache with API/RPC source management
- `client.orderPackets`: async order-packet builders backed by cached market
  metadata
- `client.ixs`: async instruction builders backed by cached exchange metadata
  and PDA resolution
- `client.rpc`: on-chain read surface for raw account-backed data

`client.pda` uses a bounded LRU cache (`pdaCache: { maxEntries: 1024 }`) by
default; pass `pdaCache: false` to disable memoization.

## Directory Structure

- `src/api/`: typed HTTP route clients (`candles`, `collateral`, `exchange`,
  `funding`, `invite`, `markets`, `notifications`, `orderbook`, `orders`,
  `splines`, `traders`, `trades`) and shared helpers in `api/utils/`
- `src/auth/`: auth client, session manager, storage adapters, runtime helpers,
  and retry/backoff logic
- `src/accounts/`: Phoenix and Flight account decoders and fetchers
- `src/core/`: low-level Phoenix instruction builders, account resolution, and
  shared client types
- `src/ixs.ts`: exchange-aware async instruction client built on `src/core/` and
  PDA derivation
- `src/orderPackets.ts`: synchronous order-packet builders from explicit market
  params
- `src/exchange-cache/`: API/RPC metadata bootstrap, selectors, store logic, and
  projected market views
- `src/flight/`: Flight account fetchers, proxy-instruction helpers,
  register-builder/update-fee builders, fee-collector derivation, and the
  routing guide in `src/flight/README.md`
- `src/ws/`: WebSocket transport, built-in adapters, and custom subscription
  registration
- `src/market-data/`, `src/orderbooks/`, `src/trader-state/`: live stores for
  exchange data and trader state
- `src/rpc.ts`, `src/pdas.ts`, `src/pdaClient.ts`: raw account reads and address
  derivation
- `src/primitives/`, `src/types/`, `src/margin/`: shared SDK value types, wire
  types, and margin helpers
- `src/builders.ts`, `src/flows.ts`: async helpers composing packet builders, ix
  builders, and deposit/withdraw/order flows
- `src/_exports/`: stable re-export bundles for schemas, SDK helpers,
  instruction builders, and public types
- `examples/`, `scripts/`, `tests/`: runnable examples, scripts, and tests

## Examples

Runnable reference scripts (full index in
[examples/README.md](./examples/README.md)):

- [examples/01-http-client.ts](./examples/01-http-client.ts): basic HTTP client
- [examples/02-ws-fills.ts](./examples/02-ws-fills.ts): WebSocket fills stream
- [examples/03-build-limit-order-ix.ts](./examples/03-build-limit-order-ix.ts):
  metadata-backed packet and ix construction
- [examples/04-trader-state-store.ts](./examples/04-trader-state-store.ts): live
  trader-state store wiring
- [examples/05-cancel-all-conditional-orders.ts](./examples/05-cancel-all-conditional-orders.ts):
  batch conditional-order cancellation
- [examples/06-flight-market-order.ts](./examples/06-flight-market-order.ts):
  Flight-routed market order with builder fee routing
- [examples/09-referral-activation-tx.ts](./examples/09-referral-activation-tx.ts):
  referral activation via `/v1/referral/activate-tx`
- [examples/10-builder-onboarding-tx.ts](./examples/10-builder-onboarding-tx.ts):
  builder onboarding without a referral code via
  `/v1/exchange/build-register-ixs` and `/v1/exchange/send-register-ixs`
  (fee-payer keypair defaults to `~/.config/solana/id.json`; trader authority
  defaults to the fee payer's public key):

  ```text
  bun examples/10-builder-onboarding-tx.ts \
    [--fee-payer-keypair-path <PATH>] \
    [--trader-authority <TRADER_AUTHORITY_PUBKEY>]
  ```

- [examples/phoenix-client-example.ts](./examples/phoenix-client-example.ts):
  HTTP API walkthrough
- [examples/phoenix-ws-example.ts](./examples/phoenix-ws-example.ts):
  interactive WebSocket demo

## Atomic isolated market entry with TP/SL v2

`client.api.orders().placeIsolatedMarketOrder()` and
`client.api.orders().placeIsolatedMarketOrderEnhanced()` accept
`greaterTrigger`, `lessTrigger`, and `sizePercent: 100`. Supply either trigger
or both. Both triggers share one position conditional and cancel together when
one executes. The backend allocates the isolated subaccount if needed and
includes collection creation in the same instruction bundle.

```ts
const instructions = await client.api.orders().placeIsolatedMarketOrder({
  authority,
  symbol: "SOL-PERP",
  side: "buy",
  quantity: 1,
  transferAmount: 10_000_000,
  sizePercent: 100,
  greaterTrigger: { side: "sell", triggerPrice: 180 },
  lessTrigger: { side: "sell", triggerPrice: 140 },
});
```

For short entries, use `side: "buy"` on the closing triggers; take profit is
then the less trigger and stop loss the greater trigger. Authority, fee-payer,
and Flight options apply to the whole bundle. Do not combine v2 fields with
legacy `tpSl`. Partial and fixed-lot protection are not supported on this path.
This request requires a backend release containing PRO-829.

## Recommended Client Setup

`createPhoenixClient()` requires no config and defaults `apiUrl` to
`https://perp-api.phoenix.trade`.

For long-running processes that use exchange-aware helpers:

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

To route supported placement instructions through Flight, configure `flight`
once on the client:

> **Use embedded wallets for Flight integrations.** Provision an embedded wallet
> per user rather than routing Flight orders through the user's raw/external
> wallet. A raw wallet shares its Phoenix trader state across every dapp the
> user trades on, causing positions, balances, and orders to bleed across
> integrations. Embedded wallets isolate per-integration state.

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

Configuration notes:

- Use `apiUrl` (`baseUrl` is a deprecated alias) and keep PDA memoization
  enabled for long-lived processes.
- Set `exchangeMetadata: { stream: true }` when using `client.exchange`,
  `client.orderPackets`, or `client.ixs` over time so WebSocket deltas apply
  after snapshot bootstrap.
- Set `rpcUrl` (or `NEXT_PUBLIC_SOLANA_RPC_URL` / `SOLANA_RPC_URL`) to enable
  `client.rpc` and RPC fallback when the API metadata snapshot is unavailable.
- Keep the default rate-limit cooldown enabled for long-lived clients. When a
  retryable GET or HEAD returns `429` with `Retry-After`, subsequent retryable
  requests on the same client wait for that window (capped at 30 seconds by
  default, with positive release jitter). The cooldown is client-local and never
  delays non-idempotent methods like POST.
- Configure Flight with `builderAuthority` and optional `builderPdaIndex` /
  `builderSubaccountIndex` (both default to `0`). Set `feeBpsOverride` only when
  local order builders should emit Flight's fee-override proxy instruction. See
  [src/flight/README.md](./src/flight/README.md) for full Flight routing
  details.

Tune or disable the rate-limit cooldown:

```ts
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  rateLimitCooldown: {
    maxDelayMs: 30_000,
    fallbackDelayMs: 1_000,
  },
});

const cooldownDisabledClient = createPhoenixClient({
  rateLimitCooldown: false,
});
```

Starter examples: [examples/01-http-client.ts](./examples/01-http-client.ts),
[examples/02-ws-fills.ts](./examples/02-ws-fills.ts),
[examples/03-build-limit-order-ix.ts](./examples/03-build-limit-order-ix.ts),
[examples/04-trader-state-store.ts](./examples/04-trader-state-store.ts), and
[examples/README.md](./examples/README.md).

## Quick Start

### Public HTTP only

```ts
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient();

const exchange = await client.api.exchange().getExchange();
const metadata = await client.exchange.ready();
const markets = await client.api.markets().getMarkets();
const now = Date.now();
const candlesPage = await client.api.candles().getCandlesV2("SOL", {
  timeframe: "1m",
  from: now - 60 * 60_000,
  to: now,
  limit: 10_000,
  includePartial: false,
});
const trader = await client.api.traders().getTrader("TRADER_PUBKEY");
```

Use `getCandlesV2` for canonical v2 candle data: it returns millisecond
timestamps, cursor pagination (default 1,000 bars per page, max 10,000),
partial-bar control, mark-price OHLC, external-source metadata, and `isFinal`.
`getCandles` stays on the legacy endpoint to preserve its millisecond-timestamp
array response, exchange-only default, server-relative finalized window, and
2,500-bar limit.

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

`client.streams` uses a lazy WebSocket transport derived from `apiUrl`
(`https://perp-api.phoenix.trade` by default) with a 30-second idle close. Pass
`ws: false` to disable it. `client.exchange` bootstraps from the HTTP snapshot
on first use even when `client.streams` is available, and falls back to RPC when
an RPC URL is configured and the API snapshot is unavailable; set
`exchangeMetadata: { stream: true }` to apply live WebSocket deltas afterward.

Stateful live stores are constructed explicitly:

- `createPhoenixMarketData({ exchange, marketStats, allMids? })`
- `createPhoenixTraderStateManager({ api, traderState? })`
- `createTraderStateStore(client, config?)`
- `createPhoenixOrderbookManager({ api, l2Book? })`

See [examples/04-trader-state-store.ts](./examples/04-trader-state-store.ts) for
a `client.streams.traderState(...)` store example. In React, use leaf selectors
against `resource.store` and avoid allocating new wrapper objects inside
selectors.

## Exchange Metadata

`client.exchange` manages exchange and market metadata for packet and
instruction builders:

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

- Lazy startup: nothing is fetched until `client.exchange` is used
- API-first bootstrap from `/v1/exchange/snapshot`, with RPC fallback when an
  RPC URL is configured (`rpcUrl`, `NEXT_PUBLIC_SOLANA_RPC_URL`, or
  `SOLANA_RPC_URL`, which also enables `client.rpc`)
- Exchange WebSocket live updates when `ws` is configured

See [src/flight/README.md](./src/flight/README.md) for Flight order-routing
examples.

### Collateral asset registry

`client.exchange.spotCollaterals()` (and `client.api.collateral().getAssets()`)
returns every registered collateral asset, including inactive ones:

- An asset accepts deposits only when `spot` is non-null and `spot.isActive` is
  true. The quote asset (USDC) has no `spot`.
- `spot.mint` is the SPL mint; native SOL has no mint. The USDC mint is
  `exchange.usdcMint` in exchange state, not a registry field.
- A `spotCollateralsUpdated` exchange delta carries the complete registry and
  replaces the previous list. The cache emits an `exchangeUpdated` event with
  `change: "spotCollaterals"`.
- A trader-state `spotCollaterals[].maxBalance` is that trader's effective cap;
  the exchange-wide cap (`maxGlobalBalance - currGlobalBalance`) may bind first.

The `exchange` stream decodes the server's default `base64+zstd` snapshots
transparently. Raw WebSocket clients must decode them or subscribe with
`"encoding": "json"`.

## Flame Deposit Funding

Use `buildFlameDepositFundingFlow` for sponsored USDC deposits that avoid
creating user-owned rent accounts:

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
USDC into it from the user's wallet ATA; the Flame crank/indexer completes the
Phoenix collateral deposit asynchronously.

For an atomic sponsored deposit, use `buildFlameAtomicDepositFlow`, which adds
permission setup and Flame `DepositToPhoenix`. The user signs the deposit
permission while the sponsor fee payer cranks the deposit and fronts transient
proxy ATA rent (refunded on close in the same instruction), allowing zero-SOL
wallets to deposit.

Use `buildDepositFlow` for direct Ember + Phoenix deposits.

## Sync Order Packet Builders

When you already have market params (`tickSize`, `baseLotsDecimals`), build
order packets synchronously without a client:

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

To resolve market params from the exchange metadata cache instead, use
`client.ixs.orderPackets.*`:

```ts
const packet = await client.ixs.orderPackets.buildMarketOrderPacket({
  symbol: "SOL-PERP",
  side: Side.Bid,
  baseUnits: "0.50",
  priceLimitUsd: "136.10",
});
```

## Sync Resolved Ix Builders

When exchange, market, and trader accounts are already resolved, build
instructions synchronously:

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

The resolved surface also includes deposit, withdraw, transfer, and market-order
builders; `client.ixs.*` is the async wrapper over it.

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

Enable `auth` to attach bearer tokens, refresh sessions, and authenticate
WebSocket subscriptions.

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

Use `sessionControl: "external"` when your app owns the session lifecycle and
`rise` should only read the current session. `rise` will not refresh, clear, or
mutate session state automatically, though explicitly importing or providing a
session through your `sessionManager` still writes to its backing storage.

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

With `auth: true`, `rise` hydrates from storage without forcing an eager login.
Built-in route clients attach the shared session opportunistically; custom
transport calls can override this via `RequestOptions.auth`.

### React Native package entry

Resolvers with the `react-native` condition enabled select
`dist/native/index.js`, which exposes the same public API and types as the
default Node/browser entry without a Node filesystem loader. The host runtime
must provide UTF-8 encoding, randomness, and the cryptographic capabilities SDK
operations require.

Service-account credential files are unsupported in the native entry (whereas
the default entry lazily loads `fs/promises`). Calling
`loadServiceAccountCredentialFromPath` or setting
`PHOENIX_SERVICE_ACCOUNT_CREDENTIAL` rejects with `PhoenixAuthError` code
`service_account_credential_file_unavailable` and does not fall back to split
environment credentials, because a configured file path takes precedence.
Explicit credentials and split environment variables work identically to the
default entry.

### Service-account sessions

Server-side tools can authenticate with the same service-account credential env
vars supported by Rise Rust:

- `PHOENIX_SERVICE_ACCOUNT_CREDENTIAL`: path to a JSON credential with
  `client_id`, `key_id`, and `private_key`
- `PHOENIX_SERVICE_ACCOUNT_CLIENT_ID`
- `PHOENIX_SERVICE_ACCOUNT_KEY_ID`
- `PHOENIX_SERVICE_ACCOUNT_PRIVATE_KEY`

The split vars also accept the legacy `PHOENIX_SERVICE_CLIENT_ID`,
`PHOENIX_SERVICE_KEY_ID`, and `PHOENIX_SERVICE_PRIVATE_KEY` aliases.

```ts
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  auth: true,
});

await client.auth!.loginWithServiceAccountFromEnv();
```

## WebSocket Streams

Construct a standalone WebSocket client when you only need streams:

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

To reuse an existing socket that implements `WsClient`, wrap it with
`createPhoenixWsFacade(...)`:

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

Register typed custom channels on `createPhoenixWsClient(...)` or via the
top-level `registerSubscription(...)` / `registerSubscriptions(...)` helpers:

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

`PhoenixHttpClient` implements `HttpTransport`, so you can call custom routes
with `get`, `post`, `put`, `patch`, and `del`:

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
