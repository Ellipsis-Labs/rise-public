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

## Crate Links

[`phoenix-rise`](../sdk/README.md) | [`phoenix-rise-api`](../api/README.md) |
[`phoenix-rise-ix`](../ix/README.md) |
[`phoenix-rise-accounts`](../accounts/README.md)
