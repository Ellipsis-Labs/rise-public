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

## Crate Links

[`phoenix-rise`](../sdk/README.md) | [`phoenix-rise-ix`](../ix/README.md) |
[`phoenix-rise-math`](../math/README.md) |
[`phoenix-rise-cli`](../cli/README.md)
