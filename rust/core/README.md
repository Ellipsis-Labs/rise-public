# Phoenix Rise Core

`phoenix-rise-core` provides off-chain account fetchers, order-ticket types, and
transaction builders that combine Phoenix API metadata with low-level
instruction builders.

Most users should start with [`phoenix-rise`](../sdk/README.md) and enable the
`core` feature. Protocol documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You need `PhoenixTxBuilder` without the SDK facade.
- You need RPC account fetch and decode helpers.
- You need event extraction adapters for RPC or Yellowstone transactions.

## Features

- `events`: enables event-parsing adapters via `phoenix-rise-events`.

## Load Consistent Trader State

Wrap an existing synchronous Solana `RpcClient` in `PhoenixAccountClient` and
call `fetch_trader_snapshot(&trader_address)`. The loader discovers the GTI/ATB
arenas, reads them with the trader account at one slot, and selects authoritative
hot or cold state, positions, and collateral. The returned `TraderSnapshot`
retains the slot; arena-count changes and incomplete snapshots return errors.
Custom account sources can implement `AccountSnapshotFetcher` for the same
loading and validation logic.

See the [snapshot example](../sdk/examples/fetch_spline_state.rs) and
[spline onboarding guide](../docs/spline-onboarding.md) for usage, commitment
selection, and the RPC account-count limit.

## Transaction errors

`phoenix_rise::core::transaction_errors::parse_transaction_error` combines the
resolved instructions you submitted with an RPC or simulation error and logs:

```rust,ignore
use phoenix_rise::core::transaction_errors::parse_transaction_error;

if let Some(error) = &simulation.value.err {
    let context = parse_transaction_error(
        &instructions,
        error,
        simulation.value.logs.as_deref(),
    );
    println!("{context:#?}");
}
```

The context preserves the top-level instruction index, program/instruction
identity, and original Solana instruction error. An aligned invocation trace
adds a failure hint with the rejecting program, custom code, IDL error name,
and message. Missing, truncated, or mismatched logs leave the hint absent.
Transaction-wide errors such as `BlockhashNotFound` return `None`.

`failure.hint` adds narrowly verified context without changing the native error.
For example, `ConditionalOrdersAccountUninitialized` requires Eternal to reject
with `InvalidAccountData` and its own loader diagnostic to identify an empty
`ConditionalOrderHeader`. Other invalid-account failures remain generic. A
client can use this hint to ensure TP/SL setup exists before rebuilding a
transaction; it is not permission to retry a transaction with unknown delivery.

The shared Rust/TypeScript corpus includes real program failures for self-trade,
IOC minimum fill, missing TP/SL setup, insufficient transferable collateral,
an uncreated transfer destination, and a child-account sweep requiring a signer.

Nested attribution is best effort: a caller can catch and later return the same
error. A CPI hint has no instruction name because runtime logs contain no CPI
instruction bytes. When those bytes are available, decode that instruction
separately with `phoenix_rise::ix::diagnostics::decode_instruction_error`.
These diagnostic hints do not prescribe retry behavior.

The shared-corpus integration test runs from a Rise source checkout. Crate
archives omit that test because its canonical fixture lives outside this crate;
the published library does not require the workspace corpus.

Diagnostics do not establish delivery or completion. Confirm delivery and inspect
the operation's outcome before deciding whether another submission is appropriate.
