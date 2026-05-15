# Rust Rise SDK

`rise/rust` is the Rust workspace for Phoenix perpetuals. It contains the
published `phoenix-rise` crate, its source roots for instruction builders,
math helpers, and API payload types, plus a small CLI for smoke testing.

## Workspace Structure

```text
rust/
├── Cargo.toml               # Workspace manifest
├── README.md                # Canonical Rust workspace guide
├── AGENTS.md                # Pointer to this README and examples
├── cli/                     # phoenix-rise-sdk-cli
├── ix/                      # source for phoenix_rise::ix
├── math/                    # source for phoenix_rise::math
├── sdk/                     # source for the phoenix-rise library
│   ├── AGENTS.md
│   ├── examples/
│   ├── src/
│   │   ├── api/
│   │   ├── accounts.rs
│   │   ├── auth.rs
│   │   ├── auth_lifecycle.rs
│   │   ├── auth_signers.rs
│   │   ├── client.rs
│   │   ├── env.rs
│   │   ├── exchange_cache.rs
│   │   ├── flight_client.rs
│   │   ├── http_client.rs
│   │   ├── order_tickets.rs
│   │   ├── transport.rs
│   │   ├── tx_builder.rs
│   │   └── ws_client.rs
│   └── tests/
└── types/                   # source for phoenix_rise::types
```

## Workspace Packages

- `phoenix-rise`: published Rust SDK crate, imported as `phoenix_rise`,
  exposing the high-level HTTP, WebSocket, auth, exchange-cache, Flight, and
  transaction-builder surface, plus the lower-level `ix`, `math`, and `types`
  modules
- `phoenix-rise-sdk-cli`: smoke-test CLI for HTTP, websocket, and auth flows

## Main SDK Surfaces

### `PhoenixHttpClient`

Use this when you want typed REST access without the reconnecting live runtime.

- Exchange and market data: `exchange()`, `markets()`, `candles()`
- Trader and history routes: `traders()`, `orders()`, `trades()`,
  `collateral()`, `funding()`
- Invite activation: `invite()`
- Public constructors: `PhoenixHttpClient::new_from_env()` and
  `PhoenixHttpClient::builder(...)`
- Auth-enabled constructor: `PhoenixHttpClient::new_from_env_with_auth()`

### `PhoenixWSClient`

Use this when you want direct typed websocket subscriptions without the
higher-level reconnecting client.

- `subscribe_to_all_mids()`
- `subscribe_to_funding_rate(symbol)`
- `subscribe_to_orderbook(symbol)`
- `subscribe_to_market(symbol)`
- `subscribe_to_trades(symbol)`
- `subscribe_to_candles(symbol, timeframe)`
- `subscribe_to_trader_state(authority)`
- `subscribe_to_trader_state_with_pda(authority, trader_pda_index)`

### `PhoenixClient`

Use this when you want the higher-level Rust client that combines HTTP
bootstrap with a reconnecting live subscription runtime.

- Bootstraps exchange metadata over HTTP
- Reconnects websocket subscriptions automatically
- Exposes `subscribe(...)` over `PhoenixSubscription`
- Emits typed `PhoenixClientEvent` updates

### `PhoenixTxBuilder`

Use this when you want local Solana instruction construction from cached
metadata rather than server-assisted order helpers.

- Cross-margin order placement: `place_market_order(...)`,
  `place_limit_order(...)`
- Cancellation helpers: `build_cancel_orders(...)`,
  `build_cancel_bracket_leg(...)`
- Trader and collateral flows: `build_register_trader(...)`,
  `build_deposit_funds(...)`, `build_withdraw_funds(...)`,
  `build_transfer_collateral(...)`, `build_sync_parent_to_child(...)`
- Isolated helpers: `build_isolated_market_order(...)`,
  `build_isolated_limit_order(...)`

### `PhoenixFlightClient`

Use this when you want to wrap supported Phoenix order instructions through
Flight so builder fees are routed to the configured builder trader account.

> **Use embedded wallets for Flight integrations.** Integrators are strongly
> encouraged to provision an embedded wallet per user rather than routing
> Flight orders through the user's raw/external wallet. A raw wallet shares
> its Phoenix trader state with every other dapp the user trades on, which
> results in bad UX (positions, balances, and orders bleeding across
> integrations). Embedded wallets isolate per-integration state.

## Crate Internals By Area

### `sdk/`

- `src/http_client.rs`: `PhoenixHttpClient` plus auth-aware builder
- `src/ws_client.rs`: low-level typed websocket client
- `src/client.rs`: reconnecting high-level `PhoenixClient`
- `src/tx_builder.rs`: metadata-backed transaction builder
- `src/order_tickets.rs`: typed tickets consumed by `PhoenixTxBuilder`
- `src/flight_client.rs`: Flight wrapper for supported order instructions
- `src/auth.rs`, `src/auth_lifecycle.rs`, `src/auth_signers.rs`: session
  storage, login flows, and signer abstractions
- `src/exchange_cache.rs`: exchange metadata storage and change events

### `ix/`

- Order placement: `limit_order.rs`, `market_order.rs`, `multi_limit_order.rs`
- Order cancellation: `cancel_orders.rs`, `cancel_stop_loss.rs`
- Conditional orders: `conditional_order.rs`
- Collateral and funding flows: `deposit_funds.rs`, `withdraw_funds.rs`,
  `transfer_collateral.rs`, `sync_parent_to_child.rs`
- Account lifecycle: `register_trader.rs`, `create_ata.rs`, `spl_approve.rs`
- Ember conversions: `ember_deposit.rs`, `ember_withdraw.rs`
- Flight-specific builders: `flight/register_builder.rs`,
  `flight/update_fee.rs`, and proxy helpers

### `math/`

- Price and lot conversions: `market_math.rs`, `price.rs`
- Margin and risk: `margin.rs`, `margin_calc.rs`, `risk.rs`,
  `leverage_tiers.rs`
- Portfolio state: `portfolio.rs`, `trader_position.rs`,
  `limit_order_state.rs`
- Quantity wrappers: `quantities/`

### `types/`

- Exchange and market payloads: `exchange.rs`, `exchange_ws.rs`, `market.rs`,
  `market_state.rs`, `market_stats.rs`, `l2book.rs`, `candles.rs`, `trades.rs`
- Trader payloads: `trader.rs`, `trader_http.rs`, `trader_key.rs`,
  `trader_state.rs`
- Shared protocol types: `ws.rs`, `ws_error.rs`, `subscription_key.rs`,
  `client.rs`, `core.rs`, `http_error.rs`, `ix.rs`
- Account-backed types: `accounts/`

## Features

- `phoenix-rise`
  - `solana-keypair`: enables wallet-keypair login helpers
  - `ed25519-dalek`: enables Ed25519 service-account auth signing helpers
  - `rust_decimal`: enables decimal-backed helpers
  - `utoipa`: enables OpenAPI schema derivations where needed

## Examples

Use `rise/rust/sdk/examples/` as the main reference set:

- HTTP and auth: `http_client.rs`, `register_trader.rs`
- WebSocket subscriptions: `subscribe_trader_state.rs`,
  `subscribe_market_stats.rs`, `subscribe_l2_book.rs`,
  `subscribe_candles.rs`, `subscribe_trades.rs`, `ws_debug_cli.rs`
- Transaction building and trading: `send_limit_order.rs`,
  `send_market_order.rs`, `send_flight_market_order.rs`, `cancel_order.rs`,
  `cancel_stop_loss.rs`, `deposit_funds.rs`
- Isolated flows: `isolated_limit_order.rs`,
  `isolated_market_order_client.rs`, `isolated_market_order_server.rs`
- Broader reference flows: `phoenix_client.rs`, `market_maker.rs`,
  `compute_trader_margin.rs`, `fetch_on_chain_accounts.rs`

## Build And Run

Run commands from `rise/rust/`:

```bash
cargo build
cargo test

# Common env vars:
# PHOENIX_API_URL=https://perp-api.phoenix.trade
# PHOENIX_WS_URL=wss://perp-api.phoenix.trade/v1/ws

cargo run -p phoenix-rise --example http_client
cargo run -p phoenix-rise --example subscribe_l2_book -- SOL
cargo run -p phoenix-rise --example subscribe_trader_state --features solana-keypair
cargo run -p phoenix-rise --example send_market_order --features solana-keypair -- SOL
cargo run -p phoenix-rise --example send_flight_market_order --features solana-keypair -- \
    Builder1111111111111111111111111111111111 0 0 SOL bid 67
```
