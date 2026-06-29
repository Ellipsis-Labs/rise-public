# Phoenix Rise Rust SDK

`phoenix-rise` is the facade crate for the Phoenix Rise Rust SDK. It re-exports
the account, instruction, math, type, API, event, and transaction-builder crates
behind feature profiles so integrators can choose a compact on-chain surface or
a batteries-included off-chain SDK.

For protocol concepts and product documentation, see
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Crate Navigation

- [`phoenix-rise`](../sdk/README.md): facade SDK and examples
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
- [`phoenix-rise-core`](../core/README.md): account fetchers, order tickets,
  and transaction builders
- [`phoenix-rise-litesvm-test`](../litesvm-test/README.md): fixture and
  LiteSVM localnet helpers
- [`phoenix-rise-cli`](../cli/README.md): command-line API, websocket, and RPC
  account inspection

## Feature Profiles

- `default = ["api", "ws", "sdk", "tx-builder"]`: HTTP/websocket clients,
  account fetchers, transaction builders, account views, instruction builders,
  math, and API types.
- `sdk`: account views, instruction builders, math, and API/domain types without
  API transport or RPC clients.
- `api`: enables `phoenix-rise-api` HTTP/auth route clients.
- `ws`: enables websocket clients and live trader-state helpers on top of
  `api`.
- `tx-builder`: enables `phoenix-rise-core` account fetchers and transaction
  builders.
- `core`: compatibility alias for `tx-builder`.
- `events`: enables market event parsing and core transaction adapters.
- `cpi`: program-friendly profile for account byte decoders, instruction
  layouts, and CPI helpers without the API/RPC/websocket stack.
- `serde`: JSON serialization for account views, instruction params, math
  wrappers, and API DTOs.
- `opentelemetry`: enables OpenTelemetry parent context propagation and trace
  headers for API HTTP and websocket clients.
- `utoipa`: OpenAPI schema derivations for API DTOs.

Compatibility aliases `solana-keypair`, `ed25519-dalek`, and `types-sdk` are
retained for existing manifests.

## Common Imports

```rust
use phoenix_rise::{
    api::PhoenixHttpClient,
    core::PhoenixTxBuilder,
    ix::{create_place_market_order_ix, Side},
    math::{MarketCalculator, QuoteLotsPerBaseLotPerTick},
};
```

For on-chain or CPI-oriented code, prefer a narrower dependency:

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

Run examples from `rise/rust/`:

```bash
cargo run -p phoenix-rise --example http_client --features api
cargo run -p phoenix-rise --example subscribe_l2_book --features ws -- SOL
cargo run -p phoenix-rise --example send_market_order --features api,tx-builder -- SOL
cargo run -p phoenix-rise --example fetch_on_chain_accounts --features api,tx-builder
```

The CLI has its own README with script-friendly commands:
[`phoenix-rise-cli`](../cli/README.md).
