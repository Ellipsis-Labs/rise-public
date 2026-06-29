# Phoenix Rise Math

`phoenix-rise-math` contains market math, margin/risk helpers, funding helpers,
price conversion utilities, and type-safe quantity wrappers.

Most users should start with [`phoenix-rise`](../sdk/README.md). Protocol
documentation lives at [docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You need price-to-tick or lot conversion helpers.
- You need offline margin, risk, or funding calculations.
- You want the source-of-truth Rise quantity wrapper types.

## Features

- `serde`: serializes integer-backed quantity wrappers as decimal strings and
  accepts strings or numbers on deserialize.
- `solana`: enables Solana pubkey support where required by helpers.

Decimal-backed helpers are part of the crate API and do not require a separate
feature.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-events`](../events/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-api`](../api/README.md)
