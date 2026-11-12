# Onboard a spline integration with Rise

Use the public [Rise SDK](https://github.com/Ellipsis-Labs/rise-public) for
account decoding, event indexing, and supported instruction builders. The
read-only Global Trader Index (GTI) and Active Trader Buffer (ATB) views let an
integration inspect active trader state without maintaining a separate account
layout package. These views do not expose the protocol's account mutation
implementation.

Start with the runnable examples in this repository. They read state or parse
events and do not place quotes. Spline registration, signing authority, and
funding must be configured for the intended deployment before sending quote
updates.

## Load a trader snapshot

Clone the public repository and work from its `rust/` directory. Run:

```bash
PHOENIX_RPC_URL=https://your-solana-rpc.example.com \
  cargo run -p phoenix-rise --example fetch_spline_state --features tx-builder -- \
  <TRADER_ACCOUNT_PUBKEY>
```

Pass the **trader account PDA**, not its wallet authority. Set `PHOENIX_ENV=beta`
for the beta Phoenix deployment and supply the corresponding RPC endpoint.
Do not mix account addresses or program IDs from different deployments.

The [example](../sdk/examples/fetch_spline_state.rs) uses the reusable
`PhoenixAccountClient::fetch_trader_snapshot` loader. It prints the snapshot
slot, selected trader state, positions, quote collateral, spot collateral
(including native SOL), and locked deposit. It handles both hot and cold
traders without merging stale copies of their state.

Enable `core` (or `tx-builder`) on `phoenix-rise` to use the loader in your
application. Pass your existing synchronous Solana `RpcClient` to the account
client; its configured commitment applies to every read:

```rust
use phoenix_rise::core::{PhoenixAccountClient, TraderSnapshot, TraderSnapshotError};
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;

fn load_trader(
    rpc: RpcClient,
    trader: &Pubkey,
) -> Result<TraderSnapshot, TraderSnapshotError> {
    let client = PhoenixAccountClient::new(rpc);
    let snapshot = client.fetch_trader_snapshot(trader)?;
    println!("slot={}: {:?}", snapshot.context_slot(), snapshot.state());
    println!("positions: {:?}", snapshot.positions());
    println!("spot collateral: {:?}", snapshot.spot_collaterals());
    Ok(snapshot)
}
```

The loader discovers arena counts, then fetches the trader account, global
configuration, and **all** GTI/ATB arenas together in one contextual response.
It validates ownership, expected addresses, account layouts, and hot-state
consistency. The final read cannot precede the discovery slot, and its returned
slot is available through `context_slot()`. If arena counts change, repeat the
complete load; a missing account or invalid layout is an error, not an empty
portfolio.

The RPC implementation rejects requests exceeding `getMultipleAccounts`'s
100-account limit. For larger deployments or another account source, implement
`AccountSnapshotFetcher` alongside the existing `AccountDataFetcher` trait.
The batch must come from one bank at or after the requested minimum slot;
sequential RPC reads cannot satisfy this contract.

## Inspect raw account buffers

For use in another Rust application, enable `accounts` on `phoenix-rise`, or
depend directly on `phoenix-rise-accounts`. The borrowed reader API accepts
full account byte buffers, including each account's discriminator and header:

```rust
use phoenix_rise::accounts::active_trader_buffer::ActiveTraderBuffer;
use phoenix_rise::accounts::global_trader_index::GlobalTraderIndex;
use solana_pubkey::Pubkey;

fn inspect(
    trader: &Pubkey,
    gti_accounts: &[Vec<u8>],
    atb_accounts: &[Vec<u8>],
) -> Result<(), Box<dyn std::error::Error>> {
    let gti = GlobalTraderIndex::try_from_account_buffers(
        gti_accounts.iter().map(Vec::as_slice),
    )?;
    let atb = ActiveTraderBuffer::try_from_account_buffers(
        atb_accounts.iter().map(Vec::as_slice),
    )?;
    if let Some(trader_id) = gti.index(trader) {
        for (position_id, position) in atb.iter_positions(trader_id) {
            println!("{position_id:?}: {position:?}");
        }
    }
    Ok(())
}
```

Supply all initialized arenas and keep their buffers alive while using the
views. Validate each account's owner and expected PDA before calling a byte
decoder; serialized bytes alone cannot prove which account supplied them.
Refresh the discovery step if an arena is missing or the header's arena count
changes during loading.

The numeric GTI index identifies an active trader entry in that snapshot; it
is not a permanent trader identifier. Use the trader PDA as the durable key
and resolve its GTI index again after reloading. Pair GTI and ATB reads from
the same snapshot. The snapshot loader applies state selection automatically.
For a hot trader present in GTI, use GTI state and ATB
entries as the authoritative active state. For a cold trader, read the trader
account. Do not add hot and cold copies together or fill a missing hot entry
from a potentially stale cold copy. An absent GTI entry does not prove a zero
balance. A complete portfolio and margin calculation must select the correct
state source and include all collateral, market metadata, prices, and funding.

TypeScript users can call `decodeGlobalTraderIndex(buffers)` and
`decodeActiveTraderBuffer(buffers)` from `@ellipsis-labs/rise`. The
[`read-spline-state` example](../../ts/examples/read-spline-state.ts) accepts
binary account dumps. From the repository's `ts/` directory:

```bash
bun examples/read-spline-state.ts \
  --gti gti-header.bin --gti gti-arena-1.bin \
  --atb atb-header.bin --atb atb-arena-1.bin \
  --trader <TRADER_ACCOUNT_PUBKEY>
```

Repeat `--gti`/`--atb` for every registered arena in order; omit overflow
arguments when a tree has only its header account. Capture both trees from a
consistent source and verify their owner/address metadata before saving the
raw dumps. The example cannot recover that metadata from bytes alone.

## Inspect quotes before sending updates

Use the existing `SplineCollection` reader and market metadata to identify
your trader's registered spline. The
[`fetch_on_chain_accounts` example](../sdk/examples/fetch_on_chain_accounts.rs)
shows general account fetching, and
[`SplineCollection`](../accounts/src/spline_collection.rs) exposes quote and
region inspection.

Check each side's base width, region offsets and density, filled amounts,
`user_update_slot`, and region lifespan. Registration or a nonzero mid-price
alone does not mean a spline has executable liquidity. Combine the spline's
`is_active()` and `is_enabled()` checks with region
`is_active(current_slot, spline.user_update_slot())` and `is_empty()` checks.
The `active_bid_regions()`/`active_ask_regions()` methods select the current
region window; they do not filter expired or fully filled regions.

Prices and region offsets use ticks; quantities and position limits use base
lots. Read conversion parameters from the market's metadata rather than
assuming a fixed number of decimals. Preserve the separate price and parameter
sequence numbers when using ordered updates, and account for expiration while
the quoting process is disconnected.

Supported builders live in [`phoenix_rise::ix::spline`](../ix/src/spline.rs).
`SplineUpdateAccounts` identifies the signer, trader account, and spline
collection. Choose a price, parameter, metadata, or position-limit update for
the change you intend. Building an instruction does not grant the signer
permission to execute it. Reconcile the resulting on-chain state and events
after execution before considering an update applied.

## Index fills and reconcile

Follow [Index Phoenix transaction events with Rise](event-indexing.md) to
consume `LogEventLengths` and `Log` CPI data. Run its offline example before
connecting a transaction feed, and use the Rise CLI to inspect a real signature.
Reject failed transactions, preserve complete CPI ancestry, retain decode
failures for replay, and checkpoint only after a transaction is persisted.

Filter `SplineFilled` events by the maker **trader account**, retaining the
market header, PnL events, and execution context. Use the public
[`fill_id` helpers](event-indexing.md#match-phoenix-api-fill-ids) with the parser's
Phoenix instruction ordinal to match PnL events to API trade-history fill IDs.
Periodically compare your event-derived
state with a consistent GTI/ATB and trader-account snapshot. The public HTTP
trade history and trader-state WebSocket APIs are also useful reconciliation
sources; see the [Rust SDK guide](../README.md) and
[TypeScript examples](../../ts/examples/README.md).

The account and indexing examples provide a starting point for loading state
and observing execution. A quoting application must additionally manage its
price feed, inventory limits, transaction submission, retries, quote expiration,
and recovery after missed events.
