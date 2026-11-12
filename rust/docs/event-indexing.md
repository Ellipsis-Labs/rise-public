# Index Phoenix transaction events with Rise

Phoenix emits market events through self-CPI instructions named `LogEventLengths`
and `Log`. Read their **instruction data** from executed transactions. RPC
`logMessages` and WebSocket `logsSubscribe` text are not the event payloads;
a logs subscription can notify you of a signature to fetch separately.

The Rust SDK already includes both the event decoder and transaction adapters:

| Input                                                          | Public API                                                                        |
| -------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Full executed instructions with CPI paths                      | `phoenix_rise::core::events::parse_events_from_executed_instructions_with_errors` |
| RPC top-level instructions and inner groups with `stackHeight` | `phoenix_rise::core::events::parse_events_from_rpc_instructions_with_errors`      |
| Logger instructions already grouped by Phoenix invocation      | `phoenix_rise::events::parse_with_errors`                                         |

Enable `events` on `phoenix-rise`, plus `serde` for JSON output. Consumers that
already group logger instructions can depend directly on `phoenix-rise-events`
without the RPC/client stack. These are Rust APIs; TypeScript consumers can use
Rise's typed HTTP/WebSocket trade streams when they do not need raw CPI indexing.

## Run the examples

From the public [Rise repository](https://github.com/Ellipsis-Labs/rise-public),
change into `rust/` (or `rise/rust/` in the Phoenix source tree).

Run the offline CPI example without credentials:

```bash
cargo run -p phoenix-rise --example index_transaction_events --features events,serde
```

The [example source](../sdk/examples/index_transaction_events.rs) creates a
synthetic `SlotContext` and `PnL` events, frames them with a
`LogEventLengths`/`Log` pair, attaches both CPIs to their parent instruction,
and prints indexed events with the PnL event's derived fill ID.
It also accepts a normalized transaction JSON file:

```bash
cargo run -p phoenix-rise --example index_transaction_events --features events,serde -- transaction.json
```

The example input is an integration boundary, **not** an RPC response schema:

```json
{
  "signature": "TRANSACTION_SIGNATURE",
  "slot": 123,
  "succeeded": true,
  "phoenixProgramId": "PHOENIX_PROGRAM_PUBKEY",
  "instructions": [
    {
      "programId": "PROGRAM_PUBKEY",
      "dataBase64": "BASE64_FULL_INSTRUCTION_DATA",
      "stackPath": [0]
    }
  ]
}
```

Populate `succeeded` from transaction execution metadata, not from the presence
of log instructions. Include every executed instruction in execution order.
Public keys are base58 strings; `dataBase64` includes the instruction
discriminant. The example rejects failed transactions and reported decoder
failures before producing output. It does not implement streaming, durable
storage, or finality tracking.

For a real transaction, use the existing Rise CLI RPC adapter:

```bash
cargo run -p phoenix-rise-cli --bin phoenix-rise -- \
  --rpc-url "$PHOENIX_RPC_URL" --json rpc parse-tx \
  --signature <TRANSACTION_SIGNATURE> --with-errors
```

Pass `--program-id <PHOENIX_PROGRAM_PUBKEY>` to `parse-tx` for another deployment.
The CLI is an inspection tool and can show events emitted during failed
transactions. Check the transaction status before using its output as committed
state. Its output is not the normalized input schema above.

## Adapt RPC or Yellowstone transactions

1. Choose the expected Phoenix program ID and commitment for your deployment.
   Discard failed transactions for committed-state indexing. Keep raw data for
   diagnostics separately if needed.
2. Request complete transaction metadata, including inner instructions. For
   versioned transactions, resolve the combined account-key list in this order:
   static message keys, loaded writable keys, then loaded readonly keys. Resolve
   every `programIdIndex` against that list. Reject missing lookup addresses or
   out-of-range indices; do not silently drop instructions.
3. Decode instruction bytes according to the transport. Compiled instruction
   data in Solana RPC JSON is base58; a binary transaction message or Yellowstone
   compiled instruction already has bytes. The example's normalized file uses
   base64 deliberately, so decode the transport before converting it.
4. Preserve execution order and CPI ancestry. For the RPC adapter, include all
   top-level instructions, each inner group's original top-level `index`, and
   each inner instruction's `stackHeight`. Include other programs too, since
   they occupy positions in the call tree. The adapter filters by Phoenix
   program ID after reconstructing paths.
5. For the executed-instruction adapter, a top-level instruction has path `[i]`.
   Its first CPI has path `[i, 0]`, the next sibling `[i, 1]`, and the first
   child of that CPI `[i, 1, 0]`. Do not substitute a flat inner-instruction
   offset for this path. If the source lacks stack heights, recover the call
   tree before relying on exact attribution; the RPC adapter's legacy fallback
   cannot recover missing nested ancestry.
6. Use the `with_errors` entry point, inspect `orphan_logs` and each
   instruction's `failures`, and retain the original transaction for replay.
   Missing metadata or an empty decode does not establish that a transaction
   had no events.

The adapters accept borrowed views, so retain the decoded instruction buffers
until parsing finishes. The CLI's [RPC implementation](../cli/src/commands/rpc.rs)
shows the account-key and inner-instruction conversion.

## Understand the framing

Both instructions begin with their eight-byte discriminant. The payload after
that discriminant is:

| Instruction       | Payload                                                                                                    |
| ----------------- | ---------------------------------------------------------------------------------------------------------- |
| `LogEventLengths` | little-endian `u32 batch_index`, `u32 length_count`, then that many little-endian `u16` event byte lengths |
| `Log`             | little-endian `u32 batch_index`, `u32 event_count`, then concatenated Borsh event bytes                    |

Use the exported `LOG_INSTRUCTION_TAG` and `LOG_EVENT_LENGTHS_INSTRUCTION_TAG`
instead of maintaining duplicate constants. `PhoenixLogInstruction::from_instruction_data`
accepts full instruction bytes and strips the discriminant. Its `new` constructor
expects the payload **after** the discriminant.

Pair the length batch and event batch within the same Phoenix invocation.
Batch indices can repeat across invocations and transactions; they are not
global identifiers. Preserve logger execution order. The transaction adapters
perform per-invocation grouping before calling the event decoder.

Length framing lets the decoder isolate an unknown or malformed event and
continue decoding later frames. `parse_with_errors` exposes skipped bytes and
failures; `parse` returns only events that decoded. For an accounting or position
indexer, retain and quarantine a transaction with failures until it can be
decoded and replayed. Treating a partial decode as a complete state transition
can lose fills. The decoder also supports historical unframed logs when an
invocation contains no `LogEventLengths`; keep both CPI kinds in modern input.

## Store events and reconcile state

Keep the transaction signature, slot, instruction path, and event ordinal with
each event. Within a fully decoded transaction, `(signature, instruction path,
event ordinal)` provides a replay deduplication key. Commit the event rows and
the transaction checkpoint atomically. A consumer that elects to skip unknown
events needs a separate identity scheme that preserves raw frame ordinals;
enumerating only decoded events can shift indices after an SDK upgrade.

Process transactions in ledger order. Choose finalized input or track forks
and rollbacks for lower-latency commitments. On reconnect, backfill from the
durable checkpoint and deduplicate the overlap. A signature notification alone
is not a complete ingestion feed.

Spline makers can match `MarketEvent::SplineFilled` by its `maker` trader
account. Preserve surrounding `Header` events for market context and retain
funding, collateral, and other position-affecting events when maintaining
balances. A fill-only stream is not a complete trader state replayer. Periodically
reconcile against account snapshots using the
[spline onboarding guide](spline-onboarding.md), especially after gaps or decoder
failures.

## Match Phoenix API fill IDs

Use `phoenix_rise::events::fill_id` to reproduce Phoenix API trade-history fill
IDs from CPI-indexed `PnL` events. The helpers preserve the UUID v3 namespace
and byte encoding used by the legacy SDK. They accept the transaction
signature, the emitting instruction's **Phoenix instruction ordinal**, and
the PnL event's trader PDA, asset ID, base lots before/after, and realized PnL
in quote lots. Funding payment and virtual quote lots are not hash inputs.
Keep the original signed integer lot values; do not convert to UI amounts.

The transaction adapters expose this ordinal as
`ParsedInstructionEvents::phoenix_instruction_index`. It counts every
non-logger Phoenix instruction in execution order, including instructions
without events and nested Phoenix invocations. Other programs and the `Log`
and `LogEventLengths` CPIs do not consume ordinals. A top-level index, stack
path, event index, or count of only fill-producing instructions gives a
different fill ID.

```rust
use phoenix_rise::core::events::ParsedTransactionEvents;
use phoenix_rise::events::{MarketEvent, fill_id::generate_fill_id_from_pnl};

fn print_fill_ids(
    signature: &str,
    parsed: &ParsedTransactionEvents<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Call only after checking transaction success, orphan logs, and failures.
    for instruction in &parsed.instructions {
        let index = i32::try_from(instruction.phoenix_instruction_index)?;
        for event in &instruction.events {
            if let MarketEvent::PnL(pnl) = event {
                let fill_id = generate_fill_id_from_pnl(signature, index, pnl)?;
                println!("{fill_id}: {pnl:?}");
            }
        }
    }
    Ok(())
}
```

`generate_fill_id_v3` accepts individual base58 and lot-value inputs, and
`generate_fill_id_v3_from_bytes` accepts decoded signature and trader bytes.
`collect_parsed_pnls_with_fill_id` attaches an `EventParserCtx` and fill ID to
each PnL event in an invocation. If supplying context yourself, retain the
same Phoenix ordinal even for nested CPIs. Full ancestry is required to
attribute a logger to its emitting invocation; the RPC fallback for missing
stack heights cannot guarantee nested fill-ID parity.

The [indexing example](../sdk/examples/index_transaction_events.rs) emits
`phoenixInstructionIndex` on each row and `fillId` on PnL rows. Keep these
alongside the transaction path and event ordinal: the fill ID reconciles the
trade-history record, while the latter fields identify individual indexed
events. Retain PnL events alongside `SplineFilled` or `OrderFilled` events;
the fill-event payload alone does not contain all fill-ID inputs.
