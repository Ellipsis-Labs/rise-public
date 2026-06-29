# Phoenix Rise Core

`phoenix-rise-core` contains off-chain account fetchers, order-ticket types, and
transaction builders built from Phoenix API metadata plus low-level instruction
builders.

Most users should start with [`phoenix-rise`](../sdk/README.md) and enable the
`core` feature. Protocol documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You want `PhoenixTxBuilder` without importing the facade.
- You need account fetch/decode helpers for RPC account data.
- You want event extraction adapters for RPC or Yellowstone-style transactions.

## Features

- `events`: enables event parsing adapters through `phoenix-rise-events`.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-api`](../api/README.md) |
[`phoenix-rise-ix`](../ix/README.md) |
[`phoenix-rise-accounts`](../accounts/README.md)
