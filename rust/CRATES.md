# Rise Rust SDK Crates

This workspace publishes Rust crates to crates.io for use independently or
through the `phoenix-rise` facade crate.

## Published Crates

- [`phoenix-rise`](sdk/README.md): facade crate imported as `phoenix_rise`,
  exposing the high-level SDK surface and optional lower-level crate reexports
- [`phoenix-rise-accounts`](accounts/README.md): borrowed Phoenix account views,
  optional owned readers, account discriminators, PDA helpers, and account
  layout helpers
- [`phoenix-rise-ix`](ix/README.md): Phoenix, Ember, Flight, and Hawkeye
  instruction builders, account metas, CPI helpers, and ix payloads
- [`phoenix-rise-math`](math/README.md): pure market math, margin and risk
  calculations, funding helpers, price conversions, and quantity wrappers
- [`phoenix-rise-events`](events/README.md): Phoenix market event parsing from
  `Log` and `LogEventLengths` batches
- [`phoenix-rise-types`](types/README.md): owned/materialized API, websocket,
  trader, market, and account snapshot types
- [`phoenix-rise-api`](api/README.md): HTTP, websocket, auth, exchange-cache,
  Flight, Hawkeye, and transport clients
- [`phoenix-rise-core`](core/README.md): off-chain account fetchers, order
  tickets, and transaction builders
- [`phoenix-rise-cli`](cli/README.md): CLI for HTTP API calls, websocket probes,
  RPC account deserialization, and PerpAssetMap metadata inspection
- [`phoenix-rise-litesvm-test`](litesvm-test/README.md): LiteSVM fixture and
  localnet helpers for testing Rise program integrations

All crates in this workspace share the same version.

```toml
[dependencies]
phoenix-rise = "0.2"
```

Use the lower-level crates directly when you only need one SDK layer:

```toml
[dependencies]
phoenix-rise-accounts = "0.2"
phoenix-rise-ix = "0.2"
phoenix-rise-math = "0.2"
```

## Facade Features

- `default = ["api", "ws", "sdk", "tx-builder"]`: HTTP/websocket clients,
  account fetchers, transaction builders, account views, instruction builders,
  and math
- `sdk`: account views, instruction builders, math, and domain types without API
  transport or RPC dependencies
- `api`: `phoenix-rise-api` typed HTTP/auth clients and API route helpers
- `ws`: websocket clients and live trader-state helpers on top of `api`
- `tx-builder`: local transaction builder helpers, RPC-backed bracket flows, and
  account fetchers via `phoenix-rise-core`
- `core`: compatibility alias for `tx-builder`
- `events`: market event parsing and core adapters for RPC and Yellowstone-style
  transaction instruction streams
- `cpi`: program-friendly facade profile for account byte decoders, instruction
  layouts, and Pinocchio CPI helpers without API, RPC, websocket, math, type
  DTO, or `solana-instruction` dependencies
- `types-sdk`: compatibility alias for `types`, which enables the client-side
  owned DTOs from `phoenix-rise-types`
- `solana-keypair`, `ed25519-dalek`: compatibility aliases for `api`
- `opentelemetry`: API trace propagation and trace headers
- `serde`: JSON serialization for account views, instruction params, math
  wrappers, and API DTOs
- `utoipa`: OpenAPI schema derivations for API types
- PDA helpers in `phoenix-rise-accounts` / `phoenix-rise-ix`: available by
  default. On Solana targets these helpers use syscall-backed `solana-pubkey`.
- `cpi` on `phoenix-rise-ix`: additive Pinocchio CPI invoke helpers; raw
  instruction data and explicit account metas do not require Pinocchio

## Solana Pubkey Compatibility

The crates accept `solana-pubkey` `>=3.0.0, <5` and resolve to 4.x by default.
Both majors re-export `solana_address::Address` (2.x) as `Pubkey`, so a Rise
`Pubkey` matches the caller's `Pubkey` either way. Projects pinned to
`solana-pubkey` 3.0 can drop the duplicate crate with
`cargo update -p solana-pubkey@<4.x version> --precise 3.0.0` if nothing else in
the dependency graph requires 4.x.

## Source Layout

- [`accounts/`](accounts/README.md): `phoenix-rise-accounts`
- [`ix/`](ix/README.md): `phoenix-rise-ix`
- [`math/`](math/README.md): `phoenix-rise-math`
- [`events/`](events/README.md): `phoenix-rise-events`
- [`types/`](types/README.md): `phoenix-rise-types`
- [`api/`](api/README.md): `phoenix-rise-api`
- [`core/`](core/README.md): `phoenix-rise-core` account fetchers and
  transaction builders
- [`sdk/`](sdk/README.md): `phoenix-rise` facade, examples, integration tests,
  and packaged fixture copies
- [`cli/`](cli/README.md): `phoenix-rise-cli` CLI
- [`litesvm-test/`](litesvm-test/README.md): `phoenix-rise-litesvm-test` fixture
  crate for LiteSVM program tests

## Examples

Examples live in `sdk/examples/` and run from `rust/`:

```bash
cargo run -p phoenix-rise --example subscribe_l2_book --features ws -- SOL
cargo run -p phoenix-rise --example register_trader --features api -- <AUTHORITY_PUBKEY> --ref-code REF123
cargo run -p phoenix-rise --example subscribe_trader_state --features ws
cargo run -p phoenix-rise --example referral_activation_tx --features api,tx-builder,solana-keypair -- REFERRAL_CODE --trader-keypair-path ~/.config/solana/id.json
cargo run -p phoenix-rise --example builder_onboarding_tx --features api -- --fee-payer-keypair-path ~/.config/solana/id.json [--trader-authority <TRADER_AUTHORITY_PUBKEY>]
cargo run -p phoenix-rise --example onboard_trader_delegated --features api,tx-builder -- <TRADER_AUTHORITY>
cargo run -p phoenix-rise-cli -- --json market list
cargo run -p phoenix-rise-cli -- --json rpc account --address <ACCOUNT_PUBKEY> --account-type trader
```

`referral_activation_tx` calls `/v1/referral/activate-tx` for delegated
onboarding with a referral code. `builder_onboarding_tx` calls
`/v1/exchange/build-register-ixs` and `/v1/exchange/send-register-ixs` to
register a trader authority pubkey without a referral code, with only the fee
payer signing locally. `--fee-payer-keypair-path` defaults to
`~/.config/solana/id.json`; `--trader-authority` is optional and defaults to the
fee payer's pubkey.

## Basic Usage

```rust
use phoenix_rise::{
    api::{PhoenixHttpClient, PhoenixWSClient},
    core::PhoenixTxBuilder,
};
```

Docs: <https://docs.phoenix.trade/>
