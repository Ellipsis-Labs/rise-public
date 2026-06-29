# Phoenix Rise Types

`phoenix-rise-types` contains owned API, websocket, trader, market, and request
DTOs used by the Rise clients.

Most users should start with [`phoenix-rise`](../sdk/README.md) and let the
facade enable this crate through `api`, `core`, or `types`. Protocol
documentation lives at [docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You only need DTOs and do not want HTTP/websocket clients.
- You want request/response shapes for your own transport layer.
- You need OpenAPI schema derivations for API payloads.

## Import Style

Prefer module imports such as `phoenix_rise_types::exchange::ExchangeView`,
`phoenix_rise_types::trader_http::TraderView`, or
`phoenix_rise_types::ix::PlaceIsolatedMarketOrderRequest`. Use
`phoenix_rise_types::prelude::*` only when a tool or test intentionally wants
the broad DTO set.

## Features

- `serde`: JSON serialization and deserialization for DTOs.
- `utoipa`: OpenAPI schemas for supported DTOs.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-api`](../api/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
