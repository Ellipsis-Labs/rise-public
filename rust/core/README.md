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

## Crate Links

[`phoenix-rise`](../sdk/README.md) | [`phoenix-rise-api`](../api/README.md) |
[`phoenix-rise-ix`](../ix/README.md) |
[`phoenix-rise-accounts`](../accounts/README.md)
