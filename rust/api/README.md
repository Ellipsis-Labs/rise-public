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

### Order History V2

Use `orders().get_user_order_history_v2(...)` for wallet-authority history or
`orders().get_trader_order_history_v2(...)` for an actual trader PDA. Both return
flat packets, including TWAP children, and accept `OrderHistoryV2QueryParams`.
The corresponding `get_user_organized_order_history_v2(...)` and
`get_trader_organized_order_history_v2(...)` methods return terminal TWAP parents
with nested children in `data.twap_orders`, plus `data.single_orders`.
The page limit counts parents and standalone orders, not nested children.

V2 methods are also available on `PhoenixHttpClient`. Keep cursors opaque and
start without a cursor when switching views. Existing legacy Order History
methods are unchanged. Live `OrderHistoryDelta` rows retain optional
`parent_order_id` and `parent_order_type`; callers constructing these rows with
struct literals must initialize the new fields (usually `None`).

## Crate Links

[`phoenix-rise`](../sdk/README.md) |
[`phoenix-rise-types`](../types/README.md) |
[`phoenix-rise-core`](../core/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
