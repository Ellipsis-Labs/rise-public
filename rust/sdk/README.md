# Phoenix Rise Rust SDK

`phoenix-rise` is the facade crate for the Phoenix Rise Rust SDK. It re-exports
the account, instruction, math, type, API, event, and transaction-builder crates
behind feature flags for both on-chain CPI programs and off-chain clients.

Protocol documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Crate Navigation

- [Workspace guide](../README.md): crate architecture, feature flags, and
  examples
- [`phoenix-rise-accounts`](../accounts/README.md): account byte views,
  discriminators, PDA helpers, and optional serde output
- [`phoenix-rise-ix`](../ix/README.md): instruction builders, discriminants,
  account metas, and optional CPI helpers
- [`phoenix-rise-math`](../math/README.md): market math, margin/risk helpers,
  funding helpers, and quantity wrappers
- [`phoenix-rise-events`](../events/README.md): Phoenix market event parsing
- [`phoenix-rise-types`](../types/README.md): API and websocket DTOs
- [`phoenix-rise-api`](../api/README.md): HTTP, websocket, auth, and transport
  clients
- [`phoenix-rise-core`](../core/README.md): account fetchers, order tickets, and
  transaction builders
- [`phoenix-rise-litesvm-test`](../litesvm-test/README.md): fixture and LiteSVM
  localnet helpers
- [`phoenix-rise-cli`](../cli/README.md): command-line API, websocket, and RPC
  account inspection

## Feature Profiles

- `default = ["api", "ws", "sdk", "tx-builder"]`: HTTP and websocket clients,
  account fetchers, transaction builders, account views, instruction builders,
  math, and API types.
- `sdk`: account views, instruction builders, math, and API/domain types without
  API transport or RPC clients.
- `api`: `phoenix-rise-api` HTTP and auth route clients.
- `ws`: websocket clients and live trader-state helpers on top of `api`.
- `tx-builder`: `phoenix-rise-core` account fetchers and transaction builders.
- `core`: compatibility alias for `tx-builder`.
- `events`: market event parsing and core transaction adapters.
- `cpi`: on-chain profile with account decoders, instruction layouts, and CPI
  helpers without the API, RPC, or websocket stack.
- `serde`: JSON serialization for account views, instruction params, math
  wrappers, and API DTOs.
- `opentelemetry`: OpenTelemetry parent context propagation and trace headers
  for HTTP and websocket clients.
- `utoipa`: OpenAPI schema derivations for API DTOs.

Compatibility aliases `solana-keypair`, `ed25519-dalek`, and `types-sdk` remain
available for existing manifests.

## Common Imports

```rust
use phoenix_rise::{
    api::PhoenixHttpClient,
    core::PhoenixTxBuilder,
    ix::{create_place_market_order_ix, Side},
    math::{MarketCalculator, QuoteLotsPerBaseLotPerTick},
};
```

For on-chain or CPI programs, use a minimal feature set:

```toml
[dependencies]
phoenix-rise = { version = "0.2", default-features = false, features = ["cpi"] }
```

For API-only clients:

```toml
[dependencies]
phoenix-rise = { version = "0.2", default-features = false, features = ["api"] }
```

## Examples

Run examples from `rust/`:

```bash
cargo run -p phoenix-rise --example http_client --features api
cargo run -p phoenix-rise --example subscribe_l2_book --features ws -- SOL
cargo run -p phoenix-rise --example send_market_order --features api,tx-builder -- SOL
cargo run -p phoenix-rise --example fetch_on_chain_accounts --features api,tx-builder
```

See [`phoenix-rise-cli`](../cli/README.md) for script-friendly CLI commands.
