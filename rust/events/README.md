# Phoenix Rise Events

`phoenix-rise-events` parses Phoenix market event batches from `Log` and
`LogEventLengths` instruction payloads.

Most users should start with [`phoenix-rise`](../sdk/README.md) and enable the
`events` feature. Protocol documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You need to parse market events from transaction instructions.
- You want skipped-byte diagnostics through `parse_with_errors`.
- You want event DTOs without importing internal Phoenix program crates.

## Features

- `serde`: JSON serialization for parsed event types. Quantity wrappers from
  `phoenix-rise-math` serialize as strings and deserialize from strings or
  numbers.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-math`](../math/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
