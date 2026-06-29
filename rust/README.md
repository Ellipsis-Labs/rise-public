# Rust Rise SDK

`rise/rust` is the Rust SDK workspace for Phoenix perpetuals. It contains the
published `phoenix-rise` facade crate plus separately usable account, ix, math,
types, and API crates. The facade keeps the ergonomic SDK surface while the
lower-level crates can be enabled independently by CPI integrations, API-only
users, and math consumers.

Protocol and product documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Package Structure

```text
rust/
├── Cargo.toml               # workspace manifest and shared dependency table
├── README.md                # Canonical Rust package guide
├── AGENTS.md                # Pointer to this README and examples
├── accounts/                # phoenix-rise-accounts package
├── api/                     # phoenix-rise-api package
├── cli/                     # phoenix-rise-cli package
├── core/                    # phoenix-rise-core package
├── events/                  # phoenix-rise-events package
├── ix/                      # phoenix-rise-ix package
├── math/                    # phoenix-rise-math package
├── sdk/                     # phoenix-rise facade package
│   ├── examples/            # runnable SDK examples
│   ├── src/                 # facade exports and transaction helpers
│   └── tests/               # facade integration tests
├── litesvm-test/            # phoenix-rise-litesvm-test fixture/helper package
└── types/                   # phoenix-rise-types package
```

## Package Targets

- [`phoenix-rise`](sdk/README.md): facade crate imported as `phoenix_rise`,
  exposing the high-level SDK surface plus optional reexports of the
  lower-level crates
- [`phoenix-rise-accounts`](accounts/README.md): borrowed account views,
  account discriminators, PDA helpers, and account layout helpers
- [`phoenix-rise-ix`](ix/README.md): Phoenix, Ember, Flight, and Hawkeye
  instruction builders and instruction/account metadata
- [`phoenix-rise-math`](math/README.md): market math, margin/risk
  calculations, funding helpers, price conversions, and quantity wrappers
- [`phoenix-rise-events`](events/README.md): market event parsing from Phoenix
  `Log` and `LogEventLengths` batches
- [`phoenix-rise-types`](types/README.md): owned/materialized API, websocket,
  trader, market, and account snapshot types
- [`phoenix-rise-api`](api/README.md): HTTP, websocket, auth, exchange-cache,
  Flight, Hawkeye, and transport clients
- [`phoenix-rise-core`](core/README.md): account fetchers, order tickets, and
  transaction builders
- [`phoenix-rise-cli`](cli/README.md): CLI package for HTTP API calls,
  websocket probes, RPC account deserialization, and PerpAssetMap metadata
  inspection
- [`phoenix-rise-litesvm-test`](litesvm-test/README.md): LiteSVM fixture and
  localnet helpers for tests

## Changelog

The public Rust SDK changelog is published at
<https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/CHANGELOG.md>.

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
  `build_cancel_all_orders(...)`, `build_cancel_up_to(...)`,
  `build_cancel_bracket_leg(...)`
- Maintenance helpers: `build_uncross_crank(...)`
- Trader and collateral flows: `build_register_trader(...)`,
  `build_deposit_funds(...)`, `build_withdraw_funds(...)`,
  `build_transfer_collateral(...)`, `build_sync_parent_to_child(...)`
- Isolated helpers: `build_isolated_market_order(...)`,
  `build_isolated_limit_order(...)`

### `PhoenixFlightClient`

Use this when you want to wrap supported Phoenix placement instructions through
Flight. Use `try_wrap_order_instruction_with_fee_bps_override(...)` when a
specific wrapped order should use Flight's `proxy_instruction_with_fee_override`
variant instead of the builder's registered fee.

> **Use embedded wallets for Flight integrations.** Integrators are strongly
> encouraged to provision an embedded wallet per user rather than routing
> Flight orders through the user's raw/external wallet. A raw wallet shares
> its Phoenix trader state with every other dapp the user trades on, which
> results in bad UX (positions, balances, and orders bleeding across
> integrations). Embedded wallets isolate per-integration state.

## Crate Internals By Area

### `accounts/src/`

- Raw views for Phoenix account data: `trader.rs`,
  `perp_asset_map.rs`, `global_config.rs`, `permission.rs`,
  `conditional_orders.rs`, `stop_losses.rs`, and withdrawal/escrow layouts
- `owned/`: optional serde-backed owned account readers for off-chain decoding
- `discriminants.rs`: account discriminator constants
- `common.rs`: shared parsing and account-data helpers

### `ix/src/`

- Order placement: `limit_order.rs`, `market_order.rs`, `multi_limit_order.rs`
- Order cancellation: `cancel_orders.rs`, `cancel_stop_loss.rs`
- Conditional orders: `conditional_order.rs`
- Collateral and funding flows: `deposit_funds.rs`, `withdraw_funds.rs`,
  `transfer_collateral.rs`, `sync_parent_to_child.rs`
- Account lifecycle: `register_trader.rs`, `create_ata.rs`, `spl_approve.rs`
- Ember conversions: `ember_deposit.rs`, `ember_withdraw.rs`
- Flight-specific builders: `flight/register_builder.rs`,
  `flight/update_fee.rs`, and proxy helpers
- Hawkeye view instructions and return-data decoding: `hawkeye.rs`

### `math/src/`

- Price and lot conversions: `market_math.rs`, `price.rs`
- Margin and risk: `margin.rs`, `margin_calc.rs`, `risk.rs`,
  `leverage_tiers.rs`
- Portfolio state: `portfolio.rs`, `trader_position.rs`,
  `limit_order_state.rs`
- Quantity wrappers: `quantities/`

### `types/src/`

- Exchange and market payloads: `exchange.rs`, `exchange_ws.rs`, `market.rs`,
  `market_state.rs`, `market_stats.rs`, `l2book.rs`, `candles.rs`, `trades.rs`
- Trader payloads: `trader.rs`, `trader_http.rs`
- Shared protocol types: `auth.rs`, `core.rs`, `ix.rs`, `service_accounts.rs`,
  `ws.rs`

### `api/src/`

- `http_client.rs`: `PhoenixHttpClient` plus auth-aware builder
- `ws_client.rs`: low-level typed websocket client
- `client.rs`: reconnecting high-level `PhoenixClient`
- `flight_client.rs`: Flight wrapper for supported order instructions
- `hawkeye_client.rs`: Hawkeye simulation helpers
- `auth.rs`, `auth_lifecycle.rs`, `auth_signers.rs`: session storage, login
  flows, and signer abstractions
- `exchange_cache.rs`: exchange metadata storage and change events
- `routes/`: REST route client modules

### `core/src/`

- `account_client.rs`: fetcher trait and account-data decoding client
- `tx_builder.rs`: metadata-backed transaction builder
- `order_tickets.rs`: typed tickets consumed by `PhoenixTxBuilder`
- `hawkeye_client.rs`: Hawkeye simulation helpers

### `sdk/src/`

- `lib.rs`: facade exports for the split crates and high-level SDK types

## Features

- `phoenix-rise`
  - `default = ["api", "ws", "sdk", "tx-builder"]`: HTTP/websocket clients,
    account fetchers, transaction builders, account views, instruction
    builders, and math
  - `sdk`: account views, instruction builders, math, and domain types,
    without API transport or RPC dependencies
  - `api`: enables the `phoenix-rise-api` HTTP/auth surface
  - `ws`: enables websocket clients and live trader-state helpers on top of
    `api`
  - `tx-builder`: enables local transaction helpers and RPC-backed bracket
    flows plus account fetchers through `phoenix-rise-core`
  - `core`: compatibility alias for `tx-builder`
  - `events`: enables market event parsing and core adapters for RPC and
    Yellowstone-style transaction instruction streams
  - `cpi`: program-friendly facade profile for account byte decoders,
    instruction layouts, and Pinocchio CPI helpers without API, RPC,
    websocket, math, type DTO, or `solana-instruction` dependencies
  - `types-sdk`: enables client-side owned API/domain helpers such as
    `TraderKey`, `Trader`, and `PhoenixMetadata`
  - `solana-keypair`, `ed25519-dalek`: compatibility aliases for `api`
  - `opentelemetry`: enables API trace propagation and trace headers
  - `serde`: JSON serialization for account views, instruction params, math
    wrappers, and API DTOs
  - `utoipa`: enables OpenAPI schema derivations where needed

## Examples

Use `rise/rust/sdk/examples/` as the main reference set:

- HTTP and auth: `http_client.rs`, `register_trader.rs`
- WebSocket subscriptions: `subscribe_trader_state.rs`,
  `subscribe_market_stats.rs`, `subscribe_l2_book.rs`,
  `subscribe_candles.rs`, `subscribe_trades.rs`, `ws_debug_cli.rs`
- Transaction building and trading: `send_limit_order.rs`,
  `send_market_order.rs`, `send_flight_market_order.rs`, `cancel_order.rs`,
  `cancel_stop_loss.rs`, `deposit_funds.rs`, `referral_activation_tx.rs`,
  `builder_onboarding_tx.rs`, `onboard_trader_delegated.rs`
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

cargo run -p phoenix-rise --example http_client --features api
cargo run -p phoenix-rise --example subscribe_l2_book --features ws -- SOL
cargo run -p phoenix-rise --example subscribe_trader_state --features ws
cargo run -p phoenix-rise --example referral_activation_tx --features api,tx-builder -- \
    REFERRAL_CODE --trader-keypair-path ~/.config/solana/id.json
cargo run -p phoenix-rise --example builder_onboarding_tx --features api -- \
    --trader-keypair-path ~/.config/solana/id.json
cargo run -p phoenix-rise --example send_market_order --features api,tx-builder -- SOL
cargo run -p phoenix-rise --example send_flight_market_order --features api,tx-builder -- \
    Builder1111111111111111111111111111111111 0 0 SOL bid 67

cargo run -p phoenix-rise-cli -- --json market list
cargo run -p phoenix-rise-cli -- --rpc-url http://localhost:8899 --json rpc account \
    --address <ACCOUNT_PUBKEY> --account-type perp-asset-map
cargo run -p phoenix-rise-cli -- --rpc-url http://localhost:8899 --json rpc perp-asset \
    --perp-asset-map <PERP_ASSET_MAP_PUBKEY> --symbol SOL
```

`referral_activation_tx` uses `/v1/referral/activate-tx` when the user has a
referral code. `builder_onboarding_tx` uses
`/v1/exchange/build-register-ixs` and `/v1/exchange/send-register-ixs` to
register a trader without a referral code.
