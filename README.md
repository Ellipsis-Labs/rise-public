# Rise SDK

Rise is the developer-facing SDK surface for Phoenix perpetuals. It currently
ships as:

- `rise/ts`: the TypeScript SDK, with HTTP route clients, a unified
  exchange-aware client, instruction builders, Flight helpers, and WebSocket
  adapters
- `rise/rust`: the Rust workspace, centered on the `phoenix-rise` crate, with
  typed HTTP/WS clients, transaction builders, math helpers, and low-level
  instruction builders

If you are deciding where to start:

- Reach for `PhoenixHttpClient` / `client.api` when you need public HTTP data or
  invite activation
- Reach for `createPhoenixClient(...)` when you want exchange metadata, PDA
  derivation, order-packet helpers, and `client.ixs`
- Reach for `client.streams` or `createPhoenixWsClient(...)` when you want
  typed live adapters
- Reach for `PhoenixTxBuilder` in Rust when you want to construct and sign your
  own Solana instructions

## Runnable Entry Points

TypeScript:

- [rise/ts/examples/01-http-client.ts](./ts/examples/01-http-client.ts)
- [rise/ts/examples/02-ws-fills.ts](./ts/examples/02-ws-fills.ts)
- [rise/ts/examples/03-build-limit-order-ix.ts](./ts/examples/03-build-limit-order-ix.ts)
- [rise/ts/examples/04-trader-state-store.ts](./ts/examples/04-trader-state-store.ts)
- [rise/ts/examples/05-cancel-all-conditional-orders.ts](./ts/examples/05-cancel-all-conditional-orders.ts)
- [rise/ts/examples/06-flight-market-order.ts](./ts/examples/06-flight-market-order.ts)
- [rise/ts/examples/09-referral-activation-tx.ts](./ts/examples/09-referral-activation-tx.ts)
- [rise/ts/examples/10-builder-onboarding-tx.ts](./ts/examples/10-builder-onboarding-tx.ts)
- [rise/ts/examples/phoenix-client-example.ts](./ts/examples/phoenix-client-example.ts)
- [rise/ts/examples/phoenix-ws-example.ts](./ts/examples/phoenix-ws-example.ts)
- [rise/ts/README.md](./ts/README.md)
- [rise/ts/examples/README.md](./ts/examples/README.md)

Rust:

- [rise/rust/README.md](./rust/README.md)
- [rise/rust/examples/register_trader.rs](./rust/examples/register_trader.rs)
- [rise/rust/examples/http_client.rs](./rust/examples/http_client.rs)
- [rise/rust/examples/send_limit_order.rs](./rust/examples/send_limit_order.rs)
- [rise/rust/examples/send_market_order.rs](./rust/examples/send_market_order.rs)
- [rise/rust/examples/send_flight_market_order.rs](./rust/examples/send_flight_market_order.rs)
- [rise/rust/examples/referral_activation_tx.rs](./rust/examples/referral_activation_tx.rs)
- [rise/rust/examples/builder_onboarding_tx.rs](./rust/examples/builder_onboarding_tx.rs)

## Onboarding Paths

These onboarding routes are not interchangeable:

- Use `POST /v1/invite/activate` when you have an access code / allowlist code.
  Send that value as `code`.
- Use `POST /v1/referral/activate-tx` when you have a referral code and want
  delegated onboarding. The referral code is required, and the trader authority
  must sign the transaction.
- Use `POST /v1/exchange/build-register-ixs` followed by
  `POST /v1/exchange/send-register-ixs` when a builder wants to register and
  onboard a trader without a referral code. The builder chooses the transaction
  fee payer, and the API signs only after validating and simulating the
  submitted transaction.

For copyable Rust and TypeScript examples of both delegated onboarding paths,
see [sdk/delegated-onboarding.mdx](../sdk/delegated-onboarding.mdx). Runnable
examples are also available in
[09-referral-activation-tx.ts](./ts/examples/09-referral-activation-tx.ts),
[10-builder-onboarding-tx.ts](./ts/examples/10-builder-onboarding-tx.ts),
[referral_activation_tx.rs](./rust/examples/referral_activation_tx.rs), and
[builder_onboarding_tx.rs](./rust/examples/builder_onboarding_tx.rs).

The access-code route remains simpler because the user already has an allowlist
code:

TypeScript:

```ts
import { PhoenixHttpClient } from "@ellipsis-labs/rise";

const client = new PhoenixHttpClient({
  apiUrl: "https://perp-api.phoenix.trade",
  auth: true,
});

const authority = "AUTHORITY_PUBKEY";

const activatedWithAccessCode = await client.invite().activateInvite({
  authority,
  code: "ACCESS_CODE",
});
```

Rust:

```rust
use phoenix_rise::PhoenixHttpClient;
use solana_pubkey::Pubkey;
use std::str::FromStr;

let client = PhoenixHttpClient::new_from_env_with_auth()?;
let authority = Pubkey::from_str("AUTHORITY_PUBKEY")?;

let trader_from_access = client
    .invite()
    .activate_invite(&authority, "ACCESS_CODE")
    .await?;
```

Use
[register_trader.rs](./rust/examples/register_trader.rs)
when you want a ready-to-run Rust access-code example.

## Fetching Exchange, Market, and Trader State

The HTTP surface is intentionally split by what kind of state you want:

- `exchange().getSnapshot()`: exchange-wide state plus every market's current
  config snapshot
- `exchange().getMarket(symbol)`: one market's fees, risk, funding cadence, and
  configuration
- `orderbook().getOrderbook(symbol)`: an HTTP L2 snapshot for one market
- `traders().getTraderStateSnapshot(...)` in TypeScript: a trader-centric view
  of collateral, positions, orders, and triggers
- `markets().getMarketStatsHistory(...)` and `funding().getFundingRateHistory(...)`:
  time-series data for frontends, vault products, and analytics

TypeScript:

```ts
import { PhoenixHttpClient } from "@ellipsis-labs/rise";

const client = new PhoenixHttpClient({
  apiUrl: "https://perp-api.phoenix.trade",
});

const symbol = "SOL";
const authority = "AUTHORITY_PUBKEY";

const [snapshot, market, orderbook, trader] = await Promise.all([
  client.exchange().getSnapshot(),
  client.exchange().getMarket(symbol),
  client.orderbook().getOrderbook(symbol),
  client.traders().getTraderStateSnapshot(authority, { traderPdaIndex: 0 }),
]);
```

Rust:

```rust
use phoenix_rise::PhoenixHttpClient;
use solana_pubkey::Pubkey;
use std::str::FromStr;

let client = PhoenixHttpClient::new_from_env()?;
let authority = Pubkey::from_str("AUTHORITY_PUBKEY")?;

let snapshot = client.exchange().get_snapshot().await?;
let market = client.markets().get_market("SOL").await?;
let pnl = client.traders().get_trader_pnl(&authority, Default::default()).await?;
```

For broader TypeScript walkthroughs, see
[01-http-client.ts](./ts/examples/01-http-client.ts)
and
[phoenix-client-example.ts](./ts/examples/phoenix-client-example.ts).

## Order Placement and Cancellation

The SDK intentionally separates packet construction from instruction
construction:

- build packet sizes and prices with `client.orderPackets`
- build or wrap the actual Solana instructions with `client.ixs`
- use the lower-level builders when you need conditional-account setup or other
  specialized flows

TypeScript:

```ts
import {
  Direction,
  Side,
  StopLossOrderKind,
  createPhoenixClient,
} from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  ws: false,
  exchangeMetadata: { stream: false },
});

const authority = "AUTHORITY_PUBKEY";
const symbol = "SOL-PERP";

const limitPacket = await client.orderPackets.buildLimitOrderPacket({
  symbol,
  side: Side.Bid,
  priceUsd: "150.50",
  baseUnits: "0.25",
});

const placeLimitIx = await client.ixs.placeLimitOrder({
  authority,
  symbol,
  orderPacket: limitPacket,
});

const marketPacket = await client.orderPackets.buildMarketOrderPacket({
  symbol,
  side: Side.Bid,
  baseUnits: "0.25",
});

const placeMarketIx = await client.ixs.placeMarketOrder({
  authority,
  symbol,
  orderPacket: marketPacket,
});

const stopLossIx = await client.ixs.buildPlaceStopLoss({
  authority,
  symbol,
  tradeSide: Side.Ask,
  executionDirection: Direction.LessThan,
  orderKind: StopLossOrderKind.IOC,
  triggerPrice: 1420n,
});

const cancelByIdIx = await client.ixs.buildCancelOrdersById({
  authority,
  symbol,
  orders: [{ price: 1500n, orderSequenceNumber: "123" }],
});

const cancelAllIx = await client.ixs.buildCancelAll({ authority, symbol });

const cancelStopLossIx = await client.ixs.buildCancelStopLoss({
  authority,
  symbol,
  executionDirection: Direction.LessThan,
});
```

`buildPlaceStopLoss(...)` takes tick-based trigger prices. When you are starting
from USD prices, convert them from market metadata first, or reuse the
conditional-order patterns in
[05-cancel-all-conditional-orders.ts](./ts/examples/05-cancel-all-conditional-orders.ts).

Rust:

```rust
use phoenix_rise::{
    BracketLeg, BracketLegOrders, BracketLegTicket, CancelId, LimitOrderTicket,
    MarketOrderTicket, PhoenixTxBuilder, Side, TraderKey,
};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

let trader = TraderKey::new(authority);
let builder = PhoenixTxBuilder::new(&metadata);

let limit_ixs = builder
    .place_limit_order(
        LimitOrderTicket::builder()
            .authority(trader.authority())
            .trader_account(trader.pda())
            .symbol("SOL-PERP")
            .side(Side::Bid)
            .price(150.50)
            .num_base_lots(25_000)
            .build()?,
    )
    .await?;

let market_ixs = builder
    .place_market_order(
        MarketOrderTicket::builder()
            .authority(trader.authority())
            .trader_account(trader.pda())
            .symbol("SOL-PERP")
            .side(Side::Bid)
            .num_base_lots(25_000)
            .build()?,
    )
    .await?;

let bracket_ixs = builder
    .place_position_bracket_order(
        trader.authority(),
        trader.pda(),
        "SOL-PERP",
        Side::Bid,
        BracketLegTicket::new(
            Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com".into())),
            BracketLegOrders {
                stop_loss: Some(BracketLeg::new(142.0)),
                take_profit: None,
            },
        ),
    )
    .await?;

let cancel_ixs = builder.build_cancel_orders(
    trader.authority(),
    trader.pda(),
    "SOL-PERP",
    vec![CancelId::new(1500, 123)],
)?;
```

Runnable examples:

- [03-build-limit-order-ix.ts](./ts/examples/03-build-limit-order-ix.ts)
- [05-cancel-all-conditional-orders.ts](./ts/examples/05-cancel-all-conditional-orders.ts)
- [send_limit_order.rs](./rust/examples/send_limit_order.rs)
- [send_market_order.rs](./rust/examples/send_market_order.rs)
- [cancel_order.rs](./rust/examples/cancel_order.rs)

## Flight Builder Activation and Routed Orders

Flight is the builder-routing layer. The important pieces are:

- the builder still needs a Phoenix trader account
- builder registration is its own on-chain instruction
- the builder's associated trader account is the fee collector for Flight-routed
  orders
- once a client is configured with `flight: { builderAuthority, ... }`,
  supported order instructions are wrapped automatically

When you register Flight against a builder authority and its associated trader
account, all builder fees from Flight-routed orders accrue to that builder
trader account. Those fees are withdrawable from the Phoenix frontend.

> **Use embedded wallets for Flight integrations.** Integrators are strongly
> encouraged to provision an embedded wallet per user rather than routing
> Flight orders through the user's raw/external wallet. A raw wallet shares
> its Phoenix trader state with every other dapp the user trades on, which
> results in bad UX (positions, balances, and orders bleeding across
> integrations). Embedded wallets isolate per-integration state.

TypeScript:

```ts
import {
  MarginType,
  Side,
  createPhoenixClient,
  flight,
} from "@ellipsis-labs/rise";

const builderAuthority = "BUILDER_AUTHORITY";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  rpcUrl: "https://api.mainnet-beta.solana.com",
  ws: false,
  flight: {
    builderAuthority,
    builderPdaIndex: 0,
    builderSubaccountIndex: 0,
  },
});

const registerTraderIx = await client.ixs.buildRegisterTrader({
  authority: builderAuthority,
  marginType: MarginType.Cross,
});

const registerBuilderIx = await flight.buildRegisterBuilderIx({
  traderAuthority: builderAuthority,
  traderPdaIndex: 0,
  traderSubaccountIndex: 0,
  feeBps: 25n,
});

const routedMarketIx = await client.ixs.placeMarketOrder({
  authority: "TRADER_AUTHORITY",
  symbol: "SOL-PERP",
  orderPacket: await client.orderPackets.buildMarketOrderPacket({
    symbol: "SOL-PERP",
    side: Side.Bid,
    baseUnits: "0.25",
  }),
});

console.log(routedMarketIx.programAddress === flight.FLIGHT_PROGRAM_ADDRESS);
```

Rust:

```rust
use phoenix_rise::ix::flight::{RegisterBuilderParams, create_register_builder_ix};
use phoenix_rise::{MarketOrderTicket, PhoenixFlightClient, PhoenixTxBuilder, Side};

let register_builder_ix = create_register_builder_ix(
    RegisterBuilderParams::builder()
        .trader_authority(builder_authority)
        .trader_account(builder_trader_account)
        .fee_bps(25)
        .build()?,
)?;

let flight = PhoenixFlightClient::new(builder_authority, 0, 0);

let routed_ixs = builder
    .place_market_order(
        MarketOrderTicket::builder()
            .authority(trader_authority)
            .trader_account(trader_account)
            .symbol("SOL-PERP")
            .side(Side::Bid)
            .num_base_lots(25_000)
            .build()?,
    )
    .await?
    .into_iter()
    .map(|ix| flight.try_wrap_order_instruction(ix, trader_authority))
    .collect::<Result<Vec<_>, _>>()?;
```

Runnable Flight examples:

- [06-flight-market-order.ts](./ts/examples/06-flight-market-order.ts)
- [send_flight_market_order.rs](./rust/examples/send_flight_market_order.rs)

For more Flight-specific TypeScript examples, see
[rise/ts/src/flight/README.md](./ts/src/flight/README.md).

## Live Market Data

Use the typed WebSocket adapters when you want continuous updates instead of a
single HTTP snapshot.

TypeScript:

```ts
import { createPhoenixClient } from "@ellipsis-labs/rise";

const client = createPhoenixClient({
  apiUrl: "https://perp-api.phoenix.trade",
  ws: { connectMode: "eager" },
});

for await (const update of client.streams!.l2Book("SOL-PERP")) {
  console.log(update.bids[0], update.asks[0]);
}
```

Ready-to-run TypeScript stream examples:

- [02-ws-fills.ts](./ts/examples/02-ws-fills.ts)
- [phoenix-ws-example.ts](./ts/examples/phoenix-ws-example.ts)
