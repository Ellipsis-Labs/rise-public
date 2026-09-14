# Phoenix Rise API

`phoenix-rise-api` contains the typed Phoenix HTTP, auth, exchange cache,
Flight, Hawkeye, and optional websocket transport clients.

Most users should depend on [`phoenix-rise`](../sdk/README.md) with the `api`
feature instead of importing this crate directly. Protocol documentation lives
at [docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You want typed REST clients without the facade.
- You want typed websocket clients by enabling the `ws` feature.
- You need auth/session helpers or service account login.
- You are building an off-chain service that does not need the full SDK facade.

## Common Entry Points

- `PhoenixHttpClient`
- `PhoenixWSClient` (`ws`)
- `PhoenixClient` (`ws`)
- `PhoenixFlightClient`

`PhoenixHttpClient::candles().get_candles_v2(...)` exposes explicit
millisecond windows, cursor pagination, mark-price OHLC, external-source
metadata, partial-bar control, and finality. The existing
`get_candles(...)` method keeps its original array response and millisecond
timestamps. It remains on the legacy endpoint to preserve its exchange-only
default, server-relative finalized window, and 2,500-bar server limit. The v2
method defaults to 1,000 bars per page and accepts explicit page sizes up to
10,000.

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-types`](../types/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
