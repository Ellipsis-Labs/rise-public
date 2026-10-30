# Phoenix Rise Types

`phoenix-rise-types` provides owned API, websocket, trader, market, and request
DTOs used by the Rise clients.

Most users should start with [`phoenix-rise`](../sdk/README.md) and enable this
crate through `api`, `core`, or `types`. Protocol documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You need DTOs without HTTP or websocket clients.
- You need request and response types for a custom transport layer.
- You need OpenAPI schema derivations for API payloads.

## Import Style

Prefer module imports such as `phoenix_rise_types::exchange::ExchangeView`,
`phoenix_rise_types::trader_http::TraderView`, or
`phoenix_rise_types::ix::PlaceIsolatedMarketOrderRequest`. Use
`phoenix_rise_types::prelude::*` only in tools or tests that need the full DTO
set.

## Features

- `serde`: JSON serialization and deserialization for DTOs.
- `utoipa`: OpenAPI schemas for supported DTOs.

## Crate Links

[`phoenix-rise`](../sdk/README.md) | [`phoenix-rise-api`](../api/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
