# Phoenix Rise Math

`phoenix-rise-math` provides market math, margin and risk calculations, funding
helpers, price conversions, and type-safe quantity wrappers.

Most users should start with [`phoenix-rise`](../sdk/README.md). Protocol
documentation lives at [docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You convert between prices, ticks, and lots.
- You run offline margin, risk, or funding calculations.
- You need the canonical Rise quantity wrapper types.

## Features

- `serde`: serializes integer-backed quantity wrappers as decimal strings and
  deserializes from strings or numbers.
- `solana`: enables Solana pubkey support in helpers that require it.

Decimal-backed helpers are always enabled.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-events`](../events/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-api`](../api/README.md)
