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

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-types`](../types/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
