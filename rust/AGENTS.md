# Rust SDK Guide

Start with [README.md](./README.md). It describes the published crate, feature
flags, source layout, and example commands for `rise/rust`.

Current crate layout:

- Published crate name: `phoenix-rise`
- Rust import path: `phoenix_rise`
- Workspace crates: `accounts/`, `ix/`, `math/`, `types/`, `api/`, `sdk/`
- Facade source root: `sdk/src/`
- Examples: `sdk/examples/`

Use the SDK examples as the main reference set:

- [sdk/examples/](./sdk/examples)
- [sdk/examples/http_client.rs](./sdk/examples/http_client.rs)
- [sdk/examples/register_trader.rs](./sdk/examples/register_trader.rs)
- [sdk/examples/send_limit_order.rs](./sdk/examples/send_limit_order.rs)
- [sdk/examples/send_market_order.rs](./sdk/examples/send_market_order.rs)
- [sdk/examples/subscribe_trader_state.rs](./sdk/examples/subscribe_trader_state.rs)
- [sdk/examples/ws_debug_cli.rs](./sdk/examples/ws_debug_cli.rs)

Run commands from `rise/rust/`:

```bash
cargo build
cargo test
cargo run -p phoenix-rise --example subscribe_l2_book --features api -- SOL
cargo run -p phoenix-rise --example subscribe_trader_state --features api
```
