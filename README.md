# Rise SDK

TypeScript, Rust, and on-chain CPI SDKs for Phoenix perpetuals.

[![Rise CI](https://github.com/Ellipsis-Labs/rise-public/actions/workflows/ci.yaml/badge.svg)](https://github.com/Ellipsis-Labs/rise-public/actions/workflows/ci.yaml)
[![crates.io](https://img.shields.io/crates/v/phoenix-rise.svg)](https://crates.io/crates/phoenix-rise)
[![npm](https://img.shields.io/npm/v/@ellipsis-labs/rise.svg)](https://www.npmjs.com/package/@ellipsis-labs/rise)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)

Rise is the developer SDK for Phoenix perpetuals. The TypeScript package (`ts`,
`@ellipsis-labs/rise`) provides HTTP route clients, a unified exchange-aware
client, instruction builders, Flight helpers, and WebSocket adapters. The Rust
workspace (`rust`, centered on `phoenix-rise`) provides typed HTTP/WS clients,
transaction builders, math helpers, low-level instruction builders, and
zero-copy account views for on-chain CPI programs (`programs`).

Documentation: [docs.phoenix.trade](https://docs.phoenix.trade) ·
[ts/README.md](./ts/README.md) · [rust/README.md](./rust/README.md) ·
[rust/CRATES.md](./rust/CRATES.md) ·
[ts/examples/README.md](./ts/examples/README.md)

Where to start:

- `PhoenixHttpClient` / `client.api`: public HTTP data and referral activation
- `createPhoenixClient(...)`: exchange metadata, PDA derivation, order-packet
  helpers, and `client.ixs`
- `client.streams` or `createPhoenixWsClient(...)`: typed live WebSocket
  adapters
- `PhoenixTxBuilder` (Rust): build and sign Solana instructions
- `phoenix_rise::accounts::{Trader, TraderHeader, PerpAssetMap, GlobalConfig, PermissionAccount}`:
  borrowed Phoenix account views for on-chain CPI integrations without pulling
  in the full off-chain SDK

## Install

TypeScript:

```bash
npm install @ellipsis-labs/rise
```

Rust:

```bash
cargo add phoenix-rise
```

## Runnable Entry Points

TypeScript:

| Entry point                                                                                          | Description                                                                                                                      |
| ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| [ts/examples/01-http-client.ts](./ts/examples/01-http-client.ts)                                     | Query exchange keys, SOL market config, candles, and trader state over HTTP                                                      |
| [ts/examples/02-ws-fills.ts](./ts/examples/02-ws-fills.ts)                                           | Subscribe to live market trade fills over WebSocket                                                                              |
| [ts/examples/03-build-limit-order-ix.ts](./ts/examples/03-build-limit-order-ix.ts)                   | Load exchange metadata, encode a limit-order packet, and build a place-limit-order instruction                                   |
| [ts/examples/04-trader-state-store.ts](./ts/examples/04-trader-state-store.ts)                       | Maintain a live `TraderState` store backed by `client.streams.traderState(...)`                                                  |
| [ts/examples/05-cancel-all-conditional-orders.ts](./ts/examples/05-cancel-all-conditional-orders.ts) | Fetch a trader's conditional-orders account and batch-cancel active trigger legs                                                 |
| [ts/examples/06-flight-market-order.ts](./ts/examples/06-flight-market-order.ts)                     | Build a Flight-routed market-order instruction that credits fees to a builder trader account                                     |
| [ts/examples/09-referral-activation-tx.ts](./ts/examples/09-referral-activation-tx.ts)               | Build and submit a `/v1/referral/activate-tx` delegated onboarding transaction signed by the trader authority                    |
| [ts/examples/10-builder-onboarding-tx.ts](./ts/examples/10-builder-onboarding-tx.ts)                 | Register and onboard a trader without a referral code via `/v1/exchange/build-register-ixs` and `/v1/exchange/send-register-ixs` |
| [ts/examples/phoenix-client-example.ts](./ts/examples/phoenix-client-example.ts)                     | Walkthrough of HTTP routes covering markets, trader views, history, and PnL                                                      |
| [ts/examples/phoenix-ws-example.ts](./ts/examples/phoenix-ws-example.ts)                             | Interactive WebSocket demo for the built-in stream adapters                                                                      |
| [ts/README.md](./ts/README.md)                                                                       | TypeScript SDK package guide and API overview                                                                                    |
| [ts/examples/README.md](./ts/examples/README.md)                                                     | CLI flags and run instructions for all TypeScript examples                                                                       |

Rust:

| Entry point                                                                                      | Description                                                                                                                      |
| ------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| [rust/README.md](./rust/README.md)                                                               | Rust workspace guide, crate architecture, feature flags, and CLI usage                                                           |
| [rust/sdk/examples/register_trader.rs](./rust/sdk/examples/register_trader.rs)                   | Authenticate and activate a referral code via `POST /v1/referral/activate`                                                       |
| [rust/sdk/examples/http_client.rs](./rust/sdk/examples/http_client.rs)                           | Query exchange keys, market config, trade history, and trader PnL over REST                                                      |
| [rust/sdk/examples/send_limit_order.rs](./rust/sdk/examples/send_limit_order.rs)                 | Build and submit a signed limit-order transaction with `PhoenixTxBuilder`                                                        |
| [rust/sdk/examples/send_market_order.rs](./rust/sdk/examples/send_market_order.rs)               | Build and submit a signed market-order transaction with `PhoenixTxBuilder`                                                       |
| [rust/sdk/examples/send_flight_market_order.rs](./rust/sdk/examples/send_flight_market_order.rs) | Wrap a market order in Flight routing and submit the transaction                                                                 |
| [rust/sdk/examples/referral_activation_tx.rs](./rust/sdk/examples/referral_activation_tx.rs)     | Build and submit a `/v1/referral/activate-tx` delegated onboarding transaction signed by the trader authority                    |
| [rust/sdk/examples/builder_onboarding_tx.rs](./rust/sdk/examples/builder_onboarding_tx.rs)       | Register and onboard a trader without a referral code via `/v1/exchange/build-register-ixs` and `/v1/exchange/send-register-ixs` |

On-chain CPI examples:

| Program                                                                        | Description                                                                                                                                           |
| ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| [programs/close-position-and-withdraw](./programs/close-position-and-withdraw) | Pinocchio CPI example that closes a position (optionally via Flight), reads Hawkeye margin data, withdraws collateral, and converts to USDC via Ember |
| [programs/example-program](./programs/example-program)                         | Reference Pinocchio program exercising deposits, withdrawals, market/limit orders, stop losses, Hawkeye reads, and isolated subaccount flows          |
| [programs/trader-onboarder](./programs/trader-onboarder)                       | Pinocchio CPI program using a static program PDA to hold Phoenix trader-onboarding delegation                                                         |

## Onboarding Paths

These onboarding routes are not interchangeable:

- `POST /v1/referral/activate` (`invite().activateReferral(...)` in TypeScript,
  `invite().activate_referral(...)` in Rust): requires a referral code and an
  authenticated user session for the trader authority.
- `POST /v1/referral/activate-tx`: delegated onboarding with a required referral
  code; the trader authority must sign the transaction.
- `POST /v1/exchange/build-register-ixs` followed by
  `POST /v1/exchange/send-register-ixs`: builder onboarding without a referral
  code. The trader authority is passed only as a public key, the builder's fee
  payer is the sole local signer, and the API adds the onboarder signature after
  validating and simulating the submitted transaction. In the TypeScript and
  Rust builder-onboarding examples, `--fee-payer-keypair-path` defaults to
  `~/.config/solana/id.json` and `--trader-authority` defaults to that fee
  payer's public key.

See the [Trader onboarding guide](https://docs.phoenix.trade/sdk/register) for
copyable Rust and TypeScript walkthroughs of both delegated onboarding paths, or
run [09-referral-activation-tx.ts](./ts/examples/09-referral-activation-tx.ts),
[10-builder-onboarding-tx.ts](./ts/examples/10-builder-onboarding-tx.ts),
[referral_activation_tx.rs](./rust/sdk/examples/referral_activation_tx.rs), and
[builder_onboarding_tx.rs](./rust/sdk/examples/builder_onboarding_tx.rs).

Access-code activation (`POST /v1/invite/activate`) is retired. Use
`POST /v1/referral/activate` instead (see
[register_trader.rs](./rust/sdk/examples/register_trader.rs) for a runnable Rust
example).

## Fetching Exchange, Market, and Trader State

HTTP routes are split by state scope:

- `exchange().getSnapshot()`: exchange-wide state and every market's current
  config snapshot
- `exchange().getMarket(symbol)`: single-market fees, risk, funding cadence, and
  configuration
- `orderbook().getOrderbook(symbol)`: single-market HTTP L2 snapshot
- `traders().getTraderStateSnapshot(...)` (TypeScript): trader collateral,
  positions, orders, and triggers
- `markets().getMarketStatsHistory(...)` and
  `funding().getFundingRateHistory(...)`: time-series data for frontends,
  vaults, and analytics

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
use phoenix_rise::api::PhoenixHttpClient;
use solana_pubkey::Pubkey;
use std::str::FromStr;

let client = PhoenixHttpClient::new_from_env()?;
let authority = Pubkey::from_str("AUTHORITY_PUBKEY")?;

let snapshot = client.exchange().get_snapshot().await?;
let market = client.markets().get_market("SOL").await?;
let pnl = client.traders().get_trader_pnl(&authority, Default::default()).await?;
```

See [01-http-client.ts](./ts/examples/01-http-client.ts) and
[phoenix-client-example.ts](./ts/examples/phoenix-client-example.ts) for full
TypeScript walkthroughs.

## Order Placement and Cancellation

Order placement separates packet encoding from Solana instruction assembly:

- Build packet sizes and prices with `client.orderPackets`.
- Build or wrap Solana instructions with `client.ixs`.
- Use the lower-level builders for conditional-account setup and other
  specialized flows.

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

`buildPlaceStopLoss` and `buildCancelStopLoss` are deprecated in favor of the
conditional-order builders (`buildPlacePositionConditionalOrder`,
`buildCancelConditionalOrder`). `buildPlaceStopLoss(...)` expects a tick-based
trigger price; convert USD prices using market metadata first, or follow the
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
- [send_limit_order.rs](./rust/sdk/examples/send_limit_order.rs)
- [send_market_order.rs](./rust/sdk/examples/send_market_order.rs)
- [cancel_order.rs](./rust/sdk/examples/cancel_order.rs)

## Flight Builder Activation and Routed Orders

Flight routes orders through a registered builder:

- Register a Phoenix trader account for the builder first, then submit the
  separate on-chain builder registration instruction.
- Builder fees from Flight-routed orders accrue to the builder's associated
  trader account and are withdrawable from the Phoenix frontend.
- Configuring a client with `flight: { builderAuthority, ... }` wraps supported
  order instructions automatically.
- `flight.buildSetOnboarderSignerIx` / `create_set_onboarder_signer_ix` sets the
  builder's onboarder signer; `flight.buildOnboardTraderIx` /
  `create_onboard_trader_ix` onboards a trader from the builder's backend
  (co-signed by that onboarder key).

> **Use embedded wallets for Flight integrations.** Provision an embedded wallet
> per user instead of routing Flight orders through the user's external wallet.
> External wallets share Phoenix trader state across every dapp the user trades
> on, causing positions, balances, and orders to bleed across integrations.
> Embedded wallets isolate per-integration state.

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
    // false = owner-signed (no position authority)
    .map(|ix| flight.try_wrap_order_instruction(ix, trader_authority, false))
    .collect::<Result<Vec<_>, _>>()?;
```

Runnable Flight examples:

- [06-flight-market-order.ts](./ts/examples/06-flight-market-order.ts)
- [send_flight_market_order.rs](./rust/sdk/examples/send_flight_market_order.rs)

See [ts/src/flight/README.md](./ts/src/flight/README.md) for additional
Flight-specific TypeScript examples.

## API and WebSocket Compatibility

Unknown WebSocket channels and unfamiliar `messageType` values are skipped
without ending the stream, closing the connection, or emitting an error event.
New message types must not consume sequence numbers needed by existing delta
streams or replace updates that existing clients require.

Response objects may gain fields. Default JSON decoders ignore unfamiliar
fields, and exchange deltas skip unfamiliar operation kinds while applying known
operations and advancing their sequence. Within a protocol version, existing
field types, meanings, and sequence rules must remain compatible, and new
operations must not replace updates for state that older clients maintain.

Escrow actions with unfamiliar `kind` values decode as `Unknown` in Rust and
`{ kind: "unknown" }` in TypeScript. They stay in the request's action list and
are not no-ops; check for this marker before presenting or accepting a request,
because a client cannot explain the full effect of an unfamiliar action.
Malformed payloads for known action kinds still fail validation. TypeScript's
opt-in strict mode can reject added object fields and is intended for schema
diagnostics rather than forward-compatible production decoding.

This policy only protects clients running a release that includes these
fallbacks; it cannot retroactively update older installed SDK versions. Adding
data that changes existing margin, withdrawal, or trading semantics requires a
compatible versioned or negotiated contract. Ignoring an unfamiliar field does
not guarantee that an older client's financial calculations remain correct.

## Live Market Data

Use the typed WebSocket adapters for continuous updates instead of a single HTTP
snapshot.

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

Runnable TypeScript stream examples:

- [02-ws-fills.ts](./ts/examples/02-ws-fills.ts)
- [phoenix-ws-example.ts](./ts/examples/phoenix-ws-example.ts)

## Release notes

- TypeScript:
  [ts/CHANGELOG.md](https://github.com/Ellipsis-Labs/rise-public/blob/master/ts/CHANGELOG.md)
- Rust:
  [rust/CHANGELOG.md](https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/CHANGELOG.md)

## License

MIT, see [LICENSE](./LICENSE).
