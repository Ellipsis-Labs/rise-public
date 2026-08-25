# Rise Rust SDK Crates

`rise/rust` publishes a set of Rust crates that can be used independently or
through the facade SDK crate.

## Published Crates

- [`phoenix-rise`](sdk/README.md): facade crate imported as `phoenix_rise`
- [`phoenix-rise-accounts`](accounts/README.md): Phoenix account views,
  optional owned readers, PDA helpers, and discriminators
- [`phoenix-rise-ix`](ix/README.md): instruction builders, account metas, CPI
  helpers, and ix payloads
- [`phoenix-rise-math`](math/README.md): pure market, margin, risk, and
  quantity helpers
- [`phoenix-rise-events`](events/README.md): Phoenix market event batch parsing
  helpers
- [`phoenix-rise-types`](types/README.md): owned API, websocket, trader, and
  account snapshot types
- [`phoenix-rise-api`](api/README.md): HTTP, websocket, auth, cache, and
  transport clients
- [`phoenix-rise-core`](core/README.md): off-chain account fetchers and
  transaction builders
- [`phoenix-rise-cli`](cli/README.md): API, websocket, and RPC account
  inspection CLI
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
- `sdk`: account views, instruction builders, math, and domain types without
  API transport or RPC dependencies
- `api`: typed HTTP/auth clients and API route helpers
- `ws`: websocket clients and live trader-state helpers on top of `api`
- `tx-builder`: local transaction builder helpers and RPC-backed bracket flows
- `core`: compatibility alias for `tx-builder`
- `events`: market event parsing helpers plus core transaction instruction
  adapters for RPC and Yellowstone-style instruction streams
- `cpi`: program-friendly facade profile that exposes account byte decoders,
  instruction layouts, and Pinocchio CPI helpers without API, RPC, websocket,
  math, type DTO, or `solana-instruction` dependencies
- `types-sdk`: client-side owned state helpers
- `solana-keypair`, `ed25519-dalek`: compatibility aliases for `api`
- `opentelemetry`: API trace propagation and trace headers
- `serde`: JSON serialization for SDK account, instruction, and API DTOs
- PDA helpers in `phoenix-rise-accounts` / `phoenix-rise-ix`: available by
  default. On Solana targets these helpers use syscall-backed `solana-pubkey`.
- `cpi` on `phoenix-rise-ix`: Pinocchio CPI invoke helpers. This is additive;
  raw instruction data and explicit account metas do not require Pinocchio.
- `utoipa`: OpenAPI schema derivations for API types

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
- [`litesvm-test/`](litesvm-test/README.md): `phoenix-rise-litesvm-test`
  fixture crate for LiteSVM program tests

## Examples

Examples live under `sdk/examples/` and can be run from `rise/rust/`:

```bash
cargo run -p phoenix-rise --example subscribe_l2_book --features ws -- SOL
cargo run -p phoenix-rise --example register_trader --features api -- <AUTHORITY_PUBKEY> --access-code ACCESS123
cargo run -p phoenix-rise --example subscribe_trader_state --features ws
cargo run -p phoenix-rise --example referral_activation_tx --features api,tx-builder -- REFERRAL_CODE --trader-keypair-path ~/.config/solana/id.json
cargo run -p phoenix-rise --example builder_onboarding_tx --features api -- --fee-payer-keypair-path ~/.config/solana/id.json [--trader-authority <TRADER_AUTHORITY_PUBKEY>]
cargo run -p phoenix-rise --example onboard_trader_delegated --features api,tx-builder -- <TRADER_AUTHORITY>
cargo run -p phoenix-rise-cli -- --json market list
cargo run -p phoenix-rise-cli -- --json rpc account --address <ACCOUNT_PUBKEY> --account-type trader
```

`referral_activation_tx` demonstrates delegated onboarding with a referral code
through `/v1/referral/activate-tx`. `builder_onboarding_tx` demonstrates
registering a trader authority public key without a referral code, with only
the fee payer signing locally, through
`/v1/exchange/build-register-ixs` and `/v1/exchange/send-register-ixs`.
The fee-payer keypair path defaults to `~/.config/solana/id.json` and can be
overridden with `--fee-payer-keypair-path`. The `--trader-authority` argument
is optional and defaults to the fee payer's public key when omitted.

## Basic Usage

```rust
use phoenix_rise::{
    api::{PhoenixHttpClient, PhoenixWSClient},
    core::PhoenixTxBuilder,
};
```

Docs: <https://docs.phoenix.trade/>
