# Rise Rust SDK

`rise/rust` publishes a single Rust library crate for interacting with Phoenix.

## Published Crate

- Crate name: `phoenix-rise`
- Rust import path: `phoenix_rise`

```toml
[dependencies]
phoenix-rise = "0.1.15"
```

Optional features:

- `solana-keypair` for examples and auth/signing flows that use local Solana keypairs
- `ed25519-dalek` for service-account signing helpers
- `rust_decimal` for decimal-backed helpers

## Changelog

Release notes are maintained in the public Rise repository:
<https://github.com/Ellipsis-Labs/rise-public/blob/master/rust/CHANGELOG.md>.

## Source Layout

The published `phoenix-rise` crate uses the standard single-crate Cargo layout:

- high-level HTTP, WebSocket, auth, and transaction-building flows in `src/`
- API wire types in `src/types/`
- instruction builders in `src/ix/`
- market and margin math in `src/math/`

Examples live in `examples/` and integration tests live in `tests/`.

## Examples

Examples that use a local Solana keypair require `solana-keypair`:

```bash
cargo run -p phoenix-rise --example subscribe_trader_state --features solana-keypair
cargo run -p phoenix-rise --example onboard_trader_delegated --features solana-keypair -- <TRADER_AUTHORITY>
cargo run -p phoenix-rise --example delegated_trader_management_onboarding --features solana-keypair -- <TRADER_AUTHORITY>
```

Examples that do not need that feature can be run directly:

```bash
cargo run -p phoenix-rise --example subscribe_l2_book -- SOL
cargo run -p phoenix-rise --example register_trader -- <AUTHORITY_PUBKEY> --access-code ACCESS123
```

## Basic Usage

```rust
use phoenix_rise::{PhoenixHttpClient, PhoenixWSClient, PhoenixTxBuilder};
```

Docs: <https://docs.phoenix.trade/>
