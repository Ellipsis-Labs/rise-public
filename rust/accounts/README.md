# Phoenix Rise Accounts

`phoenix-rise-accounts` provides Phoenix account discriminators, borrowed
account-data views, PDA helpers, and optional serde-friendly owned decoding.

Most users should start with [`phoenix-rise`](../sdk/README.md). Protocol
documentation lives at [docs.phoenix.trade](https://docs.phoenix.trade/).

## Use This Crate When

- You decode account bytes from RPC or an indexer.
- You need program-friendly account layouts without API or RPC clients.
- You need PDA helpers alongside the account layouts.

## Features

- `serde`: JSON-friendly output for account views, with pubkeys encoded as
  base58 strings.

PDA helpers are always enabled. `solana-pubkey` uses the program-address syscall
on Solana targets and curve derivation on host builds.

## Global Trader Index And Active Trader Buffer

`global_trader_index::GlobalTraderIndex` and
`active_trader_buffer::ActiveTraderBuffer` provide immutable views of complete
GTI/ATB account sets. Pass full account buffers in arena-index order, header
account first. No account mutation or allocator interface is exposed.

```rust
use phoenix_rise_accounts::active_trader_buffer::ActiveTraderBuffer;
use phoenix_rise_accounts::global_trader_index::GlobalTraderIndex;
use solana_pubkey::Pubkey;

fn active_positions(
    trader: &Pubkey,
    gti_accounts: &[Vec<u8>],
    atb_accounts: &[Vec<u8>],
) -> Result<(), Box<dyn std::error::Error>> {
    let gti = GlobalTraderIndex::try_from_account_buffers(
        gti_accounts.iter().map(Vec::as_slice),
    )?;
    let atb = ActiveTraderBuffer::try_from_account_buffers(
        atb_accounts.iter().map(Vec::as_slice),
    )?;
    if let Some(trader_id) = gti.index(trader) {
        for (id, position) in atb.iter_positions(trader_id) {
            println!("{id:?}: {position:?}");
        }
    }
    Ok(())
}
```

The caller verifies account owners and PDAs and supplies GTI/ATB from the same
snapshot. A GTI index can be recycled after eviction; persist trader pubkeys,
not indices. ATB position iterators exclude collateral and reserved entries.
For a hot trader, use GTI/ATB as the active state source; for a cold trader, use
the trader account. Do not add their overlapping copies together. Use the
separate collateral accessors when assembling a portfolio. See the
[onboarding guide](../docs/spline-onboarding.md) and runnable
[`fetch_spline_state` example](../sdk/examples/fetch_spline_state.rs).

## Crate Links

[`phoenix-rise`](../sdk/README.md) | [`phoenix-rise-ix`](../ix/README.md) |
[`phoenix-rise-math`](../math/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
