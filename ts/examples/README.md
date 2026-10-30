# Rise TS Examples

Runnable TypeScript examples for `@ellipsis-labs/rise`.

Pass route-style symbols like `SOL` or `BTC` for `SYMBOL`, not `SOL-PERP`. The
ix-building example resolves `SOL` vs `SOL-PERP` from exchange metadata.

## Quick Start

### 01-http-client.ts

Minimal HTTP client setup mirroring `rust/sdk/examples/http_client.rs`.

```bash
bun examples/01-http-client.ts [TRADER_PUBKEY]
```

### 02-ws-fills.ts

Minimal WebSocket setup mirroring `rust/sdk/examples/subscribe_trades.rs`.

```bash
bun examples/02-ws-fills.ts [SYMBOL]
```

### 03-build-limit-order-ix.ts

Loads exchange metadata, builds a limit-order packet, and builds a
place-limit-order instruction.

```bash
bun examples/03-build-limit-order-ix.ts [AUTHORITY_PUBKEY] [SYMBOL]
```

Omitting `AUTHORITY_PUBKEY` defaults to the all-ones System Program address,
since instruction building only needs an authority address.

### 04-trader-state-store.ts

Creates a `TraderState` store from `createPhoenixClient(...)`, retains the
resource to subscribe via `client.streams.traderState(...)`, and reads live
store values.

```bash
bun examples/04-trader-state-store.ts [AUTHORITY_PUBKEY] [TRADER_PDA_INDEX] [--watch]
```

Omitting `AUTHORITY_PUBKEY` defaults to the all-ones System Program address.
Exits after the first live update unless `--watch` is passed (`Ctrl+C` to stop).

### 05-cancel-all-conditional-orders.ts

Loads a trader's conditional-orders account and batch-submits cancel
instructions for every active trigger leg.

```bash
bun examples/05-cancel-all-conditional-orders.ts <AUTHORITY_PUBKEY> [options]
```

Running without arguments prints safety guidance instead of submitting live
transactions.

### 06-flight-market-order.ts

Configures `createPhoenixClient(...)` with a builder authority and builds a
Flight-routed market-order instruction that credits builder fees to the builder
trader account. Builds the instruction only; the Rust counterpart in
`rust/sdk/examples/send_flight_market_order.rs` also submits it.

```bash
bun examples/06-flight-market-order.ts <BUILDER_AUTHORITY> <TRADER_AUTHORITY> <SYMBOL> <bid|ask> <NUM_BASE_LOTS> [PRICE_LIMIT_TICKS]
```

### 07-onboard-trader-delegated.ts

Loads a trader-onboarder keypair, derives its permission PDA from the current
risk authority, registers the trader account if needed, and submits the
delegated onboarding instruction.

```bash
bun examples/07-onboard-trader-delegated.ts <TRADER_AUTHORITY> [options]
```

Running without arguments prints safety guidance instead of submitting live
transactions.

### 08-delegated-trader-management-onboarding.ts

External team flow: Phoenix grants `user_key` a trader-onboarding permission
budget, and `user_key` signs the transaction that registers and onboards a
trader.

```bash
bun examples/08-delegated-trader-management-onboarding.ts <TRADER_AUTHORITY> [options]
```

Pass `--grant-manager-keypair-path` and `--grant-uses N` to run the Phoenix-side
`SetPermissionDelegated` grant before the onboarding transaction.

### 09-referral-activation-tx.ts

Builds and submits the `/v1/referral/activate-tx` transaction signed by the
trader authority keypair. Includes `register_trader` before delegated onboarding
when the trader account is missing, or delegated onboarding alone when it
already exists.

```bash
PHOENIX_API_URL=http://127.0.0.1:8080 PHOENIX_RPC_URL=<RPC_URL> \
bun examples/09-referral-activation-tx.ts <REFERRAL_CODE> [TRADER_KEYPAIR_PATH]
```

### 10-builder-onboarding-tx.ts

Registers and onboards a trader without a referral code via
`/v1/exchange/build-register-ixs` and `/v1/exchange/send-register-ixs`. Takes
the trader authority as a public key and signs locally only with the fee payer;
the API validates, adds the onboarder signature, simulates, and submits the
transaction. `--fee-payer-keypair-path` defaults to `~/.config/solana/id.json`;
`--trader-authority` defaults to the fee payer's public key.

```text
PHOENIX_API_URL=http://127.0.0.1:8080 PHOENIX_RPC_URL=<RPC_URL> \
bun examples/10-builder-onboarding-tx.ts \
  [--fee-payer-keypair-path <PATH>] \
  [--trader-authority <TRADER_AUTHORITY_PUBKEY>]
```

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
