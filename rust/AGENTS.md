# Rust SDK Guide

Start with [README.md](./README.md). It describes the published crate, feature
flags, source layout, and example commands for `rise/rust`.

Current crate layout:

- Published crate name: `phoenix-rise`
- Rust import path: `phoenix_rise`
- Main source root: `src/`
- Examples: `examples/`

Use the SDK examples as the main reference set:

- [examples/](./examples)
- [examples/http_client.rs](./examples/http_client.rs)
- [examples/register_trader.rs](./examples/register_trader.rs)
- [examples/send_limit_order.rs](./examples/send_limit_order.rs)
- [examples/send_market_order.rs](./examples/send_market_order.rs)
- [examples/subscribe_trader_state.rs](./examples/subscribe_trader_state.rs)
- [examples/ws_debug_cli.rs](./examples/ws_debug_cli.rs)

Run commands from `rise/rust/`:

```bash
cargo build
cargo test
cargo run -p phoenix-rise --example subscribe_l2_book -- SOL
cargo run -p phoenix-rise --example subscribe_trader_state --features solana-keypair
```
