# Rise TS Examples

This directory contains runnable TypeScript examples for `@ellipsis-labs/rise`.

When an example asks for `SYMBOL`, use the route-style symbol like `SOL` or
`BTC`, not `SOL-PERP`. The ix-building example resolves `SOL` vs `SOL-PERP`
from exchange metadata for you.

## Quick Start

### 01-http-client.ts

Minimal HTTP client setup mirroring `rise/rust/sdk/examples/http_client.rs`.

```bash
bun examples/01-http-client.ts [TRADER_PUBKEY]
```

### 02-ws-fills.ts

Minimal WebSocket setup mirroring `rise/rust/sdk/examples/subscribe_trades.rs`.

```bash
bun examples/02-ws-fills.ts [SYMBOL]
```

### 03-build-limit-order-ix.ts

Creates a client, loads exchange metadata, builds a limit-order packet, and
builds a place-limit-order instruction.

```bash
bun examples/03-build-limit-order-ix.ts [AUTHORITY_PUBKEY] [SYMBOL]
```

If `AUTHORITY_PUBKEY` is omitted, the example uses the all-ones System Program
address because instruction building only needs an authority address.

### 04-trader-state-store.ts

Creates a `TraderState` store from `createPhoenixClient(...)`, retains the
resource so it subscribes through `client.streams.traderState(...)`, and reads
live values from the store.

```bash
bun examples/04-trader-state-store.ts [AUTHORITY_PUBKEY] [TRADER_PDA_INDEX] [--watch]
```

If `AUTHORITY_PUBKEY` is omitted, the example uses the all-ones System Program
address and exits after the first live update. Pass `--watch` to keep the
subscription open until `Ctrl+C`.

### 05-cancel-all-conditional-orders.ts

Loads a trader's conditional-orders account, builds a cancel instruction for
every active trigger leg, and submits them in batches.

```bash
bun examples/05-cancel-all-conditional-orders.ts <AUTHORITY_PUBKEY> [options]
```

Running this example with no arguments prints safety guidance instead of
submitting live transactions.

### 06-flight-market-order.ts

Configures `createPhoenixClient(...)` with a builder authority and builds a
Flight-routed market-order instruction so builder fees would be credited to the
configured builder trader account. This example only builds the instruction; the
runnable Rust counterpart in `rise/rust/sdk/examples/send_flight_market_order.rs`
also submits it.

```bash
bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]
```

### 07-onboard-trader-delegated.ts

Loads a trader-onboarder keypair, derives its permission PDA from the current
risk authority, registers the target trader account if needed, and submits the
delegated onboarding instruction.

```bash
bun examples/07-onboard-trader-delegated.ts <TRADER_AUTHORITY> [options]
```

Running this example with no arguments prints safety guidance instead of
submitting live transactions.

## Larger Demos

### phoenix-client-example.ts

Broader HTTP API walkthrough covering markets, trader views, history routes, and
PnL queries.

```bash
bun examples/phoenix-client-example.ts [TRADER_PUBKEY]
```

### phoenix-ws-example.ts

Interactive WebSocket demo covering the built-in stream adapters.

```bash
bun examples/phoenix-ws-example.ts <CHANNEL> [ARGS...]
```
