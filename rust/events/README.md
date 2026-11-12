# Phoenix Rise Events

`phoenix-rise-events` parses Phoenix market event batches from `Log` and
`LogEventLengths` instruction payloads.

Most users should start with [`phoenix-rise`](../sdk/README.md) and enable the
`events` feature. Protocol documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

For an end-to-end integration, read
[Index Phoenix transaction events with Rise](../docs/event-indexing.md).
It covers RPC and Yellowstone instruction adaptation, CPI ancestry, length
framing, failed transactions, replay, and checkpointing. A runnable offline
example lives in
[`index_transaction_events.rs`](../sdk/examples/index_transaction_events.rs):

```bash
# Run from the rust/ workspace.
cargo run -p phoenix-rise --example index_transaction_events --features events,serde
```

`Log` and `LogEventLengths` are self-CPI instruction data, not RPC
`logMessages`. Keep the complete instruction execution tree and filter by the
expected Phoenix program ID. The `phoenix-rise-core::events` adapters group
logger instructions by their parent Phoenix invocation.

## Use This Crate When

- You parse market events from transaction instructions.
- You need skipped-byte diagnostics via `parse_with_errors`.
- You need event DTOs without internal Phoenix program crates.
- You need API-compatible fill IDs derived from PnL events.

## Decode A Logger Group

Pass full instruction bytes to `PhoenixLogInstruction::from_instruction_data`,
or payload bytes after the eight-byte discriminant to `PhoenixLogInstruction::new`.
Each group must belong to one Phoenix invocation and retain execution order:

```rust
use phoenix_rise_events::{PhoenixLogInstruction, parse_with_errors};

fn decode_logger_group(logger_instruction_data: &[Vec<u8>]) {
    let logs = logger_instruction_data
        .iter()
        .filter_map(|data| PhoenixLogInstruction::from_instruction_data(data));
    let parsed = parse_with_errors(logs);
    for failure in &parsed.failures {
        eprintln!("{}", failure.error);
    }
    // Retain the input and retry before committing a partial state transition.
    if !parsed.failures.is_empty() {
        return;
    }
    for event in parsed.events {
        println!("{}: {event:?}", event.event_type());
    }
}
```

The caller must validate the program ID, transaction success, and completeness
of the source metadata. `parse` is a forgiving API that discards diagnostics;
stateful indexers should use `parse_with_errors` and define a recovery policy.

## Reconcile Fill IDs

`fill_id::generate_fill_id_from_pnl` derives the Phoenix API fill ID from a
transaction signature, the parser's `phoenix_instruction_index`, and a PnL
event. Use the Phoenix ordinal, including eventless and nested invocations;
the top-level transaction index and stack path are different identifiers.
`fill_id::collect_parsed_pnls_with_fill_id` also retains caller-supplied event
context. See the [fill-ID guide](../docs/event-indexing.md#match-phoenix-api-fill-ids)
and the indexing example for complete usage.

## Features

- `serde`: JSON serialization for parsed event types. `phoenix-rise-math`
  quantity wrappers serialize as strings and deserialize from strings or
  numbers.

## Crate Links

[`phoenix-rise`](../sdk/README.md) | [`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-math`](../math/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
