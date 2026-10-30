# Phoenix Rise API

`phoenix-rise-api` provides typed Phoenix HTTP, auth, exchange-cache, Flight,
Hawkeye, and optional websocket clients.

Most users should start with [`phoenix-rise`](../sdk/README.md) and enable the
`api` feature. Protocol documentation lives at
[docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You need typed REST clients without the SDK facade.
- You need typed websocket clients via the `ws` feature.
- You need auth, session, or service-account login helpers.
- You build an off-chain service that does not need the full SDK facade.

## Common Entry Points

- `PhoenixHttpClient`
- `PhoenixWSClient` (`ws`)
- `PhoenixClient` (`ws`)
- `PhoenixFlightClient`

`PhoenixHttpClient::candles().get_candles_v2(...)` supports explicit millisecond
windows, cursor pagination, mark-price OHLC, external-source metadata,
partial-bar control, and finality. It defaults to 1,000 bars per page and
accepts page sizes up to 10,000. `get_candles(...)` stays on the legacy endpoint
to preserve its array response, millisecond timestamps, exchange-only default,
server-relative finalized window, and 2,500-bar server limit.

### Order History V2

Use `orders().get_user_order_history_v2(...)` for wallet-authority history or
`orders().get_trader_order_history_v2(...)` for a trader PDA. Both accept
`OrderHistoryV2QueryParams` and return flat packets, including TWAP children.
`get_user_organized_order_history_v2(...)` and
`get_trader_organized_order_history_v2(...)` return terminal TWAP parents with
nested children in `data.twap_orders`, plus standalone orders in
`data.single_orders`. The page limit counts parents and standalone orders, not
nested children.

All v2 methods are also available on `PhoenixHttpClient`. Treat cursors as
opaque and start without a cursor when switching views. Legacy Order History
methods are unchanged. Live `OrderHistoryDelta` rows retain optional
`parent_order_id` and `parent_order_type` fields; struct-literal callers must
initialize them (usually `None`).

## Crate Links

[`phoenix-rise`](../sdk/README.md) | [`phoenix-rise-types`](../types/README.md)
| [`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
